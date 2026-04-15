use core::{alloc::{GlobalAlloc, Layout}, cell::UnsafeCell, mem::MaybeUninit, ptr::null_mut};

/// An LP allocator implementation using a simple linked list of free blocks.
/// This is used for the LP coprocessor to support dynamic memory allocation.
/// Beacuse we must share information about allocation, this allocator is implemented as a struct, and the global allocator functions are just wrappers around it.
pub struct ImplLPAllocator<const SIZE : usize> {
    pub free_ptr: UnsafeCell<*mut BlockHeader>,
    pub heap : [MaybeUninit<u8>; SIZE]
}

impl<const SIZE : usize> ImplLPAllocator<SIZE> {
    /// Creates a new instance of the allocator. Note that the allocator is not initialized yet, and you must call `init` before using it.
    pub const fn new() -> Self {
        ImplLPAllocator {
            free_ptr: UnsafeCell::new(null_mut()),
            heap: [MaybeUninit::<u8>::uninit(); SIZE]
        }
    }
}
impl<const SIZE : usize> ImplLPAllocator<SIZE> {
    /// Initializes the allocator. This function must be called before using the allocator.
    /// This function sets up the initial free block that covers the entire heap.
    pub fn init(&mut self) { unsafe{
        let head = self.heap.as_mut_ptr() as *mut BlockHeader;
        *(self.free_ptr.get_mut()) = head;
        BlockHeader::init_header_value(head, self.heap.len(), null_mut(), null_mut(), null_mut());
        let fb : * mut FreeBlock = self.heap.as_mut_ptr().byte_add(core::mem::size_of::<BlockHeader>()) as * mut FreeBlock;
        (*fb).next = null_mut();
    }}
}

/// This represents a header of a memory block.
pub struct BlockHeader {
    /// The next block header.
    next: * mut BlockHeader,
    /// The virtual table pointer.
    vtable: * mut u8,
    /// The previous block header.
    prev: * mut BlockHeader,
    /// Whole size including this header.
    size: usize,
}

impl BlockHeader {
    /// Get the start address of the value stored in this block.
    unsafe fn get_value<T>(this : *mut Self) -> * mut T {
        (this as * const BlockHeader as usize + core::mem::size_of::<BlockHeader>()) as * mut T
    }
    /// Initialize the header value. This function is used to set up the header of a memory block.
    const unsafe fn init_header_value(this : *mut Self, size : usize, vtable : *mut u8, prev : * mut BlockHeader, next : * mut BlockHeader) { unsafe {
        (*this).size = size;
        (*this).vtable = vtable;
        (*this).prev = prev;
        (*this).next = next;
    }}
}

/// Write virtual table pointer to the block header of the given pointer.
#[cfg(feature = "unsafe-vtable")]
pub unsafe fn write_vtable(ptr: * mut u8, vtable: * mut u8) {
    let header = (ptr as usize - core::mem::size_of::<BlockHeader>()) as * mut BlockHeader;
    unsafe {
        (*header).vtable = vtable;
    }
}

/// This represents a free block.
/// It is stored after [`BlockHeader`] of a free block.
struct FreeBlock {
    /// The next free block header.
    next: * mut BlockHeader
}

#[cfg(feature = "has-lp-core")]
unsafe extern "Rust" {
    #[link_name = "__lpcoproc_allocator_alloc"]
    pub(crate) fn lp_allocator_alloc(layout: Layout) -> * mut u8;
    
    #[link_name = "__lpcoproc_allocator_dealloc"]
    pub(crate) fn lp_allocator_dealloc(ptr: * mut u8, layout: Layout);

    #[cfg(any(not(any(feature = "esp32c6", feature = "esp32s3")), feature = "custom_range"))]
    #[link_name = "__lpcoproc_allocator_get_lp_mem_begin_and_len"]
    pub(crate) fn get_lp_mem_begin_and_len() -> (usize, usize);
}

#[cfg(feature = "esp32c6")]
const LP_ADDRESS_MAX : usize = LP_ADDRESS_BASE + LP_ADDRESS_LEN;
#[cfg(feature = "esp32c6")]
const LP_ADDRESS_LEN : usize = 0x0004_0000;
#[cfg(feature = "esp32c6")]
const LP_ADDRESS_BASE : usize = 0x5000_0000;

