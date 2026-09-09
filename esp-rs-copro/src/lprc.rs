use core::{
    alloc::Layout, cell::Cell, marker::PhantomData, mem::Alignment, num::NonZero, ops::Deref, ptr::{NonNull, copy_nonoverlapping}, sync::atomic::{AtomicUsize, Ordering},
};

#[cfg(feature = "nottest")]
use alloc::alloc;
#[cfg(not(feature = "nottest"))]
use std::alloc;

use crate::{EspCoproError, lpalloc::{self, address_translate_to_lp}, lpbox::{self, lpbox_alloc}, movableobject::MovableObject};

/// A minimal shared-owner pointer for data stored in the same memory domain as [`LPBox<T>`].
///
/// This is intentionally a small, conservative implementation. The current translation table in
/// [`crate::addresstranslation::AddressTranslationTable`] is designed around the single-owner
/// model used by [`crate::lpbox::LPBox`], so a fully integrated LP transfer implementation still
/// needs additional bookkeeping for shared ownership and address translation.
///
/// The goal of this type is to provide the same shape as `Rc<T>` while keeping the code safe to
/// compile as a partial prototype within the current library design.
pub struct LPRc<T: ?Sized + MovableObject> {
    ptr: NonNull<Inner<T>>,
    _marker: PhantomData<T>,
}

pub struct LPArc<T: ?Sized + MovableObject> {
    ptr: NonNull<AInner<T>>,
    _marker: PhantomData<T>,
}

#[repr(C, align(2))]
struct Inner<T: ?Sized + MovableObject> {
    strong: Cell<usize>,
    weak: Cell<usize>,
    value: T,
}

#[repr(C, align(2))]
struct AInner<T: ?Sized + MovableObject> {
    strong: AtomicUsize,
    weak: AtomicUsize,
    value: T,
}

impl<T: ?Sized + MovableObject> Inner<T> {
    fn get_strong(&self) -> usize {
        self.strong.get()
    }
    fn get_weak(&self) -> usize {
        self.weak.get()
    }
    fn try_increment_strong(&self) -> bool {
        if self.strong.get() == 0 {
            false
        } else {
            self.strong.set(self.strong.get() + 1);
            true
        }
    }
}

impl<T: ?Sized + MovableObject> AInner<T> {
    fn get_strong(&self) -> usize {
        self.strong.load(Ordering::Acquire)
    }
    fn get_weak(&self) -> usize {
        self.weak.load(Ordering::Acquire)
    }
    fn try_increment_strong(&self) -> bool {
        let mut strong = self.get_strong();
        loop {
            if strong == 0 {
                return false;
            }
            match self.strong.compare_exchange_weak(
                strong,
                strong + 1,
                Ordering::Acquire,
                Ordering::Acquire,
            ) {
                Ok(_) => return true,
                Err(actual) => strong = actual,
            }
        }
    }
}

/// A weak reference to `LPRc<T>`.
///
/// A weak reference does not keep the data alive. Once the last strong reference is dropped,
/// the weak reference will return `None` from `upgrade()`.
pub struct LPWeak<T: ?Sized + MovableObject> {
    ptr: NonNull<Inner<T>>,
    _marker: PhantomData<T>,
}

/// A weak reference to `LPArc<T>`.
///
/// A weak reference does not keep the data alive. Once the last strong reference is dropped,
/// the weak reference will return `None` from `upgrade()`.
pub struct LPAWeak<T: ?Sized + MovableObject> {
    ptr: NonNull<AInner<T>>,
    _marker: PhantomData<T>,
}

impl<T: ?Sized + MovableObject> LPWeak<T> {
    /// Returns a reference to the inner `Inner` struct.
    fn get_inner(&self) -> &Inner<T> {
        unsafe { lpalloc::try_address_translate_for_current_core_nonnull(self.ptr).as_ref() }
    }

    /// Attempts to upgrade the weak reference to a strong reference.
    ///
    /// Returns `None` if the strong count is already 0 (the data has been dropped).
    pub fn upgrade(&self) -> Option<LPRc<T>> {
        if self.get_inner().try_increment_strong() {
            Some(LPRc {
                ptr: self.ptr,
                _marker: PhantomData,
            })
        } else {
            None
        }
    }

