#[cfg(feature = "has-lp-core")]
use esp_sync::{raw::{SingleCoreInterruptLock, RawLock}, RestoreState};
use crate::{movableobject::MovableObject, EspCoproError};

use core::{cell::UnsafeCell, marker::PhantomData};
use core::ptr::copy_nonoverlapping;

// Some parts of code are imported from https://github.com/esp-rs/esp-hal/blob/0fed45c84c0723ed4e0b7d19c7f3078ed79090c6/esp-sync/src/lib.rs

/// Opaque token that can be used to release a lock.
// The interpretation of this value depends on the lock type that created it,
// but bit #31 is reserved for the reentry flag.
//
// Xtensa: PS has 15 useful bits. Bits 12..16 and 19..32 are unused, so we can
// use bit #31 as our reentry flag.
// We can assume the reserved bit is 0 otherwise rsil - wsr pairings would be
// undefined behavior: Quoting the ISA summary, table 64:
// Writing a non-zero value to these fields results in undefined processor
// behavior.
//
// Risc-V: we either get the restore state from bit 3 of mstatus, or
// we create the restore state from the current Priority, which is at most 31.
#[cfg(feature = "is-lp-core")]
#[derive(Clone, Copy, Debug)]
pub struct RestoreState(u32, PhantomData<*const ()>);

#[cfg(feature = "is-lp-core")]
impl RestoreState {
    const REENTRY_FLAG: u32 = 1 << 31;

    /// Creates a new RestoreState from a raw inner state.
    ///
    /// # Safety
    ///
    /// The `inner` value must be appropriate for the [RawMutex] implementation that creates it.
    pub const unsafe fn new(inner: u32) -> Self {
        Self(inner, PhantomData)
    }

    /// Returns an invalid RestoreState.
    ///
    /// Note that due to the safety contract of [`RawLock::enter`]/[`RawLock::exit`], you must not
    /// pass a `RestoreState` obtained from this method to [`RawLock::exit`].
    pub const fn invalid() -> Self {
        Self(0, PhantomData)
    }

    #[inline]
    fn mark_reentry(&mut self) {
        self.0 |= Self::REENTRY_FLAG;
    }

    #[inline]
    fn is_reentry(self) -> bool {
        self.0 & Self::REENTRY_FLAG != 0
    }

    /// Returns the raw value used to create this RestoreState.
    #[inline]
    pub fn inner(self) -> u32 {
        self.0
    }
}

/// A lock that disables interrupts.
#[cfg(feature = "is-lp-core")]
pub struct SingleCoreInterruptLock;

#[cfg(feature = "is-lp-core")]
pub trait RawLock {
    /// Acquires the raw lock
    ///
    /// # Safety
    ///
    /// The returned tokens must be released in reverse order, on the same thread that they were
    /// created on.
    unsafe fn enter(&self) -> RestoreState;

    /// Releases the raw lock
    ///
    /// # Safety
    ///
    /// - The `token` must be created by `self.enter()`
    /// - Tokens must be released in reverse order to their creation, on the same thread that they
    ///   were created on.
    unsafe fn exit(&self, token: RestoreState);
}

/// Trait for single-core locks.
#[cfg(feature = "is-lp-core")]
impl RawLock for SingleCoreInterruptLock {
    unsafe fn enter(&self) -> RestoreState {
        unsafe{ RestoreState::new(0) }
    }

    unsafe fn exit(&self, token: RestoreState) {
    }
}

use core::sync::atomic::{AtomicUsize, Ordering};

// Safety: Ensure that when adding new chips `raw_core` doesn't return this
// value.
const UNUSED_THREAD_ID_VALUE: usize = 0x100;

#[inline]
fn thread_id() -> usize {
    fn inner_val() -> usize {
        #[cfg(feature = "esp32c6")]
        return riscv::register::mhartid::read();

        #[cfg(feature = "esp32s3")]
        return (xtensa_lx::get_processor_id() & 0x2000) as usize;
    }
    if cfg!(feature = "has-lp-core") {
        inner_val()
    } else {
        inner_val() | (1 << 15) // it is LP coprocessor.
    }
}

