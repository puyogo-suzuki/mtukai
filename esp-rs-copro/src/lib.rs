#![no_std]
extern crate alloc;
pub mod lpalloc;
pub mod lpbox;
pub mod movableobject;
#[cfg(any(feature = "has-lp-core", test))]
mod addresstranslation;
#[cfg(feature = "has-lp-core")]
#[macro_use]
extern crate esp_println;

use core::mem::ManuallyDrop;

// use alloc::alloc::Allocator;
use crate::{lpbox::LPBox, movableobject::MovableObject};
pub struct TestList {
    pub next : Option<LPBox<TestList>>,
    pub value : i32
}

#[cfg(any(feature = "has-lp-core", test))]
pub mod transfer_functions {
    use crate::{lpbox::{ADDRESS_TRANSLATION_TABLE, LPBox, cleanup}, movableobject::MovableObject};

    pub fn transfer_to_lp<T : MovableObject>(src : &T) -> *mut u8 {
        LPBox::<T>::write_to_lp(src)
    }

    pub fn transfer_to_main<T : MovableObject>(src : &mut T, dst : * mut u8) {
        println!("transfer_to_main");
        unsafe{(dst as * mut T).as_ref().unwrap().move_to_main(src as * const T as * mut u8);}
        println!("transfer_to_main2");
        cleanup();
    }
}

impl MovableObject for Option<LPBox<TestList>> {
    unsafe fn move_to_main(&self, dest : *mut u8) { unsafe {
        let dest = dest as * mut Option<LPBox<TestList>>;
        dest.write_volatile(match self {
            Some(boxed) => { 
                let addr  = boxed.get_moved_to_main();
                Some(addr)
            },
            None => { None }
        });
    }}

    unsafe fn move_to_lp(&self, dest : *mut u8) { unsafe {
        let dest = dest as * mut Option<LPBox<TestList>>;
        dest.write_volatile( match self {
            Some(boxed) => { Some(boxed.get_moved_to_lp()) },
            None => { None }
        });
    }}
}

impl MovableObject for TestList {
    unsafe fn move_to_main(&self, dest : *mut u8) { unsafe {
        let dest = dest as * mut TestList;
        (*dest).value = self.value;
        self.next.move_to_main((&mut (*dest).next) as * mut Option<LPBox<TestList>> as * mut u8);
    }}

    unsafe fn move_to_lp(&self, dest : *mut u8) { unsafe {
        let dest = dest as * mut TestList;
        (*dest).value = self.value;
        self.next.move_to_lp((&mut (*dest).next) as * mut Option<LPBox<TestList>> as * mut u8);
    }}
}