    /// Returns the number of strong references.
    pub fn strong_count(&self) -> usize {
        self.get_inner().get_strong()
    }

    /// Returns the number of weak references.
    pub fn weak_count(&self) -> usize {
        self.get_inner().get_weak().saturating_sub(1)
    }

    fn from_inner(ptr: NonNull<Inner<T>>) -> Self {
        LPWeak {
            ptr,
            _marker: PhantomData,
        }
    }
}


impl<T: ?Sized + MovableObject> LPAWeak<T> {
    /// Returns a reference to the inner `Inner` struct.
    fn get_inner(&self) -> &AInner<T> {
        unsafe { lpalloc::try_address_translate_for_current_core_nonnull(self.ptr).as_ref() }
    }

    /// Attempts to upgrade the weak reference to a strong reference.
    ///
    /// Returns `None` if the strong count is already 0 (the data has been dropped).
    pub fn upgrade(&self) -> Option<LPArc<T>> {
        if self.get_inner().try_increment_strong() {
            Some(LPArc {
                ptr: self.ptr,
                _marker: PhantomData,
            })
        } else {
            None
        }
    }

    /// Returns the number of strong references.
    pub fn strong_count(&self) -> usize {
        self.get_inner().get_strong()
    }

    /// Returns the number of weak references.
    pub fn weak_count(&self) -> usize {
        self.get_inner().get_weak().saturating_sub(1)
    }

    fn from_inner(ptr: NonNull<AInner<T>>) -> Self {
        LPAWeak {
            ptr,
            _marker: PhantomData,
        }
    }
}

impl<T: ?Sized + MovableObject> Clone for LPWeak<T> {
    fn clone(&self) -> Self {
        let inner = self.get_inner();
        inner.weak.set(inner.weak.get() + 1);
        LPWeak {
            ptr: self.ptr,
            _marker: PhantomData,
        }
    }
}

impl<T: ?Sized + MovableObject> Clone for LPAWeak<T> {
    fn clone(&self) -> Self {
        self.get_inner().weak.fetch_add(1, Ordering::Relaxed);
        LPAWeak {
            ptr: self.ptr,
            _marker: PhantomData,
        }
    }
}

impl<T: ?Sized + MovableObject> Drop for LPWeak<T> {
    fn drop(&mut self) {
        unsafe {
            // Get the inner struct
            let inner = self.get_inner();
            let previous_weak = inner.get_weak();
            inner.weak.set(previous_weak.saturating_sub(1));
            if previous_weak == 1 && inner.get_strong() == 0 {
                // Last weak reference; check if strong is also 0
                lpbox::lp_dealloc(inner as *const Inner<T> as *mut u8, Layout::for_value_raw(inner));
            }
        }
    }
}

impl<T: ?Sized + MovableObject> Drop for LPAWeak<T> {
    fn drop(&mut self) {
        unsafe {
            // Get the inner struct
            let inner = self.get_inner();
            let previous_weak = inner.weak.fetch_sub(1, Ordering::Release);
            if previous_weak == 1 && inner.get_strong() == 0 {
                // Last weak reference; check if strong is also 0
                lpbox::lp_dealloc(inner as *const AInner<T> as *mut u8, Layout::for_value_raw(inner));
            }
        }
    }
}

impl<T: ?Sized + MovableObject> core::fmt::Debug for LPWeak<T>
where
    T: core::fmt::Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("LPWeak")
            .field("strong_count", &self.strong_count())
            .field("weak_count", &self.weak_count())
            .finish()
    }
}

impl<T: ?Sized + MovableObject> core::fmt::Debug for LPAWeak<T>
where
    T: core::fmt::Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("LPAWeak")
            .field("strong_count", &self.strong_count())
            .field("weak_count", &self.weak_count())
            .finish()
    }
}

impl<T: ?Sized + MovableObject> LPRc<T> {
    fn get_inner(&self) -> &Inner<T> {
        unsafe { lpalloc::try_address_translate_for_current_core_nonnull(self.ptr).as_ref() }
    }

