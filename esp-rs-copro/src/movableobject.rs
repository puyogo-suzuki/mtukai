pub trait MovableObject {
    unsafe fn rewrite_pointers_to_main(&self, dest : *mut u8);
    unsafe fn rewrite_pointers_to_lp(&self, dest : *mut u8);
    unsafe fn move_to_main(&self, dest : *mut u8) { unsafe {
        core::ptr::copy_nonoverlapping(self as *const Self as *const u8, dest as * mut u8, core::mem::size_of_val(self));
        self.rewrite_pointers_to_main(dest);
    }}
    unsafe fn move_to_lp(&self, dest : *mut u8) { unsafe {
        core::ptr::copy_nonoverlapping(self as *const Self as *const u8, dest as * mut u8, core::mem::size_of_val(self));
        self.rewrite_pointers_to_lp(dest);
    }}
}

impl<T : MovableObject, const N : usize> MovableObject for [T; N] {
    unsafe fn rewrite_pointers_to_main(&self, dest: *mut u8) { unsafe {
        let dest = dest as * mut T;
        for i in 0..N {
            self.get_unchecked(i).rewrite_pointers_to_main(dest.offset(i as isize) as * mut u8);
        }
    }}

    unsafe fn rewrite_pointers_to_lp(&self, dest: *mut u8) { unsafe {
        let dest = dest as * mut T;
        for i in 0..N {
            self.get_unchecked(i).rewrite_pointers_to_lp(dest.offset(i as isize) as * mut u8);
        }
    }}
}

impl<T : MovableObject> MovableObject for [T] {
    unsafe fn rewrite_pointers_to_main(&self, dest: *mut u8) { unsafe {
        let dest = dest as * mut T;
        for i in 0..self.len() {
            self.get_unchecked(i).rewrite_pointers_to_main(dest.offset(i as isize) as * mut u8);
        }
    }}

    unsafe fn rewrite_pointers_to_lp(&self, dest: *mut u8) { unsafe {
        let dest = dest as * mut T;
        for i in 0..self.len() {
            self.get_unchecked(i).rewrite_pointers_to_lp(dest.offset(i as isize) as * mut u8);
        }
    }}
}

impl<T : MovableObject> MovableObject for Option<T> {
    unsafe fn rewrite_pointers_to_main(&self, dest : *mut u8) { unsafe {
        let dest = dest as * mut Option<T>;
        dest.write_volatile( match self {
            Some(v) => {
                let mut val : core::mem::MaybeUninit<T> = core::mem::MaybeUninit::uninit();
                v.rewrite_pointers_to_main(val.as_mut_ptr() as * mut u8);
                Some(val.assume_init())
            },
            None => { None }
        });
    }}

    unsafe fn rewrite_pointers_to_lp(&self, dest : *mut u8) { unsafe {
        let dest = dest as * mut Option<T>;
        dest.write_volatile( match self {
            Some(v) => { 
                let mut val : core::mem::MaybeUninit<T> = core::mem::MaybeUninit::uninit();
                v.rewrite_pointers_to_lp(val.as_mut_ptr() as * mut u8);
                Some(val.assume_init())
            },
            None => { None }
        });
    }}
}