#![no_std]

use esp_rs_copro::{io::gpio::LPOutput, collections::lpvec::LPVec};

#[derive(Clone, Copy, esp_rs_copro_procmacro::MovableObject)]
pub struct Note {
    pub frequency : u16,
    pub duration : u16
}

impl Note {
    pub fn new(frequency : u16, duration : u16) -> Self {
        Note {
            frequency,
            duration
        }
    }
    pub fn rest(duration : u16) -> Self {
        Note::new(0, duration)
    }
    pub fn c4(duration : u16) -> Self {
        Note::new(261, duration)
    }
    pub fn d4(duration : u16) -> Self {
        Note::new(293, duration)
    }
    pub fn e4(duration : u16) -> Self {
        Note::new(329, duration)
    }
    pub fn f4(duration : u16) -> Self {
        Note::new(349, duration)
    }
    pub fn g4(duration : u16) -> Self {
        Note::new(392, duration)
    }
    pub fn a4(duration : u16) -> Self {
        Note::new(440, duration)
    }
    pub fn b4(duration : u16) -> Self {
        Note::new(493, duration)
    }
}

#[derive(esp_rs_copro_procmacro::MovableObject)]
pub struct MainLPParcel<'a> {
    pub score : LPVec<Note>,
    pub outpin : LPOutput<'a, 1>
}