use core::{any::Any, arch::asm, mem, ptr::{null, NonNull}};
use alloc::boxed::Box;
use esp_rs_copro::MovableObject;

use crate::lpalloc;
extern crate alloc;

pub struct LPBox<T: ?Sized + MovableObject>(NonNull<T>);

fn get_vtable(obj: &dyn MovableObject) -> *const u8 {
    let fat_ptr_addr = obj as *const dyn MovableObject as *const [usize; 2];
    unsafe{
        let vtable_ptr_addr = fat_ptr_addr.add(1);
        mem::transmute_copy(&vtable_ptr_addr)
    }
}

impl<T: MovableObject> LPBox<T> {
    pub fn new(value: T) -> Self {
        let layout = core::alloc::Layout::new::<T>();
        unsafe {
            let ptr = alloc::alloc::alloc(layout) as *mut T;
            if ptr.is_null() {
                alloc::alloc::handle_alloc_error(layout);
            }
            let vt = get_vtable(&value);
            lpalloc::write_vtable(ptr as * mut u8, vt as * mut u8);
            ptr.write(value);
            LPBox(NonNull::new_unchecked(ptr))
        }
    }
}