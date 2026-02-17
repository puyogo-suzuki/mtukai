use core::{intrinsics::copy_nonoverlapping, mem::MaybeUninit};

pub trait MovableObject {
    unsafe fn move_to_main(&self, dest : *mut u8);
    unsafe fn move_to_lp(&self, dest : *mut u8);
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

impl<T : MovableObject> MovableObject for Option<T> {
    unsafe fn move_to_main(&self, dest : *mut u8) { unsafe {
        let dest = dest as * mut Option<T>;
        // BELIEVE Compiler Optimization!
        dest.write_volatile( match self {
            Some(v) => {
                let mut val : core::mem::MaybeUninit<T> = core::mem::MaybeUninit::uninit();
                v.move_to_main(val.as_mut_ptr() as * mut u8);
                Some(val.assume_init())
            },
            None => { None }
        });
    }}

    unsafe fn move_to_lp(&self, dest : *mut u8) { unsafe {
        let dest = dest as * mut Option<T>;
        // BELIEVE Compiler Optimization!
        dest.write_volatile( match self {
            Some(v) => { 
                let mut val : core::mem::MaybeUninit<T> = core::mem::MaybeUninit::uninit();
                v.move_to_lp(val.as_mut_ptr() as * mut u8);
                Some(val.assume_init())
            },
            None => { None }
        });
    }}
}

impl MovableObject for () {
    unsafe fn move_to_main(&self, dest: *mut u8) {}
    unsafe fn move_to_lp(&self, dest: *mut u8) {}
}

// We only support MaybeUninit of Copy types, because otherwise we would need to move the value out of the MaybeUninit in order to move it, which would prevent us from leaving the original MaybeUninit uninitialized in the case where the value is not actually initialized.
impl<T : MovableObject + Copy> MovableObject for MaybeUninit<T> {
    unsafe fn move_to_main(&self, dest: *mut u8) {
        unsafe { copy_nonoverlapping(self as * const Self, dest as * mut Self, 1); }
    }

    unsafe fn move_to_lp(&self, dest: *mut u8) {
        unsafe { copy_nonoverlapping(self as * const Self, dest as * mut Self, 1); }
    }
}