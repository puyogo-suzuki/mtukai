#![no_std]

use core::{option::Option, ptr};
use esp_rs_copro::{movableobject::MovableObject, lpbox::LPBox, io::i2c::LPI2C};

pub struct TempAndHumid {
    pub temperature : i32,
    pub humidity : i32
}

impl MovableObject for TempAndHumid {
    unsafe fn move_to_main(&self, dest : *mut u8) { unsafe {
        ptr::copy_nonoverlapping(self, dest as * mut TempAndHumid, 1);
    }}

    unsafe fn move_to_lp(&self, dest : *mut u8) { unsafe {
        ptr::copy_nonoverlapping(self, dest as * mut TempAndHumid, 1);
    }}
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
    unsafe fn move_to_main(&self, dest : *mut u8) { unsafe {
        let dest = dest as * mut MainLPParcel;
        self.i2c.move_to_main((&mut (*dest).i2c) as * mut LPI2C as * mut u8);
        self.result.move_to_main((&mut (*dest).result) as * mut LPBox<[TempAndHumid]> as * mut u8);
    }}

    unsafe fn move_to_lp(&self, dest : *mut u8) { unsafe {
        let dest = dest as * mut MainLPParcel;
        self.i2c.move_to_lp((&mut (*dest).i2c) as * mut LPI2C as * mut u8);
        self.result.move_to_lp((&mut (*dest).result) as * mut LPBox<[TempAndHumid]> as * mut u8);
    }}
}

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