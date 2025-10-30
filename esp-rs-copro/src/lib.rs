#![no_std]
#![feature(allocator_api)]
extern crate alloc;
pub mod lpalloc;
pub mod lpbox;
pub mod movableobject;
mod addresstranslation;

// use alloc::alloc::Allocator;

use crate::{lpbox::LPBox, movableobject::MovableObject};
pub struct TestList {
    pub next : Option<LPBox<TestList>>,
    pub value : i32
}

impl MovableObject for TestList {
    unsafe fn move_to_main(&self, dest : *mut u8) { unsafe {
        let dest = dest as * mut TestList;
        dest.write(TestList {
            next: self.next.as_ref().map(|v| { 
                v.move_to_lp()
            }),
            value: self.value
        });
    }}

    unsafe fn move_to_lp(&self, dest : *mut u8) { unsafe {
        let dest = dest as * mut TestList;
        dest.write(TestList {
            next: self.next.as_ref().map(|v| { v.move_to_lp() }),
            value: self.value
        });
    }}
}