use core::{alloc::{GlobalAlloc, Layout}, cell::UnsafeCell, mem::MaybeUninit, ptr::null_mut};
extern crate alloc;
pub struct LPAllocator<const SIZE: usize> {
    allocated : usize,
    free_ptr : UnsafeCell<*mut BlockHeader>,
    heap : [MaybeUninit<u8>; SIZE]
}

struct BlockHeader {
    next: * mut BlockHeader,
    vtable: * mut u8,
    prev: * mut BlockHeader,
    size: usize, // whole size including this header.
}

impl BlockHeader {
    unsafe fn get_value<T>(this : *mut Self) -> * mut T {
        (this as * const BlockHeader as usize + core::mem::size_of::<BlockHeader>()) as * mut T
    }
    const unsafe fn init_header_value(this : *mut Self, size : usize, vtable : *mut u8, prev : * mut BlockHeader, next : * mut BlockHeader) { unsafe {
        (*this).size = size;
        (*this).vtable = vtable;
        (*this).prev = prev;
        (*this).next = next;
    }}
}

pub unsafe fn write_vtable(ptr: * mut u8, vtable: * mut u8) {
    let header = (ptr as usize - core::mem::size_of::<BlockHeader>()) as * mut BlockHeader;
    unsafe {
        (*header).vtable = vtable;
    }
}

struct FreeBlock {
    next: * mut BlockHeader,
}

impl<const SIZE: usize> LPAllocator<SIZE> {
    pub const fn new() -> Self { unsafe {
        let mut ret = LPAllocator { allocated: 0, free_ptr: UnsafeCell::new(null_mut()), heap : [MaybeUninit::uninit(); SIZE] };
        let bh : * mut BlockHeader = ret.heap.as_mut_ptr().cast();
        BlockHeader::init_header_value(bh, SIZE, null_mut(), null_mut(), null_mut());
        let fb : * mut FreeBlock = ret.heap.as_mut_ptr().byte_add(core::mem::size_of::<BlockHeader>()) as * mut FreeBlock;
        (*fb).next = null_mut();
        ret
    }}

    pub fn init(&self) {
        unsafe{self.free_ptr.get().write(self.heap.as_ptr() as * mut BlockHeader);}
    }
}

unsafe impl<const SIZE: usize> GlobalAlloc for LPAllocator<SIZE> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 { unsafe{
        let layout = layout.align_to(core::mem::align_of::<FreeBlock>()).unwrap().pad_to_align();
        let total_size = layout.size() + core::mem::size_of::<BlockHeader>();
        let mut current_ptr : *mut *mut BlockHeader = &self.free_ptr as * const _ as * mut * mut BlockHeader;
        let mut current = *current_ptr ;
        while !current.is_null() {
            let fb = BlockHeader::get_value::<FreeBlock>(current);
            if (*current).size < total_size {
                current_ptr = &(*fb).next as * const _ as * mut * mut BlockHeader;
                current = (*fb).next;
                continue;
            }
            // found a block
            let remaining = (*current).size - total_size;
            if remaining > core::mem::size_of::<BlockHeader>() + core::mem::size_of::<FreeBlock>() {
                // Split the block
                let new_block = (current as usize + total_size) as * mut BlockHeader;
                BlockHeader::init_header_value(new_block, remaining, null_mut(), current, (*current).next);
                if !(*current).next.is_null() {
                    (*(*current).next).prev = new_block;
                }
                (*current).next = new_block;
                (*current).size = total_size;
                (*(BlockHeader::get_value::<FreeBlock>(new_block))).next = (*fb).next;
                *current_ptr = new_block;
            } else {
                *current_ptr = (*fb).next;
            }
            return (current as usize + core::mem::size_of::<BlockHeader>()) as * mut u8;
        }
        return null_mut();
    }}

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) { unsafe{
        let header = (ptr as usize - core::mem::size_of::<BlockHeader>()) as * mut BlockHeader;
        // Coalesce with previous block if free
        if !(*header).prev.is_null() {
            let prev_header = (*header).prev;
            if (*prev_header).vtable.is_null() { // check whether the previous block is free.
                // Merge
                (*prev_header).size += (*header).size;
                (*prev_header).next = (*header).next;
                if !(*header).next.is_null() {
                    (*(*header).next).prev = prev_header;
                }
                return;
            }
        }
        (*header).vtable = null_mut(); // mark as free
        let fb = BlockHeader::get_value::<FreeBlock>(header);
        (*(fb)).next = self.free_ptr.get().read();
        self.free_ptr.get().write(header);
    }}

    // We can also override alloc_zeroed and realloc for more detailed logging
    // or rely on their default implementations. For brevity, we'll use defaults.
}

#[alloc_error_handler]
fn ignore_alloc_error(_: core::alloc::Layout) -> ! {
    loop{}
}

unsafe impl<const SIZE: usize> Sync for LPAllocator<SIZE> {}