#[cfg(feature = "esp32s3")]
const LP_ADDRESS_MAX : usize = LP_ADDRESS_BASE + LP_ADDRESS_LEN;
#[cfg(feature = "esp32s3")]
const LP_ADDRESS_LEN : usize = 0x0002_0000;
#[cfg(feature = "esp32s3")]
const LP_ADDRESS_BASE : usize = 0x5000_0000;

#[cfg(all(feature = "nottest", any(feature = "esp32c6", feature = "esp32s3"), not(feature = "custom_range")))]
#[inline(always)]
pub(crate) const fn get_lp_mem_begin_and_len() -> (usize, usize) {
    (LP_ADDRESS_BASE, LP_ADDRESS_LEN)
}

#[cfg(all(feature = "esp32s3", feature = "has-lp-core"))]
#[inline(always)]
pub fn address_translate_to_lp<T>(addr : * mut T) -> * mut T where T : ?Sized {
    if in_lp_mem_range(addr) {
        addr.wrapping_byte_sub(LP_ADDRESS_BASE)
    } else {
        addr
    }
}

#[cfg(any(not(feature = "esp32s3"), feature = "is-lp-core"))]
#[inline(always)]
pub const fn address_translate_to_lp<T>(addr : * mut T) -> * mut T where T : ?Sized {
    addr
}

#[cfg(all(feature = "esp32s3", feature = "has-lp-core"))]
#[inline(always)]
pub fn address_translate_to_main<T>(addr : * mut T) -> * mut T where T : ?Sized {
    if in_lp_mem_range_translated(addr) {
        addr.wrapping_byte_add(LP_ADDRESS_BASE)
    } else {
        addr
    }
}
    
#[cfg(any(not(feature = "esp32s3"), feature = "is-lp-core"))]
#[inline(always)]
pub const fn address_translate_to_main<T>(addr : * mut T) -> * mut T where T : ?Sized {
    addr
}


#[cfg(all(feature = "esp32s3", feature = "has-lp-core"))]
#[inline(always)]
pub fn address_translate_to_main_const<T>(addr : * const T) -> * const T where T : ?Sized {
    if in_lp_mem_range_translated(addr) {
        addr.wrapping_byte_add(LP_ADDRESS_BASE)
    } else {
        addr
    }
}
    
#[cfg(any(not(feature = "esp32s3"), feature = "is-lp-core"))]
#[inline(always)]
pub const fn address_translate_to_main_const<T>(addr : * const T) -> * const T where T : ?Sized {
    addr
}

/// Check whether the given address is in the LP memory range.
#[inline(always)]
#[cfg(not(feature = "esp32s3"))]
pub fn in_lp_mem_range<T>(addr : * const T) -> bool where T : ?Sized {
    let addr = addr as * const () as usize;
    #[allow(unused_unsafe)]
    let (base, len) = unsafe { get_lp_mem_begin_and_len() };
    addr.wrapping_sub(base) < len
}

/// Check whether the given address is in the LP memory range.
#[inline(always)]
#[cfg(feature = "esp32s3")]
pub fn in_lp_mem_range<T>(addr : * const T) -> bool where T : ?Sized {
    let addr = addr as * const () as usize;
    #[allow(unused_unsafe)]
    let (base, len) = unsafe { get_lp_mem_begin_and_len() };
    addr.wrapping_sub(base) < len
}

#[cfg(feature = "esp32s3")]
const LP_ADDRESS_TRANSLATED_MAX : usize = LP_ADDRESS_TRANSLATED_BASE + LP_ADDRESS_TRANSLATED_LEN;
#[cfg(feature = "esp32s3")]
const LP_ADDRESS_TRANSLATED_LEN : usize = 0x0001_FF00;
#[cfg(feature = "esp32s3")]
const LP_ADDRESS_TRANSLATED_BASE : usize = 0x100; // For dangling pointers. We believe that the code on LP will be longer than 0x100 and typeof::<T> will be smaller than 0x100.