#[repr(transparent)]
pub(super) struct LPLockedState {
    owner: AtomicUsize,
}

impl LPLockedState {
    #[inline]
    pub const fn new() -> Self {
        Self {
            owner: AtomicUsize::new(UNUSED_THREAD_ID_VALUE),
        }
    }

    #[inline]
    pub fn lock(&self, lock: &impl RawLock) -> RestoreState {
        // We acquire the lock inside an interrupt-free context to prevent a subtle
        // race condition:
        // In case an interrupt handler tries to lock the same resource, it could win if
        // the current thread is holding the lock but isn't yet in interrupt-free context.
        // If we maintain non-reentrant semantics, this situation would panic.
        // If we allow reentrancy, the interrupt handler would technically be a different
        // context with the same `current_thread_id`, so it would be allowed to lock the
        // resource in a theoretically incorrect way.
        let try_lock = || {
            let mut tkn = unsafe { lock.enter() };

            let current_thread_id = thread_id();

            let try_lock_result = self
                .owner
                .compare_exchange(
                    UNUSED_THREAD_ID_VALUE,
                    current_thread_id,
                    Ordering::Acquire,
                    Ordering::Relaxed,
                )
                .map(|_| ());

            match try_lock_result {
                Ok(()) => Some(tkn),
                Err(owner) if owner == current_thread_id => {
                    tkn = unsafe { RestoreState::new(tkn.inner() | (1 << 31)) };
                    // tkn.mark_reentry();
                    Some(tkn)
                }
                Err(_) => {
                    unsafe { lock.exit(tkn) };
                    None
                }
            }
        };

        loop {
            if let Some(token) = try_lock() {
                return token;
            }
        }
    }

    /// # Safety:
    ///
    /// This function must only be called if the lock was acquired by the
    /// current thread.
    #[inline]
    pub unsafe fn unlock(&self) {
        self.owner.store(UNUSED_THREAD_ID_VALUE, Ordering::Release);
    }
}

impl MovableObject for LPLockedState {
    unsafe fn move_to_main(&self, dest: *mut u8) -> Result<(), EspCoproError> {
        unsafe { copy_nonoverlapping(self as *const Self as *const u8, dest, core::mem::size_of::<Self>()); }
        Ok(())
    }

    unsafe fn move_to_lp(&self, dest: *mut u8) -> Result<(), EspCoproError> {
        unsafe { copy_nonoverlapping(self as *const Self as *const u8, dest, core::mem::size_of::<Self>()); }
        Ok(())
    }
}

/// A generic lock that wraps a [`RawLock`] implementation and tracks
/// whether the caller has locked recursively.
pub struct LPGenericRawMutex<L: RawLock> {
    lock: L,
    inner: LPLockedState,
}

// Safety: LockedState ensures thread-safety
unsafe impl<L: RawLock> Sync for LPGenericRawMutex<L> {}

impl<L: RawLock> LPGenericRawMutex<L> {
    /// Create a new lock.
    pub const fn new(lock: L) -> Self {
        Self {
            lock,
            inner: LPLockedState::new(),
        }
    }

    /// Acquires the lock.
    ///
    /// # Safety
    ///
    /// - Each release call must be paired with an acquire call.
    /// - The returned token must be passed to the corresponding `release` call.
    /// - The caller must ensure to release the locks in the reverse order they were acquired.
    #[inline]
    unsafe fn acquire(&self) -> RestoreState {
        self.inner.lock(&self.lock)
    }

    /// Releases the lock.
    ///
    /// # Safety
    ///
    /// - This function must only be called if the lock was acquired by the current thread.
    /// - The caller must ensure to release the locks in the reverse order they were acquired.
    /// - Each release call must be paired with an acquire call.
    #[inline]
    unsafe fn release(&self, token: RestoreState) {
        if token.inner() & (1 << 31) == 0 { // !token.is_reentry() {
            unsafe {
                self.inner.unlock();

                self.lock.exit(token)
            }
        }
    }

