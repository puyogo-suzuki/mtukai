use core::{alloc::GlobalAlloc, mem, ops::{Deref, DerefMut}, ptr::NonNull};

use crate::MovableObject;
use crate::lpalloc;

pub struct LPBox<T: ?Sized + MovableObject>(pub(crate) NonNull<T>);

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

impl<T: Sized + MovableObject> LPBox<T> {
    pub fn new(value: T) -> Self { unsafe {
        let ptr = lpbox_alloc(core::alloc::Layout::new::<T>()) as *mut T;
        ptr.write(value);
        LPBox(NonNull::new_unchecked(ptr))
    }}

    // fn new_on_lp(value: T, allocator: &dyn lpalloc::LPAlloc) -> Self { unsafe {
    //     let layout = core::alloc::Layout::new::<T>();
    //     let ptr = allocator.alloc_on_lp(layout) as * mut T;
    //     lpalloc::write_vtable(ptr as * mut u8, get_vtable(&value) as * mut u8);
    //     ptr.write(value);
    //     LPBox(NonNull::new_unchecked(ptr))
    // }}

    fn new_on_lp(value: &T, allocator: &dyn GlobalAlloc) -> Self { unsafe {
        let ptr = value.move_to_lp(allocator);
        // ptr.copy_from(value, layout.size());
        lpalloc::write_vtable(ptr as * mut u8, get_vtable(value) as * mut u8);
        LPBox(NonNull::new_unchecked(ptr as *mut T))
    }}

    pub fn move_to_lp(&self, allocator: &dyn GlobalAlloc) -> LPBox<T>{
        match self {
            LPBox(p) => unsafe { LPBox::new_on_lp(p.as_ref(), allocator)}
        }
    }
}

impl<T : MovableObject> Deref for LPBox<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { self.0.as_ref() }
    }
}
impl<T : MovableObject> DerefMut for LPBox<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.0.as_mut() }
    }
}

// impl<T : ?Sized + MovableObject> Drop for LPBox<T>{
//     fn drop(&mut self) {
//         unsafe{alloc::alloc::dealloc(self.0.as_ptr() as *mut u8, core::alloc::Layout::for_value(self.0.as_ref()));}
//     }
// }