    fn get_inner_mut(&mut self) -> &mut Inner<T> {
        unsafe { lpalloc::try_address_translate_for_current_core_nonnull(self.ptr).as_mut() }
    }

    pub fn strong_count(&self) -> usize {
        self.get_inner().get_strong()
    }

    pub fn weak_count(&self) -> usize {
        self.get_inner().get_weak().saturating_sub(1)
    }

    pub fn as_ptr(&self) -> *const T {
        &self.get_inner().value as *const T
    }
    
    pub fn ptr_eq(this: &Self, other: &Self) -> bool {
        core::ptr::eq(this.ptr.as_ptr(), other.ptr.as_ptr())
    }

    /// Create a weak reference to the value pointed to by this `LPRc`.
    pub fn downgrade(&self) -> LPWeak<T> {
        let previous_weak = self.get_inner().weak.get();
        self.get_inner().weak.set(previous_weak + 1);
        LPWeak {
            ptr: self.ptr,
            _marker: PhantomData,
        }
    }

    pub fn into_raw_without_translation(this: Self) -> * const T {
        let this = core::mem::ManuallyDrop::new(this);
        unsafe { core::ptr::addr_of!((*this.ptr.as_ptr()).value) }
    }

    unsafe fn from_inner(ptr: NonNull<Inner<T>>) -> Self {
        LPRc {
            ptr,
            _marker: PhantomData,
        }
    }

    unsafe fn from_ptr(ptr: *mut Inner<T>) -> Self {
        unsafe { Self::from_inner(NonNull::new_unchecked(ptr)) }
    }

    #[must_use = "losing the pointer will leak memory"]
    pub fn into_raw(this: Self) -> *const T {
        let this = core::mem::ManuallyDrop::new(this);
        this.as_ptr()
    }

    pub fn from_raw(ptr: *const T) -> Self {
        let layout = Layout::new::<Inner<()>>();
        let offset = layout.size() + layout.padding_needed_for(unsafe{Alignment::of_val_raw(ptr)});
        let rc_ptr = unsafe { ptr.byte_sub(offset) as *mut Inner<T> };
        unsafe { Self::from_ptr(rc_ptr) }
    }

}

impl<T: ?Sized + MovableObject> LPArc<T> {
    fn get_inner(&self) -> &AInner<T> {
        unsafe { lpalloc::try_address_translate_for_current_core_nonnull(self.ptr).as_ref() }
    }

    fn get_inner_mut(&mut self) -> &mut AInner<T> {
        unsafe { lpalloc::try_address_translate_for_current_core_nonnull(self.ptr).as_mut() }
    }

    pub fn strong_count(&self) -> usize {
        self.get_inner().get_strong()
    }

    pub fn weak_count(&self) -> usize {
        self.get_inner().get_weak().saturating_sub(1)
    }

    pub fn as_ptr(&self) -> *const T {
        &self.get_inner().value as *const T
    }
    
    pub fn ptr_eq(this: &Self, other: &Self) -> bool {
        core::ptr::eq(this.ptr.as_ptr(), other.ptr.as_ptr())
    }

    /// Create a weak reference to the value pointed to by this `LPRc`.
    pub fn downgrade(&self) -> LPAWeak<T> {
        self.get_inner().weak.fetch_add(1, Ordering::Relaxed);
        LPAWeak {
            ptr: self.ptr,
            _marker: PhantomData,
        }
    }

    pub fn into_raw_without_translation(this: Self) -> * const T {
        let this = core::mem::ManuallyDrop::new(this);
        unsafe { core::ptr::addr_of!((*this.ptr.as_ptr()).value) }
    }

    unsafe fn from_inner(ptr: NonNull<AInner<T>>) -> Self {
        LPArc {
            ptr,
            _marker: PhantomData,
        }
    }

    unsafe fn from_ptr(ptr: *mut AInner<T>) -> Self {
        unsafe { Self::from_inner(NonNull::new_unchecked(ptr)) }
    }

    #[must_use = "losing the pointer will leak memory"]
    pub fn into_raw(this: Self) -> *const T {
        let this = core::mem::ManuallyDrop::new(this);
        this.as_ptr()
    }

