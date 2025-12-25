#![no_std]
#![feature(layout_for_ptr)]
extern crate alloc;
pub mod lpalloc;
pub mod lpbox;
pub mod movableobject;
pub mod movableobjectwrapper;
pub mod io;
pub mod prelude;
#[cfg(any(feature = "has-lp-core", test))]
mod addresstranslation;
#[cfg(feature = "has-lp-core")]
#[macro_use]
extern crate esp_println;

// use alloc::alloc::Allocator;

#[cfg(any(feature = "has-lp-core", test))]
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