    /// Runs the callback with this lock locked.
    ///
    /// Note that this function is not reentrant, calling it reentrantly will
    /// panic.
    #[inline]
    pub fn lock_non_reentrant<R>(&self, f: impl FnOnce() -> R) -> R {
        let _token = LPLockGuard::new_non_reentrant(self);
        f()
    }

    /// Runs the callback with this lock locked.
    #[inline]
    pub fn lock<R>(&self, f: impl FnOnce() -> R) -> R {
        let _token = LPLockGuard::new_reentrant(self);
        f()
    }
}

// Currently, we do not move `L` and do not require it is [`MovableObject`].
impl<L: RawLock> MovableObject for LPGenericRawMutex<L> {
    unsafe fn move_to_main(&self, dest: *mut u8) -> Result<(), EspCoproError> {
        unsafe { copy_nonoverlapping(self as *const Self as *const u8, dest, core::mem::size_of::<Self>()); }
        Ok(())
    }

    unsafe fn move_to_lp(&self, dest: *mut u8) -> Result<(), EspCoproError> {
        unsafe { copy_nonoverlapping(self as *const Self as *const u8, dest, core::mem::size_of::<Self>()); }
        Ok(())
    }
}

/// A mutual exclusion primitive.
///
/// This lock disables interrupts on the current core while locked.
pub struct LPRawMutex {
    inner: LPGenericRawMutex<SingleCoreInterruptLock>,
}

impl Default for LPRawMutex {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl LPRawMutex {
    /// Create a new lock.
    #[inline]
    pub const fn new() -> Self {
        Self {
            inner: LPGenericRawMutex::new(SingleCoreInterruptLock),
        }
    }

    /// Acquires the lock.
    ///
    /// # Safety
    ///
    /// - Each release call must be paired with an acquire call.
    /// - The returned token must be passed to the corresponding `release` call.
    /// - The caller must ensure to release the locks in the reverse order they were acquired.
    #[inline]
    pub unsafe fn acquire(&self) -> RestoreState {
        unsafe { self.inner.acquire() }
    }

    /// Releases the lock.
    ///
    /// # Safety
    ///
    /// - This function must only be called if the lock was acquired by the current thread.
    /// - The caller must ensure to release the locks in the reverse order they were acquired.
    /// - Each release call must be paired with an acquire call.
    #[inline]
    pub unsafe fn release(&self, token: RestoreState) {
        unsafe {
            self.inner.release(token);
        }
    }

    /// Runs the callback with this lock locked.
    ///
    /// Note that this function is not reentrant, calling it reentrantly will
    /// panic.
    #[inline]
    pub fn lock_non_reentrant<R>(&self, f: impl FnOnce() -> R) -> R {
        self.inner.lock_non_reentrant(f)
    }

    /// Runs the callback with this lock locked.
    #[inline]
    pub fn lock<R>(&self, f: impl FnOnce() -> R) -> R {
        self.inner.lock(f)
    }
}

impl MovableObject for LPRawMutex {
    unsafe fn move_to_main(&self, dest: *mut u8) -> Result<(), EspCoproError> {
        unsafe { self.inner.move_to_main(dest) }
    }

    unsafe fn move_to_lp(&self, dest: *mut u8) -> Result<(), EspCoproError> {
        unsafe { self.inner.move_to_lp(dest) }
    }
}

#[cfg(feature = "has-lp-core")]
unsafe impl embassy_sync_06::blocking_mutex::raw::RawMutex for LPRawMutex {
    #[allow(clippy::declare_interior_mutable_const)]
    const INIT: Self = Self::new();

    fn lock<R>(&self, f: impl FnOnce() -> R) -> R {
        self.inner.lock(f)
    }
}

#[cfg(feature = "has-lp-core")]
unsafe impl embassy_sync_07::blocking_mutex::raw::RawMutex for LPRawMutex {
    #[allow(clippy::declare_interior_mutable_const)]
    const INIT: Self = Self::new();