    pub fn from_raw(ptr: *const T) -> Self {
        let layout = Layout::new::<AInner<()>>();
        let offset = layout.size() + layout.padding_needed_for(unsafe{Alignment::of_val_raw(ptr)});
        let rc_ptr = unsafe { ptr.byte_sub(offset) as *mut AInner<T> };
        unsafe { Self::from_ptr(rc_ptr) }
    }

}

impl<T: MovableObject> LPRc<T> {
    /// Create a new [`LPRc`] containing the given value.
    /// The value is allocated on the main memory on the main processor, and is allocated on the LP memory on the LP coprocessor.
    /// The ownership is transferred to the caller.
    pub fn new(value: T) -> Self { unsafe {
        let ptr = lpbox_alloc(Layout::new::<Inner<T>>()) as *mut Inner<T>;
        ptr.write(Inner {
            strong: Cell::new(1),
            weak: Cell::new(1),
            value,
        });
        LPRc {
            ptr: NonNull::new_unchecked(ptr),
            _marker: PhantomData,
        }
    }}

    /// This is for testing. It creates a new [`LPRc`] containing the given value on the simulated LP heap.
    #[cfg(not(feature = "nottest"))]
    pub unsafe fn new_lp(value: T) -> Self { unsafe {
        let inner = Inner {
            strong: Cell::new(1),
            weak: Cell::new(1),
            value,
        };
        let ptr = lpalloc::lp_allocator_alloc(Layout::for_value(&inner)) as * mut Inner<T>;
        ptr.write_volatile(inner);
        LPRc {
            ptr: NonNull::new_unchecked(address_translate_to_lp(ptr)),
            _marker: PhantomData,
        }
    }}

    pub fn try_unwrap(this: Self) -> Result<T, Self> {
        if this.strong_count() == 1 {
            let this = core::mem::ManuallyDrop::new(this);
            let val: T = unsafe { core::ptr::read(&**this) }; // copy the contained object

            let inner = this.get_inner();
            inner.strong.set(inner.strong.get().saturating_sub(1));
            let _weak = LPWeak { ptr: this.ptr, _marker: PhantomData }; // fake for drop
            Ok(val)
        } else {
            Err(this)
        }
    }

    pub fn into_inner(this: Self) -> Option<T> {
        Self::try_unwrap(this).ok()
    }
}

impl<T: MovableObject> LPArc<T> {
    /// Create a new [`LPRc`] containing the given value.
    /// The value is allocated on the main memory on the main processor, and is allocated on the LP memory on the LP coprocessor.
    /// The ownership is transferred to the caller.
    pub fn new(value: T) -> Self { unsafe {
        let ptr = lpbox_alloc(Layout::new::<AInner<T>>()) as *mut AInner<T>;
        ptr.write(AInner {
            strong: AtomicUsize::new(1),
            weak: AtomicUsize::new(1),
            value,
        });
        LPArc {
            ptr: NonNull::new_unchecked(ptr),
            _marker: PhantomData,
        }
    }}

    /// This is for testing. It creates a new [`LPRc`] containing the given value on the simulated LP heap.
    #[cfg(not(feature = "nottest"))]
    pub unsafe fn new_lp(value: T) -> Self { unsafe {
        let inner = AInner {
            strong: AtomicUsize::new(1),
            weak: AtomicUsize::new(1),
            value,
        };
        let ptr = lpalloc::lp_allocator_alloc(Layout::for_value(&inner)) as * mut AInner<T>;
        ptr.write_volatile(inner);
        LPArc {
            ptr: NonNull::new_unchecked(address_translate_to_lp(ptr)),
            _marker: PhantomData,
        }
    }}

    pub fn try_unwrap(this: Self) -> Result<T, Self> {
        if this.strong_count() == 1 {
            let this = core::mem::ManuallyDrop::new(this);
            let val: T = unsafe { core::ptr::read(&**this) }; // copy the contained object

            this.get_inner().strong.fetch_sub(1, Ordering::Acquire);
            let _weak = LPAWeak { ptr: this.ptr, _marker: PhantomData }; // fake for drop
            Ok(val)
        } else {
            Err(this)
        }
    }

