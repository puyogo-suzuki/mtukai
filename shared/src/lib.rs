#![no_std]

use core::option::Option;
use esp_rs_copro::{movableobject::MovableObject, lpbox::LPBox, io::i2c::LPI2C};

#[derive(Clone, Copy, esp_rs_copro_procmacro::MovableObject)]
pub struct TempAndHumid {
    pub temperature : i32,
    pub humidity : i32
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

#[derive(esp_rs_copro_procmacro::MovableObject)]
pub struct MainLPParcel{
    pub i2c : LPI2C,
    pub result : LPBox<[TempAndHumid]>,
}

#[derive(esp_rs_copro_procmacro::MovableObject)]
pub struct TestList {
    pub next : Option<LPBox<TestList>>,
    pub value : i32
}