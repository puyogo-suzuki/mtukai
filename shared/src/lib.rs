#![no_std]

use core::option::Option;
use esp_rs_copro::{movableobject::MovableObject, lpbox::LPBox};

pub struct TestList {
    pub next : Option<LPBox<TestList>>,
    pub value : i32
}

impl TestList {
    unsafe fn option_move_to_main(&self, dest : *mut u8) { unsafe {
        let dest = dest as * mut Option<LPBox<TestList>>;
        dest.write_volatile(match self.next.as_ref() {
            Some(boxed) => { 
                let addr  = boxed.get_moved_to_main();
                Some(addr)
            },
            None => { None }
        });
    }}

    unsafe fn option_move_to_lp(&self, dest : *mut u8) { unsafe {
        let dest = dest as * mut Option<LPBox<TestList>>;
        dest.write_volatile( match self.next.as_ref() {
            Some(boxed) => { Some(boxed.get_moved_to_lp()) },
            None => { None }
        });
    }}
}

impl MovableObject for TestList {
    unsafe fn move_to_main(&self, dest : *mut u8) { unsafe {
        let dest = dest as * mut TestList;
        (*dest).value = self.value;
        self.option_move_to_main((&mut (*dest).next) as * mut Option<LPBox<TestList>> as * mut u8);
    }}

    unsafe fn move_to_lp(&self, dest : *mut u8) { unsafe {
        let dest = dest as * mut TestList;
        (*dest).value = self.value;
        self.option_move_to_lp((&mut (*dest).next) as * mut Option<LPBox<TestList>> as * mut u8);
    }}
}