    fn lock<R>(&self, f: impl FnOnce() -> R) -> R {
        self.inner.lock(f)
    }
}

#[cfg(feature = "has-lp-core")]
unsafe impl embassy_sync_08::blocking_mutex::raw::RawMutex for LPRawMutex {
    #[allow(clippy::declare_interior_mutable_const)]
    const INIT: Self = Self::new();

    fn lock<R>(&self, f: impl FnOnce() -> R) -> R {
        self.inner.lock(f)
    }
}

/// A non-reentrant (panicking) mutex.
///
/// This is largely equivalent to a `critical_section::Mutex<RefCell<T>>`, but accessing the inner
/// data doesn't hold a critical section on multi-core systems.
pub struct LPNonReentrantMutex<T> where T : MovableObject {
    lock_state: LPRawMutex,
    data: UnsafeCell<T>,
}

impl<T : MovableObject> LPNonReentrantMutex<T> {
    /// Create a new instance
    pub const fn new(data: T) -> Self {
        Self {
            lock_state: LPRawMutex::new(),
            data: UnsafeCell::new(data),
        }
    }

    /// Provide exclusive access to the protected data to the given closure.
    ///
    /// Calling this reentrantly will panic.
    pub fn with<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        self.lock_state
            .lock_non_reentrant(|| f(unsafe { &mut *self.data.get() }))
    }
}

unsafe impl<T: Send + MovableObject> Send for LPNonReentrantMutex<T> {}
unsafe impl<T: Send + MovableObject> Sync for LPNonReentrantMutex<T> {}

impl<T: MovableObject> MovableObject for LPNonReentrantMutex<T> {
    unsafe fn move_to_main(&self, dest: *mut u8) -> Result<(), EspCoproError> {
        unsafe { self.lock_state.move_to_main(dest) }?;
        unsafe { (*self.data.get()).move_to_main(dest.add(core::mem::size_of::<LPRawMutex>())) }
    }

    unsafe fn move_to_lp(&self, dest: *mut u8) -> Result<(), EspCoproError> {
        unsafe { self.lock_state.move_to_lp(dest) }?;
        unsafe { (*self.data.get()).move_to_lp(dest.add(core::mem::size_of::<LPRawMutex>())) }
    }
}

struct LPLockGuard<'a, L: RawLock> {
    lock: &'a LPGenericRawMutex<L>,
    token: RestoreState,
}

impl<'a, L: RawLock> LPLockGuard<'a, L> {
    #[inline]
    fn new_non_reentrant(lock: &'a LPGenericRawMutex<L>) -> Self {
        let this = Self::new_reentrant(lock);
        if this.token.inner() & (1 << 31) == 0 { // this.token.is_reentry() {
            panic_lock_not_reentrant();
        }
        this
    }

    #[inline]
    fn new_reentrant(lock: &'a LPGenericRawMutex<L>) -> Self {
        let token = unsafe {
            // SAFETY: the same lock will be released when dropping the guard.
            // This ensures that the lock is released on the same thread, in the reverse
            // order it was acquired.
            lock.acquire()
        };

        Self { lock, token }
    }
}

impl<L: RawLock> Drop for LPLockGuard<'_, L> {
    fn drop(&mut self) {
        unsafe { self.lock.release(self.token) };
    }
}

impl<L: RawLock> MovableObject for LPLockGuard<'_, L> {
    unsafe fn move_to_main(&self, dest: *mut u8) -> Result<(), EspCoproError> {
        unsafe { self.lock.move_to_main(dest) }
    }

    unsafe fn move_to_lp(&self, dest: *mut u8) -> Result<(), EspCoproError> {
        unsafe { self.lock.move_to_lp(dest) }
    }
}

#[inline(never)]
#[cold]
fn panic_lock_not_reentrant() -> ! {
    panic!("lock is not reentrant");
}