    pub fn into_inner(this: Self) -> Option<T> {
        Self::try_unwrap(this).ok()
    }
}

impl<T: ?Sized + MovableObject> Clone for LPRc<T> {
    fn clone(&self) -> Self {
        let inner = self.get_inner();
        inner.strong.set(inner.strong.get().saturating_add(1));

        Self {
            ptr: self.ptr,
            _marker: PhantomData,
        }
    }
}

impl<T: ?Sized + MovableObject> Clone for LPArc<T> {
    fn clone(&self) -> Self {
        self.get_inner().strong.fetch_add(1, Ordering::Relaxed);

        Self {
            ptr: self.ptr,
            _marker: PhantomData,
        }
    }
}

impl<T: ?Sized + MovableObject> Deref for LPRc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.get_inner().value
    }
}

impl<T: ?Sized + MovableObject> Deref for LPArc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.get_inner().value
    }
}

impl<T: ?Sized + MovableObject> Drop for LPRc<T> {
    fn drop(&mut self) {
        unsafe {
            let inner = self.get_inner_mut();
            let previous = inner.strong.get();
            inner.strong.set(previous.saturating_sub(1));
            if previous == 1 {
                // Drop the value, but leave the Inner allocation to LPWeak cleanup
                core::ptr::drop_in_place(&inner.value as *const T as *mut T);
                
                // Decrement weak count (account for the strong reference's implicit weak ref)
                let previous_weak = inner.weak.get();
                inner.weak.set(previous_weak.saturating_sub(1));
                if previous_weak == 1 {
                    // No more weak references, so deallocate Inner
                    let layout = Layout::for_value(inner);
                    lpbox::lp_dealloc(inner as *const Inner<T> as *mut u8, layout);
                }
            }
        }
    }
}

impl<T: ?Sized + MovableObject> Drop for LPArc<T> {
    fn drop(&mut self) {
        unsafe {
            let inner = self.get_inner_mut();
            let previous = inner.strong.fetch_sub(1, Ordering::Release);
            if previous == 1 {
                // Drop the value, but leave the Inner allocation to LPWeak cleanup
                core::ptr::drop_in_place(&inner.value as *const T as *mut T);
                
                // Decrement weak count (account for the strong reference's implicit weak ref)
                let previous_weak = inner.weak.fetch_sub(1, Ordering::Acquire);
                if previous_weak == 1 {
                    // No more weak references, so deallocate Inner
                    let layout = Layout::for_value(inner);
                    lpbox::lp_dealloc(inner as *const AInner<T> as *mut u8, layout);
                }
            }
        }
    }
}


impl<T: MovableObject> AsRef<T> for LPRc<T> {
    fn as_ref(&self) -> &T {
        &**self
    }
}

impl<T: MovableObject> AsRef<T> for LPArc<T> {
    fn as_ref(&self) -> &T {
        &**self
    }
}

impl<T: ?Sized + MovableObject> PartialEq for LPRc<T> {
    fn eq(&self, other: &Self) -> bool {
        LPRc::ptr_eq(self, other)
    }
}

impl<T: ?Sized + MovableObject> PartialEq for LPArc<T> {
    fn eq(&self, other: &Self) -> bool {
        LPArc::ptr_eq(self, other)
    }
}

impl<T: ?Sized + MovableObject> Eq for LPRc<T> {}
impl<T: ?Sized + MovableObject> Eq for LPArc<T> {}

impl<T: ?Sized + MovableObject + core::fmt::Debug> core::fmt::Debug for LPRc<T>
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("LPRc")
            .field("strong_count", &self.strong_count())
            .field("value", &self.deref())
            .finish()
    }
}

impl<T: ?Sized + MovableObject + core::fmt::Debug> core::fmt::Debug for LPArc<T>
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("LPArc")
            .field("strong_count", &self.strong_count())
            .field("value", &self.deref())
            .finish()
    }
}

