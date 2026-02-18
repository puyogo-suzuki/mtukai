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
#[cfg(feature = "nottest")]
extern crate alloc;
pub mod lpalloc;
pub mod lpbox;
pub mod lpadapter;
pub mod movableobject;
pub mod movableobjectwrapper;
pub mod io;
pub mod collections;
pub mod prelude;
#[cfg(not(feature = "is-lp-core"))]
mod addresstranslation;
#[cfg(feature = "has-lp-core")]
#[macro_use]
extern crate esp_println;

#[derive(Debug, Hash, Clone, Copy, Eq, PartialEq)]
pub enum EspCoproError {
    NotAllowed,
    IncorrectlyTransferred,
    OutOfMemory
}

impl core::fmt::Display for EspCoproError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            EspCoproError::NotAllowed => write!(f, "EspCoproError::NotAllowed"),
            EspCoproError::IncorrectlyTransferred => write!(f, "T  EspCoproError::IncorrectlyTransferred"),
            EspCoproError::OutOfMemory => write!(f, "EspCoproError::OutOfMemory"),
        }
    }
}

#[cfg(feature = "has-lp-core")]
pub mod transfer_functions {
    use crate::{lpbox::{LPBox, cleanup, remove_by_main}, movableobject::MovableObject};
    use crate::EspCoproError;
    pub fn transfer_to_lp<T : MovableObject>(src : &T) -> Result<*mut u8, EspCoproError> {
        LPBox::<T>::write_to_lp(src)
    }

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
