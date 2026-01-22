use core::{alloc::{GlobalAlloc, Layout}, cell::UnsafeCell, mem::MaybeUninit, ptr::null_mut};


pub struct ImplLPAllocator<const SIZE : usize> {
    pub free_ptr: UnsafeCell<*mut BlockHeader>,
    pub heap : [MaybeUninit<u8>; SIZE]
}

impl<const SIZE : usize> ImplLPAllocator<SIZE> {
    pub const fn new() -> Self {
        ImplLPAllocator {
            free_ptr: UnsafeCell::new(null_mut()),
            heap: [MaybeUninit::<u8>::uninit(); SIZE]
        }
    }
}
impl<const SIZE : usize> ImplLPAllocator<SIZE> {
    pub fn init(&mut self) { unsafe{
        let head = self.heap.as_mut_ptr().cast::<BlockHeader>();
        *(self.free_ptr.get_mut()) = head;
        BlockHeader::init_header_value(head, self.heap.len(), null_mut(), null_mut(), null_mut());
        let fb : * mut FreeBlock = self.heap.as_mut_ptr().byte_add(core::mem::size_of::<BlockHeader>()) as * mut FreeBlock;
        (*fb).next = null_mut();
    }}
}
pub struct BlockHeader {
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

struct FreeBlock { next: * mut BlockHeader }

#[cfg(any(feature = "has-lp-core", test))]
unsafe extern "Rust" {
    #[link_name = "__lpcoproc_allocator_alloc"]
    pub(crate) fn lp_allocator_alloc(layout: Layout) -> * mut u8;

    #[link_name = "__lpcoproc_allocator_dealloc"]
    pub(crate) fn lp_allocator_dealloc(ptr: * mut u8, layout: Layout);
}

unsafe impl<const SIZE : usize> GlobalAlloc for ImplLPAllocator<SIZE> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        unsafe {
            let layout = layout.align_to(core::mem::align_of::<FreeBlock>()).unwrap().pad_to_align();
            let total_size = layout.size() + core::mem::size_of::<BlockHeader>();
            let mut current_ptr: *mut *mut BlockHeader = self.free_ptr.get();
            let mut current = *current_ptr;
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
        }
    }

    unsafe fn dealloc(&self, ptr : *mut u8, _layout: Layout) {
        unsafe {
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
        }
    }
}