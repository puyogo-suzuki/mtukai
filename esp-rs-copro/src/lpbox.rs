use core::{fmt::Debug, mem::{self, MaybeUninit}, ops::{Deref, DerefMut}, ptr::NonNull};

use crate::movableobject::MovableObject;
use crate::lpalloc;
#[cfg(feature = "nottest")]
use alloc::alloc;
#[cfg(feature = "nottest")]
use ::alloc::boxed::Box;
#[cfg(feature = "nottest")]
use core::cmp::Ordering;
#[cfg(not(feature = "nottest"))]
use std::{alloc, boxed::Box, cmp::Ordering};

pub struct LPBox<T: ?Sized + MovableObject>(pub(crate) NonNull<T>);

impl<T: MovableObject> Debug for LPBox<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "LPBox({:p})", self.0.as_ptr())
    }
}

fn get_vtable(obj: &dyn MovableObject) -> *const u8 {
    let fat_ptr_addr = obj as *const dyn MovableObject as *const [usize; 2];
    unsafe{
        let vtable_ptr_addr = fat_ptr_addr.add(1);
        mem::transmute_copy(&vtable_ptr_addr)
    }
}

pub(crate) fn lpbox_alloc(l : core::alloc::Layout) -> *mut u8 {
    unsafe {
        let ptr = alloc::alloc(l);
        if ptr.is_null() { alloc::handle_alloc_error(l); }
        ptr
    }
}

pub(crate) fn lpbox_realloc(ptr : * mut u8, old_layout : core::alloc::Layout, new_size : usize) -> * mut u8 {
    unsafe {
        let new_ptr = alloc::realloc(ptr, old_layout, new_size);
        if new_ptr.is_null() { alloc::handle_alloc_error(old_layout); }
        new_ptr
    }
}

#[cfg(any(feature = "has-lp-core", not(feature = "nottest")))]
pub(crate) fn lp_dealloc(ptr: * mut u8, layout: core::alloc::Layout) {
    unsafe {
        if lpalloc::in_lp_mem_range(ptr) {
            lpalloc::lp_allocator_dealloc(ptr, layout); // lp coprocessor
        } else {
            alloc::dealloc(ptr, layout); // main processor
        }
    }
}

#[cfg(feature = "has-lp-core")]
pub(crate) mod lpbox_static {
    use crate::addresstranslation::AddressTranslationTable;
    use core::{alloc::Layout, cell::{Cell, Ref, RefCell, RefMut}};

    pub(crate) static ADDRESS_TRANSLATION_TABLE : SyncImplementedRefCell<AddressTranslationTable> =
        SyncImplementedRefCell::new(AddressTranslationTable::new());
    static LPBOX_DROP_ENABLE : SyncImplementedBool = SyncImplementedBool::new(true);

    pub(crate) struct SyncImplementedRefCell<T>(RefCell<T>);
    impl<T> SyncImplementedRefCell<T> {
        pub const fn new(value: T) -> Self {
            SyncImplementedRefCell(RefCell::new(value))
        }
        pub fn borrow_mut(&self) -> RefMut<'_, T> {
            self.0.borrow_mut()
        }
        pub fn borrow(&self) -> Ref<'_, T> {
            self.0.borrow()
        }
    }
    unsafe impl<T> Sync for SyncImplementedRefCell<T> {}

    struct SyncImplementedBool(Cell<bool>);
    impl SyncImplementedBool {
        pub const fn new(value: bool) -> Self {
            SyncImplementedBool(Cell::new(value))
        }
        pub fn set(&self, value: bool) {
            self.0.set(value);
        }
        pub fn get(&self) -> bool {
            self.0.get()
        }
    }
    unsafe impl Sync for SyncImplementedBool {}
    pub fn cleanup() {
        LPBOX_DROP_ENABLE.set(false);
        ADDRESS_TRANSLATION_TABLE.borrow_mut().drop_and_clear();
        LPBOX_DROP_ENABLE.set(true);
    }
    pub fn check_lpbox_drop_enable() -> bool {
        LPBOX_DROP_ENABLE.get()
    }
    pub fn remove_by_main(main: usize) -> Option<usize> {
        ADDRESS_TRANSLATION_TABLE.borrow_mut().remove_by_main(main)
    }
    pub fn remove_by_lp(lp: usize) -> Option<(usize, Layout)> {
        ADDRESS_TRANSLATION_TABLE.borrow_mut().remove_by_lp(lp)
    }
    pub(crate) fn get_by_main(main: usize) -> Option<usize> {
        ADDRESS_TRANSLATION_TABLE.borrow().get_by_main(main)
    }
    pub(crate) fn insert_no_drop<T : ?Sized>(main: *mut T, lp: usize) {
        ADDRESS_TRANSLATION_TABLE.borrow_mut().insert_no_drop(main, lp);
    }
}