#[cfg(feature = "esp32s3")]
#[inline(always)]
pub(crate) const fn get_lp_mem_begin_and_len_translated() -> (usize, usize) {
    (LP_ADDRESS_TRANSLATED_BASE, LP_ADDRESS_TRANSLATED_LEN)
}
#[cfg(feature = "esp32s3")]
#[inline(always)]
fn in_lp_mem_range_translated<T>(addr : * const T) -> bool where T : ?Sized {
    let addr = addr as * const () as usize;
    #[allow(unused_unsafe)]
    let (base, len) = unsafe { get_lp_mem_begin_and_len_translated() };
    addr.wrapping_sub(base) < len
}

#[cfg(not(feature = "nottest"))]
use std::cell::RefCell;

#[cfg(not(feature = "nottest"))]
thread_local! {
    /// This is a simulator of the global allocator for the LP coprocessor. It is used for testing purposes when the actual LP coprocessor is not available.
    static GLOBAL_LP_ALLOCATOR : RefCell<ImplLPAllocator<4096>> = RefCell::new(ImplLPAllocator::new());
}
#[cfg(not(feature = "nottest"))]
pub fn lp_allocator_init() {
    GLOBAL_LP_ALLOCATOR.with_borrow_mut(|a| a.init());
}

#[cfg(not(feature = "nottest"))]
pub(crate) fn lp_allocator_alloc(layout: Layout) -> * mut u8 {
    unsafe { GLOBAL_LP_ALLOCATOR.with_borrow(|a| a.alloc(layout)) }
}
#[cfg(not(feature = "nottest"))]
pub(crate) fn lp_allocator_dealloc(ptr: * mut u8, layout: Layout) {
    unsafe { GLOBAL_LP_ALLOCATOR.with_borrow(|a| a.dealloc(ptr, layout)) }
}
#[cfg(not(feature = "nottest"))]
pub(crate) fn get_lp_mem_begin_and_len() -> (usize, usize) {
    GLOBAL_LP_ALLOCATOR.with_borrow(|a| (a.heap.as_ptr() as usize, a.heap.len() as usize))
}

unsafe impl<const SIZE : usize> GlobalAlloc for ImplLPAllocator<SIZE> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        unsafe {
            let layout = layout.align_to(core::mem::align_of::<FreeBlock>()).unwrap().pad_to_align();
            let total_size = layout.size() + core::mem::size_of::<BlockHeader>();
            let mut current_ptr: *mut *mut BlockHeader =  self.free_ptr.get();
            let mut current = address_translate_to_main(*current_ptr);
            while !current.is_null() {
                let fb = BlockHeader::get_value::<FreeBlock>(current);
                if (*current).size < total_size {
                    current_ptr = &(*fb).next as * const _ as * mut * mut BlockHeader;
                    current = address_translate_to_main((*fb).next);
                    continue;
                }
                // found a block
                let remaining = (*current).size - total_size;
                if remaining > core::mem::size_of::<BlockHeader>() + core::mem::size_of::<FreeBlock>() {
                    // Split the block
                    let new_block = (current as usize + total_size) as * mut BlockHeader;
                    BlockHeader::init_header_value(new_block, remaining, 1 as * mut u8, current, (*current).next);
                    if !(*current).next.is_null() {
                        (*address_translate_to_main((*current).next)).prev = address_translate_to_lp(new_block);
                    }
                    (*current).next = address_translate_to_lp(new_block);
                    (*current).size = total_size;
                    (*(BlockHeader::get_value::<FreeBlock>(new_block))).next = (*fb).next;
                    *current_ptr = address_translate_to_lp(new_block);
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
                let prev_header = address_translate_to_main((*header).prev);
                if (*prev_header).vtable.is_null() { // check whether the previous block is free.
                    // Merge
                    (*prev_header).size += (*header).size;
                    (*prev_header).next = (*header).next;
                    if !(*header).next.is_null() {
                        (*address_translate_to_main((*header).next)).prev = (*header).prev;
                    }
                    return;
                }
            }
            (*header).vtable = null_mut(); // mark as free
            let fb = BlockHeader::get_value::<FreeBlock>(header);
            (*(fb)).next = self.free_ptr.get().read();
            self.free_ptr.get().write(address_translate_to_lp(header));
        }
    }
}