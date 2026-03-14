#![no_std]

use core::option::Option;
use esp_rs_copro::{io::i2c::LPI2C, collections::lpvec::LPVec};

#[derive(Clone, Copy, esp_rs_copro_procmacro::MovableObject)]
pub struct Vector3D {
    pub x : i16,
    pub y : i16,
    pub z : i16 
}

impl Vector3D {
    pub fn new(x : i16, y : i16, z : i16) -> Self {
        Vector3D {
            x,
            y,
            z
        }
    }
    pub fn zero() -> Self {
        Vector3D {x: 0, y: 0, z: 0}
    }
}

#[derive(esp_rs_copro_procmacro::MovableObject)]
pub struct MainLPParcel{
    pub i2c : LPI2C,
    pub measurement_count : usize,
    pub result : LPVec<Option<Vector3D>>
}