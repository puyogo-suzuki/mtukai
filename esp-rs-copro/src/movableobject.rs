use core::mem::{self, MaybeUninit};
use alloc::boxed::Box;

pub trait MovableObject {
    unsafe fn move_to_main(&self, dest : *mut u8);
    unsafe fn move_to_lp(&self, dest : *mut u8);
}

pub trait MovableObjectCopy {
    unsafe fn copy_to_main(&self) -> Self;
    unsafe fn copy_to_lp(&self) -> Self;
}
pub trait MovableObjectCopy2 {
    unsafe fn copy_to_main(&self) -> Self;
    unsafe fn copy_to_lp(&self) -> Self;
}

impl<T : Copy> MovableObjectCopy2 for T {
    unsafe fn copy_to_main(&self) -> Self { *self }
    unsafe fn copy_to_lp(&self) -> Self { *self }
}

impl<T : MovableObject> MovableObjectCopy for T {
    unsafe fn copy_to_main(&self) -> Self {
        if core::mem::size_of_val(self) <= 32 {
            let buf : MaybeUninit<T> = mem::MaybeUninit::uninit();
            self.move_to_main(buf.as_ptr() as * mut u8);
            buf.assume_init()
        } else {
            let buf : Box<MaybeUninit<T>> = Box::new(mem::MaybeUninit::uninit());
            self.move_to_main(buf.as_ptr() as * mut u8);
            *(buf.assume_init())
        }
    }

    unsafe fn copy_to_lp(&self) -> Self {
        if core::mem::size_of_val(self) <= 32 {
            let buf : MaybeUninit<T> = mem::MaybeUninit::uninit();
            self.move_to_lp(buf.as_ptr() as * mut u8);
            buf.assume_init()
        } else {
            let buf : Box<MaybeUninit<T>> = Box::new(mem::MaybeUninit::uninit());
            self.move_to_lp(buf.as_ptr() as * mut u8);
            *(buf.assume_init())
        }
    }
}