impl<T: ?Sized + MovableObject> MovableObject for Inner<T> {
    #[cfg(not(feature = "is-lp-core"))]
    unsafe fn move_to_main(&self, dest : *mut u8) -> Result<(), crate::EspCoproError> {
        unsafe {
            let dest = core::ptr::from_raw_parts_mut::<Self>(dest, core::ptr::metadata(self as *const Self));
            copy_nonoverlapping(self as *const Self as * const Inner<()>, dest as * mut Inner<()>, 1);
            self.value.move_to_main(&mut (*dest).value as *mut T as *mut u8)
        }
    }
    #[cfg(feature = "is-lp-core")]
    unsafe fn move_to_main(&self, _dest: *mut u8) -> Result<(), crate::EspCoproError> {
        Err(EspCoproError::NotAllowed)
    }

    #[cfg(not(feature = "is-lp-core"))]
    unsafe fn move_to_lp(&self, dest : *mut u8) -> Result<(), crate::EspCoproError> {
        unsafe {
            let dest = core::ptr::from_raw_parts_mut::<Self>(dest, core::ptr::metadata(self as *const Self));
            copy_nonoverlapping(self as *const Self as * const Inner<()>, dest as * mut Inner<()>, 1);
            self.value.move_to_lp(&mut (*dest).value as *mut T as *mut u8)
        }
    }
    #[cfg(feature = "is-lp-core")]
    unsafe fn move_to_lp(&self, _dest: *mut u8) -> Result<(), crate::EspCoproError> {
        Err(EspCoproError::NotAllowed)
    }
}

impl<T: ?Sized + MovableObject> MovableObject for AInner<T> {
    #[cfg(not(feature = "is-lp-core"))]
    unsafe fn move_to_main(&self, dest : *mut u8) -> Result<(), crate::EspCoproError> {
        unsafe {
            let dest = core::ptr::from_raw_parts_mut::<Self>(dest, core::ptr::metadata(self as *const Self));
            copy_nonoverlapping(self as *const Self as * const AInner<()>, dest as * mut AInner<()>, 1);
            self.value.move_to_main(&mut (*dest).value as *mut T as *mut u8)
        }
    }
    #[cfg(feature = "is-lp-core")]
    unsafe fn move_to_main(&self, _dest: *mut u8) -> Result<(), crate::EspCoproError> {
        Err(EspCoproError::NotAllowed)
    }

    #[cfg(not(feature = "is-lp-core"))]
    unsafe fn move_to_lp(&self, dest : *mut u8) -> Result<(), crate::EspCoproError> {
        unsafe {
            let dest = core::ptr::from_raw_parts_mut::<Self>(dest, core::ptr::metadata(self as *const Self));
            copy_nonoverlapping(self as *const Self as * const AInner<()>, dest as * mut AInner<()>, 1);
            self.value.move_to_lp(&mut (*dest).value as *mut T as *mut u8)
        }
    }
    #[cfg(feature = "is-lp-core")]
    unsafe fn move_to_lp(&self, _dest: *mut u8) -> Result<(), crate::EspCoproError> {
        Err(EspCoproError::NotAllowed)
    }
}

#[cfg(not(feature = "is-lp-core"))]
fn impl_move_to_main(src: *const u8, layout: Layout) -> Result<(bool, NonNull<u8>), crate::EspCoproError> {
    let mut must_move = true;
    let inner = lpbox::lpbox_static::get_by_lp(src as usize, true).and_then(|(ata, copied)|  {
        if layout == ata.get_layout() {
            must_move = !copied;
            Some(unsafe{ NonNull::new_unchecked(ata.get_addr() as *mut u8) })
        } else {
            // If the layout doesn't match, we need to allocate a new one
            unsafe { alloc::dealloc(ata.get_addr() as * mut u8, ata.get_layout()) };
            None
        }
    });
    let inner = if let Some(inner) = inner {
        inner
    } else {
        // If not found, allocate a new one in main memory
        let addr = lpbox::lpbox_alloc(layout);
        if addr.is_null() { return Err(EspCoproError::OutOfMemory); }
        lpbox::lpbox_static::insert_no_drop_copied(addr, src  as usize);
        unsafe { NonNull::new_unchecked(addr) }
    };
    Ok((must_move, inner))
}


