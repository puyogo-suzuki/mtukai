use core::{alloc::GlobalAlloc, mem, ops::{Deref, DerefMut}, ptr::NonNull};

use crate::MovableObject;
#[cfg(any(feature = "has-lp-core", test))]
use crate::addresstranslation::AddressTranslationTable;
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

#[cfg(any(feature = "has-lp-core", test))]
static mut ADDRESS_TRANSLATION_TABLE : AddressTranslationTable = AddressTranslationTable::new();


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

    pub fn write_to_lp(value : &T) -> * mut u8 { unsafe {
        let ptr = lpalloc::lp_allocator_alloc(core::alloc::Layout::new::<T>()) as * mut u8;
        value.move_to_lp(ptr);
        // ptr.copy_from(value, layout.size());
        lpalloc::write_vtable(ptr as * mut u8, get_vtable(value) as * mut u8);
        #[cfg(any(feature = "has-lp-core", test))]
        ADDRESS_TRANSLATION_TABLE.insert(value as * const T as * const () as usize, ptr as usize);
        ptr
    }}

    pub fn move_to_lp(&self) -> LPBox<T>{
        unsafe { LPBox(NonNull::new_unchecked(Self::write_to_lp(self.0.as_ref()) as * mut T))}
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

impl<T : ?Sized + MovableObject> Drop for LPBox<T>{
    fn drop(&mut self) {
        unsafe{self.0.drop_in_place();}
        let addr : usize = self.0.as_ptr() as *mut () as usize;
        let ptr = self.0.as_ptr() as * mut u8;
        let lay = unsafe {core::alloc::Layout::for_value(self.0.as_ref())};
        if addr >= 0x5000_0000 && addr < 0x5004_0000 {
            unsafe{lpalloc::lp_allocator_dealloc(ptr, lay);} // lp coprocessor
        } else {
            unsafe{alloc::alloc::dealloc(ptr, lay);} // main processor
        }
    }
}