#[cfg(not(feature = "nottest"))]
pub(crate) mod lpbox_static {
    use crate::addresstranslation::AddressTranslationTable;
    use core::cell::{RefCell, Cell};
    use std::alloc::Layout;

    thread_local! {
        pub(crate) static ADDRESS_TRANSLATION_TABLE : RefCell<AddressTranslationTable> =
            RefCell::new(AddressTranslationTable::new());
        static LPBOX_DROP_ENABLE : Cell<bool> = Cell::new(true);
    }
    
    pub fn cleanup() {
        LPBOX_DROP_ENABLE.set(false);
        ADDRESS_TRANSLATION_TABLE.with_borrow_mut(|tbl| tbl.drop_and_clear());
        LPBOX_DROP_ENABLE.set(true);
    }
    pub fn check_lpbox_drop_enable() -> bool {
        LPBOX_DROP_ENABLE.get()
    }
    pub fn remove_by_main(main: usize) -> Option<usize> {
        ADDRESS_TRANSLATION_TABLE.with_borrow_mut(|tbl| tbl.remove_by_main(main))
    }
    pub fn remove_by_lp(lp: usize) -> Option<(usize, Layout)> {
        ADDRESS_TRANSLATION_TABLE.with_borrow_mut(|tbl| tbl.remove_by_lp(lp))
    }
    pub fn get_by_main(main: usize) -> Option<usize> {
        ADDRESS_TRANSLATION_TABLE.with_borrow(|tbl| tbl.get_by_main(main))
    }
    pub(crate) fn insert_no_drop<T : ?Sized>(main: *mut T, lp: usize) {
        ADDRESS_TRANSLATION_TABLE.with_borrow_mut(|tbl| tbl.insert_no_drop(main, lp));
    }
}

#[cfg(any(feature = "has-lp-core", not(feature = "nottest")))]
pub fn cleanup() {
    lpbox_static::cleanup();
}
#[cfg(any(feature = "has-lp-core", not(feature = "nottest")))]
pub(crate) fn remove_by_main(main: usize) -> Option<usize> {
    lpbox_static::remove_by_main(main)
}

pub fn new_array_uninitialized<T : MovableObject>(n : isize) ->  LPBox<[T]> {
    unsafe {
        let b : Box<[MaybeUninit<T>]> = Box::new_uninit_slice(n as usize);
        LPBox(NonNull::new_unchecked(Box::into_raw(b) as * mut [T]))
    }
} 

impl<T: MovableObject, const N : usize> LPBox<[T;N]> {
    pub fn into_dynamic_slice(self) -> LPBox<[T]> {
        let original = mem::ManuallyDrop::new(self);
        unsafe {
            let ptr = original.0.as_ptr() as * mut T;
            let slice_ptr = core::ptr::slice_from_raw_parts_mut(ptr, N);
            LPBox(NonNull::new_unchecked(slice_ptr))
        }
    }
}

impl<T: MovableObject> LPBox<T> {
    #[cfg(not(feature = "is-lp-core"))]
    pub fn new(value: T) -> Self { unsafe {
        let ptr = lpbox_alloc(core::alloc::Layout::new::<T>()) as *mut T;
        ptr.write(value);
        LPBox(NonNull::new_unchecked(ptr))
    }}

    #[cfg(feature = "is-lp-core")]
    pub fn new(value: T) -> Self { unsafe {
        let ptr = lpbox_alloc(core::alloc::Layout::new::<T>()) as *mut T;
        ptr.write(value);
        lpalloc::write_vtable(ptr as * mut u8, get_vtable(ptr.as_ref().unwrap()) as * mut u8);
        LPBox(NonNull::new_unchecked(ptr))
    }}
}

impl<T: ?Sized + MovableObject> LPBox<T> {
    pub fn from_box(value : Box<T>) -> Self{
        unsafe { LPBox(NonNull::new_unchecked(Box::into_raw(value))) }
    }
    pub fn from_raw(raw : * mut T) -> Self {
        unsafe { LPBox(NonNull::new_unchecked(raw)) }
    }

    pub fn into_raw(self) -> * mut T {
        let b = mem::ManuallyDrop::new(self);
        b.0.as_ptr()
    }

    pub fn as_ptr(&self) -> * const T {
        self.0.as_ptr() as * const T
    }
    pub fn as_mut_ptr(&mut self) -> * mut T {
        self.0.as_ptr()
    }

