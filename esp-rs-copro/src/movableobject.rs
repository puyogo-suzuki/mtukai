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

impl<T : MovableObject, const N : usize> MovableObject for [T; N] {
    unsafe fn move_to_main(&self, dest: *mut u8) { unsafe {
        let dest = dest as * mut T;
        for i in 0..N {
            self.get_unchecked(i).move_to_main(dest.offset(i as isize) as * mut u8);
        }
    }}

    unsafe fn move_to_lp(&self, dest: *mut u8) { unsafe {
        let dest = dest as * mut T;
        for i in 0..N {
            self.get_unchecked(i).move_to_lp(dest.offset(i as isize) as * mut u8);
        }
    }}
}

impl<T : MovableObject> MovableObject for [T] {
    unsafe fn move_to_main(&self, dest: *mut u8) { unsafe {
        let dest = dest as * mut T;
        for i in 0..self.len() {
            self.get_unchecked(i).move_to_main(dest.offset(i as isize) as * mut u8);
        }
    }}

    unsafe fn move_to_lp(&self, dest: *mut u8) { unsafe {
        let dest = dest as * mut T;
        for i in 0..self.len() {
            self.get_unchecked(i).move_to_lp(dest.offset(i as isize) as * mut u8);
        }
    }}
}

impl MovableObject for i32 {
    unsafe fn move_to_main(&self, dest: *mut u8) { unsafe {
        let dest = dest as * mut i32;
        *dest = *self;
    }}

    unsafe fn move_to_lp(&self, dest: *mut u8) { unsafe {
        let dest = dest as * mut i32;
        *dest = *self;
    }}
}

impl<T : MovableObject> MovableObjectCopy for T {
    unsafe fn copy_to_main(&self) -> Self {
        if core::mem::size_of_val(self) <= 32 {
            let buf : MaybeUninit<T> = mem::MaybeUninit::uninit();
            unsafe{
                self.move_to_main(buf.as_ptr() as * mut u8);
                buf.assume_init()
            }
        } else {
            let buf : Box<MaybeUninit<T>> = Box::new(mem::MaybeUninit::uninit());
            unsafe {
                self.move_to_main(buf.as_ptr() as * mut u8);
                *(buf.assume_init())
            }
        }
    }

    unsafe fn copy_to_lp(&self) -> Self {
        if core::mem::size_of_val(self) <= 32 {
            let buf : MaybeUninit<T> = mem::MaybeUninit::uninit();
            unsafe {
                self.move_to_lp(buf.as_ptr() as * mut u8);
                buf.assume_init()
            }
        } else {
            let buf : Box<MaybeUninit<T>> = Box::new(mem::MaybeUninit::uninit());
            unsafe {
                self.move_to_lp(buf.as_ptr() as * mut u8);
                *(buf.assume_init())
            }
        }
    }
}