#[cfg(not(feature = "is-lp-core"))]
fn impl_move_to_main_inner<T: ?Sized + MovableObject>(src: &Inner<T>) -> Result<NonNull<Inner<T>>, crate::EspCoproError> {
    let inner_layout = Layout::for_value(src);
    match impl_move_to_main(src as * const Inner<T> as *const u8, inner_layout) {
        Ok((must_move, res)) => {
            unsafe {
                if must_move {
                    src.move_to_main(res.as_ptr() as * mut u8)?;
                }
            }
            Ok(NonNull::from(src).with_addr(res.addr()))
        },
        Err(e) => Err(e)
    }
}

#[cfg(not(feature = "is-lp-core"))]
fn impl_move_to_main_ainner<T: ?Sized + MovableObject>(src: &AInner<T>) -> Result<NonNull<AInner<T>>, crate::EspCoproError> {
    let inner_layout = Layout::for_value(src);
    match impl_move_to_main(src as * const AInner<T> as *const u8, inner_layout) {
        Ok((must_move, res)) => {
            unsafe {
                if must_move {
                    src.move_to_main(res.as_ptr() as * mut u8)?;
                }
            }
            Ok(NonNull::from(src).with_addr(res.addr()))
        },
        Err(e) => Err(e)
    }
}

#[cfg(not(feature = "is-lp-core"))]
fn impl_move_to_lp_inner<T: ?Sized + MovableObject>(src: &Inner<T>) -> Result<NonNull<Inner<T>>, crate::EspCoproError> {
    let inner = if let Some(addr) = lpbox::lpbox_static::get_by_main(src as *const Inner<T> as *const () as usize) {
        NonNull::from_ref(src).with_addr(unsafe { NonZero::new_unchecked(addr) })
    } else {
        unsafe {
            // If not found, allocate a new one in LP memory
            let ptr: *mut u8 = lpalloc::lp_allocator_alloc(core::alloc::Layout::for_value(src)) as * mut u8;
            if ptr.is_null() { return Err(EspCoproError::OutOfMemory); }
            src.move_to_lp(ptr)?;
            let ptr = ptr as usize;
            lpbox::lpbox_static::insert_no_drop(src as *const Inner<T> as *mut Inner<T>, ptr);
            NonNull::from_ref(src).with_addr(NonZero::new_unchecked(ptr))
        }
    };
    Ok(inner)
}

#[cfg(not(feature = "is-lp-core"))]
fn impl_move_to_lp_ainner<T: ?Sized + MovableObject>(src: &AInner<T>) -> Result<NonNull<AInner<T>>, crate::EspCoproError> {
    let inner = if let Some(addr) = lpbox::lpbox_static::get_by_main(src as *const AInner<T> as *const () as usize) {
        NonNull::from_ref(src).with_addr(unsafe { NonZero::new_unchecked(addr) })
    } else {
        unsafe {
            // If not found, allocate a new one in LP memory
            let ptr: *mut u8 = lpalloc::lp_allocator_alloc(core::alloc::Layout::for_value(src)) as * mut u8;
            if ptr.is_null() { return Err(EspCoproError::OutOfMemory); }
            src.move_to_lp(ptr)?;
            let ptr = ptr as usize;
            lpbox::lpbox_static::insert_no_drop(src as *const AInner<T> as *mut AInner<T>, ptr);
            NonNull::from_ref(src).with_addr(NonZero::new_unchecked(ptr))
        }
    };
    Ok(inner)
}

impl<T: ?Sized + MovableObject> MovableObject for LPWeak<T> {
    #[cfg(not(feature = "is-lp-core"))]
    unsafe fn move_to_main(&self, dest : *mut u8) -> Result<(), crate::EspCoproError> {
        unsafe {
            (dest as *mut Self).write_volatile(Self::from_inner(impl_move_to_main_inner(self.get_inner())?))
        }
        Ok(())
    }
    #[cfg(feature = "is-lp-core")]
    unsafe fn move_to_main(&self, _dest: *mut u8) -> Result<(), crate::EspCoproError> {
        Err(EspCoproError::NotAllowed)
    }

