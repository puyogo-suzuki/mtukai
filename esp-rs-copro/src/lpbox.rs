use core::{any::Any, arch::asm, mem, ptr::{null, NonNull}};
use alloc::boxed::Box;
use crate::MovableObject;

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

fn lpbox_alloc(l : core::alloc::Layout) -> *mut u8 {
    unsafe {
        let ptr = alloc::alloc::alloc(l);
        if ptr.is_null() { alloc::alloc::handle_alloc_error(l); }
        ptr
    }
}

impl<T: MovableObject> LPBox<T> {
    pub fn new(value: T) -> Self { unsafe {
        let ptr = lpbox_alloc(core::alloc::Layout::new::<T>()) as *mut T;
        lpalloc::write_vtable(ptr as * mut u8, get_vtable(&value) as * mut u8);
        ptr.write(value);
        LPBox(NonNull::new_unchecked(ptr))
    }}
}