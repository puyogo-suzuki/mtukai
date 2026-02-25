#![cfg_attr(feature="nottest", no_std)]
#![feature(layout_for_ptr)]
#![feature(ptr_internals)]
#![feature(temporary_niche_types)]
#![feature(sized_type_properties)]
#![feature(rustc_attrs)]
#![feature(const_trait_impl)]
#![feature(const_default)]
#![feature(core_intrinsics)]
#![feature(slice_range)]
#![feature(specialization)]
#![feature(new_range_api)]
#![feature(cast_maybe_uninit)]
#![feature(trusted_len)]
#![feature(exact_size_is_empty)]
#![feature(unsafe_cell_access)]
#![feature(cfg_target_has_atomic)]
#[cfg(feature = "nottest")]
extern crate alloc;
/// This module provides functions and types related to memory allocation in the LP coprocessor. It includes a custom allocator implementation, as well as functions for checking whether an address is in the LP memory range.
pub mod lpalloc;
/// This module provides a smart pointer type, [`LPBox<T>`][crate::lpbox::LPBox], which is similar to [`Box<T>`] but supports allocations on the LP memory and can be transferred between the main and the LP coprocessors.
pub mod lpbox;
/// This module provides an adapter, which automatically implements [`MovableObject`][crate::movableobject::MovableObject] for types that implement [`Copy`].
pub mod lpadapter;
/// This module provides a trait, [`MovableObject`][crate::movableobject::MovableObject], for types that can be moved between the main and the LP coprocessors.
pub mod movableobject;
#[doc(hidden)]
pub mod movableobjectwrapper;
/// This module provides I/O drivers that can be transferred between the main and the LP coprocessors.
pub mod io;
/// This module provides [LPVec<T>][crate::collections::lpvec::LPVec] for heap collections that work with the LP memory.
pub mod collections;
pub mod prelude;
#[cfg(not(feature = "is-lp-core"))]
mod addresstranslation;
#[cfg(feature = "has-lp-core")]
#[macro_use]
extern crate esp_println;

/// This represents an error that can occur during the transfer of a value between the main and the LP coprocessors.
#[derive(Debug, Hash, Clone, Copy, Eq, PartialEq)]
pub enum EspCoproError {
    /// The transfer is not allowed. This can occur when you try to transfer on the LP coprocessor.
    NotAllowed,
    /// Something wrong. This is bug of this library.
    IncorrectlyTransferred,
    /// Out of memory. This can occur when the LP memory is out of memory.
    OutOfMemory,
    /// The LP coprocessor is in use. Have you started the LP coprocessor on the different thread?
    InUse
}

/// This is for internal-use.
/// This is used to prevent multiple threads from running the LP coprocessor at the same time.
#[cfg(target_has_atomic_load_store = "8")]
static ESP_COPRO_MUTEX : core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);

/// This is for internal-use.
/// Try to acquire the lock for the LP coprocessor. If the lock is already acquired, return an error.
/// If the MCU does not support atomic operations, this function always succeeds.
pub fn try_copro_lock() -> Result<(), EspCoproError> {
    #[cfg(target_has_atomic_load_store = "8")]
    if let Err(_) = ESP_COPRO_MUTEX.compare_exchange(false, true, core::sync::atomic::Ordering::Relaxed, core::sync::atomic::Ordering::Relaxed) {
        return Err(EspCoproError::InUse);
    }
    Ok(())
}

/// This is for internal-use.
/// Release the lock for the LP coprocessor. This should be called after you finish using the LP coprocessor.
/// If the MCU does not support atomic operations, this function does nothing.
pub fn copro_unlock() {
    #[cfg(target_has_atomic_load_store = "8")]
    ESP_COPRO_MUTEX.store(false, core::sync::atomic::Ordering::Relaxed);
}

impl core::fmt::Display for EspCoproError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            EspCoproError::NotAllowed => write!(f, "EspCoproError::NotAllowed"),
            EspCoproError::IncorrectlyTransferred => write!(f, "T  EspCoproError::IncorrectlyTransferred"),
            EspCoproError::OutOfMemory => write!(f, "EspCoproError::OutOfMemory"),
            EspCoproError::InUse => write!(f, "EspCoproError::InUse")
        }
    }
}

#[cfg(feature = "has-lp-core")]
pub mod transfer_functions {
    use crate::{lpbox::{LPBox, cleanup, remove_by_main}, movableobject::MovableObject};
    use crate::EspCoproError;
    /// This is used in esp-rs-copro-procmacro.
    /// Transfers a value from the main coprocessor to the LP coprocessor.
    /// The value is moved, and the ownership is transferred to the LP coprocessor.
    /// The caller must ensure that the value is not used on the main coprocessor after this function is called.
    pub fn transfer_to_lp<T : MovableObject>(src : &T) -> Result<*mut u8, EspCoproError> {
        LPBox::<T>::write_to_lp(src)
    }

    /// This is used in esp-rs-copro-procmacro.
    /// Transfers a value from the LP coprocessor to the main coprocessor.
    /// The value is moved, and the ownership is transferred to the main coprocessor.
    /// The caller must ensure that the value is not used on the LP coprocessor after this function is called.
    pub unsafe fn transfer_to_main<T : MovableObject>(src : * const u8, dst : &mut T) -> Result<(), EspCoproError> {
        if let Some(v) = unsafe{(src as * const T).as_ref()} {
            unsafe{v.move_to_main(dst as * mut T as * mut u8)?;}
            remove_by_main(dst as * mut T as usize);
            cleanup();
            Ok(())
        } else {
            Err(EspCoproError::IncorrectlyTransferred)
        }
    }
}