    #[cfg(not(feature = "is-lp-core"))]
    unsafe fn move_to_lp(&self, dest : *mut u8) -> Result<(), crate::EspCoproError> {
        unsafe {
            (dest as *mut Self).write_volatile(Self::from_inner(impl_move_to_lp_inner(self.get_inner())?))
        }
        Ok(())
    }
    #[cfg(feature = "is-lp-core")]
    unsafe fn move_to_lp(&self, _dest: *mut u8) -> Result<(), crate::EspCoproError> {
        Err(EspCoproError::NotAllowed)
    }
}

impl<T: ?Sized + MovableObject> MovableObject for LPAWeak<T> {
    #[cfg(not(feature = "is-lp-core"))]
    unsafe fn move_to_main(&self, dest : *mut u8) -> Result<(), crate::EspCoproError> {
        unsafe {
            (dest as *mut Self).write_volatile(Self::from_inner(impl_move_to_main_ainner(self.get_inner())?))
        }
        Ok(())
    }
    #[cfg(feature = "is-lp-core")]
    unsafe fn move_to_main(&self, _dest: *mut u8) -> Result<(), crate::EspCoproError> {
        Err(EspCoproError::NotAllowed)
    }

    #[cfg(not(feature = "is-lp-core"))]
    unsafe fn move_to_lp(&self, dest : *mut u8) -> Result<(), crate::EspCoproError> {
        unsafe {
            (dest as *mut Self).write_volatile(Self::from_inner(impl_move_to_lp_ainner(self.get_inner())?))
        }
        Ok(())
    }
    #[cfg(feature = "is-lp-core")]
    unsafe fn move_to_lp(&self, _dest: *mut u8) -> Result<(), crate::EspCoproError> {
        Err(EspCoproError::NotAllowed)
    }
}

impl<T: ?Sized + MovableObject> MovableObject for LPRc<T> {
    #[cfg(not(feature = "is-lp-core"))]
    unsafe fn move_to_main(&self, dest : *mut u8) -> Result<(), crate::EspCoproError> {
        unsafe {
            (dest as *mut Self).write_volatile(Self::from_inner(impl_move_to_main_inner(self.get_inner())?))
        }
        Ok(())
    }
    #[cfg(feature = "is-lp-core")]
    unsafe fn move_to_main(&self, _dest: *mut u8) -> Result<(), crate::EspCoproError> {
        Err(EspCoproError::NotAllowed)
    }

    #[cfg(not(feature = "is-lp-core"))]
    unsafe fn move_to_lp(&self, dest : *mut u8) -> Result<(), crate::EspCoproError> {
        unsafe {
            (dest as *mut Self).write_volatile(Self::from_inner(impl_move_to_lp_inner(self.get_inner())?))
        }
        Ok(())
    }
    #[cfg(feature = "is-lp-core")]
    unsafe fn move_to_lp(&self, _dest: *mut u8) -> Result<(), crate::EspCoproError> {
        Err(EspCoproError::NotAllowed)
    }
}


impl<T: ?Sized + MovableObject> MovableObject for LPArc<T> {
    #[cfg(not(feature = "is-lp-core"))]
    unsafe fn move_to_main(&self, dest : *mut u8) -> Result<(), crate::EspCoproError> {
        unsafe {
            (dest as *mut Self).write_volatile(Self::from_inner(impl_move_to_main_ainner(self.get_inner())?))
        }
        Ok(())
    }
    #[cfg(feature = "is-lp-core")]
    unsafe fn move_to_main(&self, _dest: *mut u8) -> Result<(), crate::EspCoproError> {
        Err(EspCoproError::NotAllowed)
    }

    #[cfg(not(feature = "is-lp-core"))]
    unsafe fn move_to_lp(&self, dest : *mut u8) -> Result<(), crate::EspCoproError> {
        unsafe {
            (dest as *mut Self).write_volatile(Self::from_inner(impl_move_to_lp_ainner(self.get_inner())?))
        }
        Ok(())
    }
    #[cfg(feature = "is-lp-core")]
    unsafe fn move_to_lp(&self, _dest: *mut u8) -> Result<(), crate::EspCoproError> {
        Err(EspCoproError::NotAllowed)
    }
}

