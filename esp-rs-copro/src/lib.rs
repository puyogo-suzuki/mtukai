#![no_std]

pub trait MovableObject {
    fn move_to_main(&self) -> *mut u8;
    unsafe fn move_to_lp(&self) -> *mut u8;
}
