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
mod constants;
#[cfg(not(feature = "is-lp-core"))]
mod addresstranslation;
#[cfg(feature = "has-lp-core")]
#[macro_use]
extern crate esp_println;

#[cfg(feature = "has-lp-core")]
pub mod transfer_functions {
    use crate::{lpbox::{LPBox, cleanup, remove_by_main}, movableobject::MovableObject};

    pub fn transfer_to_lp<T : MovableObject>(src : &T) -> *mut u8 {
        LPBox::<T>::write_to_lp(src)
    }

    pub fn transfer_to_main<T : MovableObject>(src : &mut T, dst : * mut u8) {
        unsafe{(dst as * mut T).as_ref().unwrap().move_to_main(src as * const T as * mut u8);}
        remove_by_main(src as * const T as usize);
        cleanup();
    }
}