    #[cfg(any(feature = "has-lp-core", not(feature = "nottest")))]
    pub fn write_to_lp(value : &T) -> * mut u8 { unsafe {
        if let Some(existing_lp_addr) = 
            lpbox_static::get_by_main(value as *const T as * const () as usize) {
            return existing_lp_addr as * mut u8;
        }
        let ptr = lpalloc::lp_allocator_alloc(core::alloc::Layout::for_value(value)) as * mut u8;
        value.move_to_lp(ptr);
        // ptr.copy_from(value, layout.size());
        // TODO: write_vtable
        // lpalloc::write_vtable(ptr as * mut u8, get_vtable(value) as * mut u8);
        lpbox_static::insert_no_drop(value as *const T as *mut T, ptr as usize);
        ptr
    }}

    #[cfg(any(feature = "has-lp-core", not(feature = "nottest")))]
    pub fn write_to_main(value : &T) -> * mut u8 { unsafe {
        let addr =
            lpbox_static::remove_by_lp(value as * const T as * const () as usize)
                .map_or_else(|| lpbox_alloc(core::alloc::Layout::for_value(value)) as usize,
                    |a| a.0);
        value.move_to_main(addr as * mut u8);
        addr as * mut u8
    }}

    #[cfg(any(feature = "has-lp-core", not(feature = "nottest")))]
    pub fn get_moved_to_lp(&self) -> LPBox<T>{
        unsafe { LPBox(self.0.with_addr(core::num::NonZero::new_unchecked(Self::write_to_lp(self.0.as_ref()) as usize)))}
    }
    #[cfg(any(feature = "has-lp-core", not(feature = "nottest")))]
    pub fn get_moved_to_main(&self) -> LPBox<T> {
        unsafe { LPBox(self.0.with_addr(core::num::NonZero::new_unchecked(Self::write_to_main(self.0.as_ref()) as usize))) }
    }
    // call the main processor's function.
    #[cfg(feature = "is-lp-core")]
    pub fn get_moved_to_lp(&self) -> LPBox<T>{ todo!(); }
    #[cfg(feature = "is-lp-core")]
    pub fn get_moved_to_main(&self) -> LPBox<T> { todo!(); }
}


impl<T: ?Sized + MovableObject> MovableObject for LPBox<T> {
    unsafe fn move_to_main(&self, dest: *mut u8) {
        unsafe { (dest as *mut LPBox<T>).write_volatile(self.get_moved_to_main()); }
    }

    unsafe fn move_to_lp(&self, dest: *mut u8) {
        unsafe { (dest as *mut LPBox<T>).write_volatile(self.get_moved_to_lp()); }
    }
}

impl<T : ?Sized + MovableObject> Deref for LPBox<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { self.0.as_ref() }
    }
}
impl<T : ?Sized + MovableObject> DerefMut for LPBox<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.0.as_mut() }
    }
}

impl<T : ?Sized + MovableObject> Drop for LPBox<T>{
    #[cfg(any(feature = "has-lp-core", not(feature = "nottest")))]
    fn drop(&mut self) {
        if !lpbox_static::check_lpbox_drop_enable() {
            return;
        }
        unsafe{self.0.drop_in_place();}
        let ptr = self.0.as_ptr() as * mut u8;
        let lay = unsafe {core::alloc::Layout::for_value(self.0.as_ref())};
        lp_dealloc(ptr, lay);
    }
    #[cfg(feature = "is-lp-core")]
    fn drop(&mut self) {
        let ptr = self.0.as_ptr() as * mut u8;
        let lay = unsafe {core::alloc::Layout::for_value(self.0.as_ref())};
        if lpalloc::in_lp_mem_range(ptr) {
            unsafe{self.0.drop_in_place();}
            unsafe{alloc::dealloc(ptr, lay);} // lp processor
        } else {
            // do not drop, as it is on the main coprocessor
        }
    }
}

impl<T : ?Sized + MovableObject + PartialEq> PartialEq for LPBox<T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        PartialEq::eq(&**self, &**other)
    }
    #[inline]
    fn ne(&self, other: &Self) -> bool {
        PartialEq::ne(&**self, &**other)
    }
}
impl<T : ?Sized + MovableObject + Eq> Eq for LPBox<T> {}

impl<T: ?Sized + MovableObject + PartialOrd> PartialOrd for LPBox<T> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        PartialOrd::partial_cmp(&**self, &**other)
    }
    #[inline]
    fn lt(&self, other: &Self) -> bool {
        PartialOrd::lt(&**self, &**other)
    }
    #[inline]
    fn le(&self, other: &Self) -> bool {
        PartialOrd::le(&**self, &**other)
    }
    #[inline]
    fn ge(&self, other: &Self) -> bool {
        PartialOrd::ge(&**self, &**other)
    }
    #[inline]
    fn gt(&self, other: &Self) -> bool {
        PartialOrd::gt(&**self, &**other)
    }
}
impl <T : ?Sized + MovableObject + Ord> Ord for LPBox<T> {
    #[inline]
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        Ord::cmp(&**self, &**other)
    }
}
