#![no_std]

use core::option::Option;
use esp_rs_copro::{movableobject::MovableObject, lpbox::LPBox, io::i2c::LPI2C};

pub struct TempAndHumid {
    pub temperature : i32,
    pub humidity : i32
}

impl MovableObject for TempAndHumid {
    unsafe fn rewrite_pointers_to_main(&self, _dest : *mut u8) {
        // Do nothing, TempAndHumid has no pointers.
    }
    unsafe fn rewrite_pointers_to_lp(&self, _dest : *mut u8) {
        // Do nothing, TempAndHumid has no pointers.
    }
}

impl TempAndHumid {
    pub fn new(temperature : i32, humidity : i32) -> Self {
        TempAndHumid {
            temperature,
            humidity
        }
    }
    pub fn zero() -> Self {
        TempAndHumid {temperature: 0, humidity: 0}
    }
}

pub struct MainLPParcel{
    pub i2c : LPI2C,
    pub result : LPBox<[TempAndHumid]>,
}

impl MovableObject for MainLPParcel {
    unsafe fn rewrite_pointers_to_main(&self, dest : *mut u8) { unsafe {
        let dest = dest as * mut MainLPParcel;
        self.i2c.rewrite_pointers_to_main((&mut (*dest).i2c) as * mut LPI2C as * mut u8);
        self.result.rewrite_pointers_to_main((&mut (*dest).result) as * mut LPBox<[TempAndHumid]> as * mut u8);
    }}

    unsafe fn rewrite_pointers_to_lp(&self, dest : *mut u8) { unsafe {
        let dest = dest as * mut MainLPParcel;
        self.i2c.rewrite_pointers_to_lp((&mut (*dest).i2c) as * mut LPI2C as * mut u8);
        self.result.rewrite_pointers_to_lp((&mut (*dest).result) as * mut LPBox<[TempAndHumid]> as * mut u8);
    }}
}

pub struct TestList {
    pub next : Option<LPBox<TestList>>,
    pub value : i32
}

impl MovableObject for TestList {
    unsafe fn rewrite_pointers_to_main(&self, dest : *mut u8) { unsafe {
        let dest = dest as * mut TestList;
        self.next.rewrite_pointers_to_main((&mut (*dest).next) as * mut Option<LPBox<TestList>> as * mut u8);
    }}

    unsafe fn rewrite_pointers_to_lp(&self, dest : *mut u8) { unsafe {
        let dest = dest as * mut TestList;
        self.next.rewrite_pointers_to_lp((&mut (*dest).next) as * mut Option<LPBox<TestList>> as * mut u8);
    }}
}