#![no_std]
#![feature(allocator_api)]
extern crate alloc;
pub mod lpalloc;
pub mod lpbox;

use core::alloc::GlobalAlloc;

// use alloc::alloc::Allocator;

use crate::lpbox::LPBox;

pub trait MovableObject {
    // fn move_to_main(&self) -> *mut u8;
    unsafe fn move_to_lp(&self, allocator: &dyn GlobalAlloc) -> *mut u8; // *mut Self is not allowed in Rust Runtime System.
}

pub struct TestList {
    pub next : Option<LPBox<TestList>>,
    pub value : i32
}

impl MovableObject for TestList {
    unsafe fn move_to_lp(&self, allocator: &dyn GlobalAlloc) -> *mut u8 { unsafe {
        let ptr = allocator.alloc(core::alloc::Layout::new::<TestList>()) as * mut TestList;
        ptr.write(TestList {
            next: self.next.as_ref().map(|v| { v.move_to_lp(allocator)}),
            value: self.value
        });
        ptr as *mut u8
    }}
}