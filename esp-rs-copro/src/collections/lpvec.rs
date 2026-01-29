/// LICENSE Information
/// Many parts of this file comes from Rust Project, licensed under Apache License 2.0 or MIT License.
/// Copyright (c) The Rust Project Contributors.
/// https://github.com/rust-lang/rust

use core::{alloc::Layout, slice, fmt, intrinsics, iter, marker::PhantomData, mem::{self, ManuallyDrop, MaybeUninit, SizedTypeProperties}, ops::{Index, IndexMut, Range, RangeBounds}, ptr::{self, NonNull, Unique}, slice::SliceIndex};
use crate::{lpbox::LPBox, movableobject::MovableObject};

#[cfg(feature = "nottest")]
use ::alloc::{alloc, boxed::Box};
#[cfg(not(feature = "nottest"))]
use std::{alloc, boxed::Box};

type Cap = core::num::niche_types::UsizeNoHighBit;

pub struct LPVec<T : MovableObject> {
    vec_inner : LPVecInner,
    len : usize,
    _marker : PhantomData<T>
}

struct LPVecInner {
    ptr : Unique<u8>,
    capacity : Cap
}

#[derive(Debug)]
pub enum LPTryReserveError {
    CapacityOverflow,
    AllocError,
}

impl LPVecInner {
    const fn new() -> Self {
        LPVecInner {
            ptr : Unique::dangling(),
            capacity : Cap::new(0).unwrap()
        }
    }
    fn try_with_capacity(capacity : usize, elem_layout : Layout) -> Option<Self> {
        let mut ret = LPVecInner::new();
        ret.grow_or_shrink(capacity, elem_layout).ok().map(|_| ret)
    }
    fn with_capacity_zerod(capacity : usize, elem_layout : Layout) -> Option<Self> {
        #[allow(unused_mut)] // acctually mutated in the next unsafe line!
        let mut ret = LPVecInner::try_with_capacity(capacity, elem_layout)?;
        unsafe {
            core::ptr::write_bytes(ret.ptr.as_ptr(), 0, ret.current_memory(elem_layout).size());
        }
        Some(ret)
    }

    const fn set_ptr_and_cap(&mut self, ptr : Unique<u8>, cap : usize) {
        self.ptr = ptr;
        self.capacity = Cap::new(cap).unwrap();
    }

    const fn capacity(&self) -> usize {
        self.capacity.as_inner()
    }
    unsafe fn current_memory(&self, elem_layout: Layout) -> Layout {
        unsafe {
            let alloc_size = elem_layout.size().unchecked_mul(self.capacity.as_inner());
            let layout = Layout::from_size_align_unchecked(alloc_size, elem_layout.align());
            layout
        }
    }
    fn layout_array(cap : usize, elem_layout : Layout) -> Option<Layout> {
        unsafe {
            let alloc_size = elem_layout.size().checked_mul(cap)?;
            let layout = Layout::from_size_align_unchecked(alloc_size, elem_layout.align());
            Some(layout)
        }
    }
    pub(crate) fn grow_or_shrink(&mut self, new_elem_count : usize, elem_layout : Layout) -> Result<(), LPTryReserveError> {
        if new_elem_count == 0 || elem_layout.size() == 0 {
            if !self.ptr.as_ptr().is_null() {
                self.deallocate(unsafe { self.current_memory(elem_layout) });
            }
            self.set_ptr_and_cap(Unique::dangling(), 0);
            return Ok(());
        }
        if let Some(new_layout) = Self::layout_array(new_elem_count, elem_layout) {
            let new_ptr = unsafe { 
                if self.ptr.as_ptr().is_null() {
                    alloc::alloc(new_layout)
                } else {
                    alloc::realloc(self.ptr.as_ptr(), self.current_memory(elem_layout), new_layout.size())
                }
            };
            if new_ptr.is_null() {
                Err(LPTryReserveError::AllocError)
            } else {
                self.set_ptr_and_cap(unsafe { Unique::new_unchecked(new_ptr) }, new_elem_count);
                Ok(())
            }
        } else {
            Err(LPTryReserveError::CapacityOverflow)
        }
    }

    fn deallocate(&mut self, elem_layout : Layout) {
        #[cfg(feature = "has-lp-core")]
        crate::lpbox::lp_dealloc(self.ptr.as_ptr(), unsafe{self.current_memory(elem_layout)});
        #[cfg(any(feature = "is-lp-core", not(feature = "nottest")))]
        unsafe { alloc::dealloc(self.ptr.as_ptr(), self.current_memory(elem_layout)); }
    }

    const unsafe fn from_raw_parts(ptr : * mut u8, capacity : usize) -> Self {
        LPVecInner {
            ptr : unsafe { Unique::new_unchecked(ptr) },
            capacity : Cap::new(capacity).unwrap()
        }
    }
    const unsafe fn as_mut_ptr(&mut self) -> * mut u8 {
        self.ptr.as_ptr()
    }
    const unsafe fn as_ptr(&self) -> * const u8 {
        self.ptr.as_ptr()
    }

    unsafe fn into_mut_ptr<T : MovableObject>(self) -> * mut T{
        let mut me = ManuallyDrop::new(self);
        unsafe{ me.as_mut_ptr() as *mut T }
    }
}

impl<T : MovableObject> LPVec<T> {
    #[inline]
    pub const fn new() -> Self {
        LPVec {
            vec_inner : LPVecInner::new(),
            len : 0,
            _marker : PhantomData
        }
    }

    pub fn try_with_capacity(capacity : usize) -> Option<Self> {
        let vec_inner = LPVecInner::try_with_capacity(capacity, T::LAYOUT)?;
        Some(LPVec {
            vec_inner,
            len : 0,
            _marker : PhantomData
        })
    }

    #[inline]
    pub const unsafe fn from_raw_parts(ptr : * mut T, len : usize, capacity : usize) -> Self {
        LPVec {
            vec_inner : unsafe { LPVecInner::from_raw_parts(ptr as * mut u8, capacity) },
            len : len,
            _marker : PhantomData
        }
    }

    #[inline]
    pub unsafe fn from_parts(ptr : NonNull<T>, len : usize, capacity : usize) -> Self {
        LPVec {
            vec_inner : unsafe { LPVecInner::from_raw_parts(ptr.as_ptr() as * mut u8, capacity) },
            len : len,
            _marker : PhantomData
        }
    }

    #[must_use = "losing the pointer will leak memory"]
    pub fn into_raw_parts(self) -> (*mut T, usize, usize) {
        let mut me = ManuallyDrop::new(self);
        (me.as_mut_ptr(), me.len(), me.capacity())
    }
    #[must_use = "losing the pointer will leak memory"]
    pub fn into_parts(self) -> (NonNull<T>, usize, usize) {
        let (ptr, len, capacity) = self.into_raw_parts();
        (unsafe { NonNull::new_unchecked(ptr) }, len, capacity)
    }

    #[rustc_never_returns_null_ptr]
    #[rustc_as_ptr]
    #[inline]
    pub const fn as_mut_ptr(&mut self) -> *mut T {
        unsafe { self.vec_inner.as_mut_ptr() as * mut T }
    }
    #[rustc_never_returns_null_ptr]
    #[rustc_as_ptr]
    #[inline]
    pub const fn as_ptr(&self) -> *const T {
        unsafe { self.vec_inner.as_ptr() as * const T }
    }

    #[inline]
    pub const fn capacity(&self) -> usize {
        if T::IS_ZST { usize::MAX } else { self.vec_inner.capacity() }
    }

    #[inline]
    const fn needs_to_grow(&self, additional : usize) -> bool {
        self.len() + additional > self.capacity()
    }

    pub fn reserve_exact(&mut self, additional : usize) {
        let _ = self.try_reserve_exact(additional);
    }
    pub fn reserve(&mut self, additional : usize) {
        let _ = self.try_reserve(additional);
    }

    pub fn try_reserve_exact(&mut self, additional : usize) -> Result<(), LPTryReserveError> {
        self.vec_inner.grow_or_shrink(additional + self.len(), T::LAYOUT)
    }
    pub fn try_reserve(&mut self, additional : usize) -> Result<(), LPTryReserveError>{
        if additional + self.len() > self.capacity() {
            self.try_reserve_exact(additional)
        } else {
            Ok(())
        }
    }

    #[inline]
    pub fn shrink_to_fit(&mut self) {
        self.shrink_to(self.len())
    }

    #[inline]
    pub fn shrink_to(&mut self, min_capacity : usize) {
        if min_capacity < self.capacity() {
            let new_capacity = core::cmp::max(self.len(), min_capacity);
            let _ = self.vec_inner.grow_or_shrink(new_capacity, T::LAYOUT);
        }
    }

    pub fn try_remove(&mut self, index: usize) -> Option<T> {
        let len = self.len();
        if index >= len {
            return None;
        }
        unsafe {
            let ret;
            {
                let ptr = self.as_mut_ptr().add(index);
                ret = ptr::read(ptr);
                ptr::copy(ptr.add(1), ptr, len - index - 1);
            }
            self.set_len(len - 1);
            Some(ret)
        }
    }

    #[track_caller]
    #[rustc_confusables("delete", "take")]
    pub fn remove(&mut self, index: usize) -> T {
        #[cold]
        #[cfg_attr(not(panic = "immediate-abort"), inline(never))]
        #[track_caller]
        fn assert_failed(index: usize, len: usize) -> ! {
            panic!("removal index (is {index}) should be < len (is {len})");
        }

        match self.try_remove(index) {
            Some(elem) => elem,
            None => assert_failed(index, self.len()),
        }
    }

    const LEAST_CAPACITY : usize = if size_of::<T>() == 1 { 8 } else { if size_of::<T>() <= 1024 { 4 } else { 1 } };

    fn grow(&mut self) -> Result<(), LPTryReserveError> {
        let old_capacity = self.capacity();
        let mut last_err : Result<(), LPTryReserveError> = Ok(());
        for i in 1..32 {
            let new_capacity = Self::LEAST_CAPACITY.max(old_capacity + (old_capacity >> i));
            if new_capacity == old_capacity { break; }
            match self.vec_inner.grow_or_shrink(new_capacity, T::LAYOUT) {
                Ok(()) => break,
                v => last_err = v
            } 
        }
        return last_err;
    }

    fn write_at(&mut self, index : usize, value : T) -> &mut T {
        unsafe {
            let ptr = self.as_mut_ptr().add(index);
            ptr::write(ptr, value);
            &mut *ptr
        }
    }

    #[must_use = "if you don't need a reference to the value, use `Vec::push` instead"]
    pub fn push_mut(&mut self, value : T) -> &mut T{
        if self.needs_to_grow(1) {
            self.grow().unwrap();
        }
        let len = self.len;
        self.len = len + 1;
        self.write_at(len, value)
    }

    pub fn push(&mut self, value : T){
        let _ = self.push_mut(value);
    }

    pub fn push_mut_within_capacity(&mut self, value: T) -> Result<&mut T, T> {
        if self.len == self.vec_inner.capacity() {
            Err(value)
        } else {
            let len = self.len;
            self.len = len + 1;
            Ok(self.write_at(len, value))
        }
    }

    pub fn push_within_capacity(&mut self, value: T) -> Result<(), T> {
        self.push_mut_within_capacity(value).map(|_| ())
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.len() == 0 {
            None
        } else {
            self.len -= 1;
            unsafe{Some(ptr::read(self.as_ptr().add(self.len())))}
        }
    }

    pub fn into_boxed_slice(mut self) -> LPBox<[T]> {
        unsafe {
            self.shrink_to_fit();
            let me = ManuallyDrop::new(self);
            let buf = ptr::read(&me.vec_inner);
            let len = me.len();
            let ptr = buf.into_mut_ptr::<T>();
            LPBox::from_raw(ptr::slice_from_raw_parts_mut(ptr, len))
        }
    }

    pub fn truncate(&mut self, len: usize) {
        if len > self.len { // Not ">=" (cf. Rust#78884 )
            return;
        }
        unsafe {
            let remaining_len = self.len - len;
            let s = ptr::slice_from_raw_parts_mut(self.as_mut_ptr().add(len), remaining_len);
            self.set_len(len);
            ptr::drop_in_place(s);
        }
    }

    #[inline]
    pub fn swap_remove(&mut self, index: usize) -> T {
        #[cold]
        fn assert_failed(index: usize, len: usize) -> ! {
            panic!("swap_remove index (is {index}) should be < len (is {len})");
        }

        let len = self.len();
        if index >= len {
            assert_failed(index, len);
        }
        unsafe {
            // We replace self[index] with the last element. Note that if the
            // bounds check above succeeds there must be a last element (which
            // can be self[index] itself).
            let value = ptr::read(self.as_ptr().add(index));
            let base_ptr = self.as_mut_ptr();
            ptr::copy(base_ptr.add(len - 1), base_ptr.add(index), 1);
            self.set_len(len - 1);
            value
        }
    }

    #[track_caller]
    pub fn insert(&mut self, index: usize, element: T) {
        let _ = self.insert_mut(index, element);
    }

    #[inline]
    #[track_caller]
    #[must_use = "if you don't need a reference to the value, use `Vec::insert` instead"]
    pub fn insert_mut(&mut self, index: usize, element: T) -> &mut T {
        #[cold]
        #[track_caller]
        fn assert_failed(index: usize, len: usize) -> ! {
            panic!("insertion index (is {index}) should be <= len (is {len})");
        }

        let len = self.len();
        if index > len {
            assert_failed(index, len);
        }

        // space for the new element
        if self.needs_to_grow(1) {
            self.grow().unwrap();
        }

        unsafe {
            let p = self.as_mut_ptr().add(index);
            {
                if index < len {
                    ptr::copy(p, p.add(1), len - index);
                }
                ptr::write(p, element);
            }
            self.set_len(len + 1);
            &mut *p
        }
    }

    
    pub fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(&T) -> bool,
    {
        self.retain_mut(|elem| f(elem));
    }

    pub fn retain_mut<F>(&mut self, mut f: F)
    where
        F: FnMut(&mut T) -> bool,
    {
        let original_len = self.len();
        if original_len == 0 { return; }
        unsafe { self.set_len(0) };

        struct BackshiftOnDrop<'a, T> where T : MovableObject{
            v: &'a mut LPVec<T>,
            processed_len: usize,
            deleted_cnt: usize,
            original_len: usize,
        }

        impl<T : MovableObject> Drop for BackshiftOnDrop<'_, T> {
            fn drop(&mut self) {
                unsafe {
                    if self.deleted_cnt > 0 {
                        ptr::copy(
                            self.v.as_ptr().add(self.processed_len),
                            self.v.as_mut_ptr().add(self.processed_len - self.deleted_cnt),
                            self.original_len - self.processed_len,
                        );
                    }
                    self.v.set_len(self.original_len - self.deleted_cnt);
                }
            }
        }

        let mut g = BackshiftOnDrop { v: self, processed_len: 0, deleted_cnt: 0, original_len };

        fn process_loop<F, T : MovableObject, const DELETED: bool>(
            original_len: usize,
            f: &mut F,
            g: &mut BackshiftOnDrop<'_, T>,
        ) where
            F: FnMut(&mut T) -> bool,
        {
            while g.processed_len != original_len {
                let cur = unsafe { &mut *g.v.as_mut_ptr().add(g.processed_len) };
                if !f(cur) {
                    g.processed_len += 1;
                    g.deleted_cnt += 1;
                    unsafe { ptr::drop_in_place(cur) };
                    if DELETED {
                        continue;
                    } else {
                        break;
                    }
                }
                if DELETED {
                    unsafe {
                        let hole_slot = g.v.as_mut_ptr().add(g.processed_len - g.deleted_cnt);
                        ptr::copy_nonoverlapping(cur, hole_slot, 1);
                    }
                }
                g.processed_len += 1;
            }
        }

        process_loop::<F, T, false>(original_len, &mut f, &mut g);
        process_loop::<F, T, true>(original_len, &mut f, &mut g);
        drop(g);
    }

    #[inline]
    pub fn dedup_by_key<F, K>(&mut self, mut key: F)
    where
        F: FnMut(&mut T) -> K,
        K: PartialEq,
    {
        self.dedup_by(|a, b| key(a) == key(b))
    }

    pub fn dedup_by<F>(&mut self, mut same_bucket: F)
    where
        F: FnMut(&mut T, &mut T) -> bool,
    {
        let len = self.len();
        if len <= 1 { return; }

        let mut first_duplicate_idx: usize = 1;
        let start = self.as_mut_ptr();
        while first_duplicate_idx != len {
            let found_duplicate = unsafe {
                let current = start.add(first_duplicate_idx);
                let prev = start.add(first_duplicate_idx - 1);
                same_bucket(&mut *current, &mut *prev)
            };
            if found_duplicate {
                break;
            }
            first_duplicate_idx += 1;
        }
        if first_duplicate_idx == len {
            return;
        }

        struct FillGapOnDrop<'a, T : MovableObject> {
            read: usize,
            write: usize,
            vec: &'a mut LPVec<T>,
        }

        impl<'a, T : MovableObject> Drop for FillGapOnDrop<'a, T> {
            fn drop(&mut self) {
                unsafe {
                    let ptr = self.vec.as_mut_ptr();
                    let len = self.vec.len();
                    let items_left = len - self.read;
                    let dropped_ptr = ptr.add(self.write);
                    let valid_ptr = ptr.add(self.read);
                    ptr::copy(valid_ptr, dropped_ptr, items_left);
                    let dropped = self.read - self.write;
                    self.vec.set_len(len - dropped);
                }
            }
        }

        let mut gap =
            FillGapOnDrop { read: first_duplicate_idx + 1, write: first_duplicate_idx, vec: self };
        unsafe {
            ptr::drop_in_place(start.add(first_duplicate_idx));

            while gap.read < len {
                let read_ptr = start.add(gap.read);
                let prev_ptr = start.add(gap.write - 1);
                if same_bucket(&mut *read_ptr, &mut *prev_ptr) {
                    gap.read += 1;
                    ptr::drop_in_place(read_ptr);
                } else {
                    let write_ptr = start.add(gap.write);
                    ptr::copy_nonoverlapping(read_ptr, write_ptr, 1);
                    gap.write += 1;
                    gap.read += 1;
                }
            }

            gap.vec.set_len(gap.write);
            mem::forget(gap);
        }
    }

    pub fn pop_if(&mut self, predicate: impl FnOnce(&mut T) -> bool) -> Option<T> {
        let last = self.as_mut_slice().last_mut()?;
        if predicate(last) { self.pop() } else { None }
    }

    #[inline]
    pub fn append(&mut self, other: &mut Self) {
        unsafe {
            self.append_elements(other.as_slice() as _);
            other.set_len(0);
        }
    }

    #[inline]
    unsafe fn append_elements(&mut self, other: *const [T]) {
        let count = other.len();
        self.try_reserve(count).unwrap();
        let len = self.len();
        unsafe { ptr::copy_nonoverlapping(other as *const T, self.as_mut_ptr().add(len), count) };
        self.len += count;
    }
    
    // pub fn drain<R>(&mut self, range: R) -> Drain<'_, T, A>
    // where
    //     R: RangeBounds<usize>,
    // {
    //     let len = self.len();
    //     let Range { start, end } = slice::range(range, ..len);

    //     unsafe {
    //         // set self.vec length's to start, to be safe in case Drain is leaked
    //         self.set_len(start);
    //         let range_slice = slice::from_raw_parts(self.as_ptr().add(start), end - start);
    //         Drain {
    //             tail_start: end,
    //             tail_len: len - end,
    //             iter: range_slice.iter(),
    //             vec: NonNull::from(self),
    //         }
    //     }
    // }

    #[inline]
    pub fn clear(&mut self) {
        let elems: *mut [T] = self.as_mut_slice();
        unsafe {
            self.len = 0;
            ptr::drop_in_place(elems);
        }
    }

    #[inline]
    #[must_use = "use `.truncate()` if you don't need the other half"]
    #[track_caller]
    pub fn split_off(&mut self, at: usize) -> Self
    {
        #[cold]
        #[track_caller]
        fn assert_failed(at: usize, len: usize) -> ! {
            panic!("`at` split index (is {at}) should be <= len (is {len})");
        }

        if at > self.len() {
            assert_failed(at, self.len());
        }

        let other_len = self.len - at;
        let mut other = LPVec::<T>::try_with_capacity(other_len).unwrap();

        // Unsafely `set_len` and copy items to `other`.
        unsafe {
            self.set_len(at);
            other.set_len(other_len);

            ptr::copy_nonoverlapping(self.as_ptr().add(at), other.as_mut_ptr(), other.len());
        }
        other
    }

    pub fn resize_with<F>(&mut self, new_len: usize, f: F)
    where
        F: FnMut() -> T,
    {
        let len = self.len();
        if new_len > len {
            self.extend_trusted(iter::repeat_with(f).take(new_len - len));
        } else {
            self.truncate(new_len);
        }
    }

    #[inline]
    pub fn spare_capacity_mut(&mut self) -> &mut [MaybeUninit<T>] {
        unsafe {
            slice::from_raw_parts_mut(
                self.as_mut_ptr().add(self.len) as *mut MaybeUninit<T>,
                self.vec_inner.capacity() - self.len,
            )
        }
    }

    #[inline]
    pub fn split_at_spare_mut(&mut self) -> (&mut [T], &mut [MaybeUninit<T>]) {
        let (init, spare, _) = unsafe { self.split_at_spare_mut_with_len() };
        (init, spare)
    }

    unsafe fn split_at_spare_mut_with_len(
        &mut self,
    ) -> (&mut [T], &mut [MaybeUninit<T>], &mut usize) {
        let ptr = self.as_mut_ptr();
        let spare_ptr = unsafe { ptr.add(self.len) };
        let spare_ptr = spare_ptr.cast_uninit();
        let spare_len = self.vec_inner.capacity() - self.len;

        unsafe {
            let initialized = slice::from_raw_parts_mut(ptr, self.len);
            let spare = slice::from_raw_parts_mut(spare_ptr, spare_len);

            (initialized, spare, &mut self.len)
        }
    }

    #[inline]
    pub const fn as_slice(&self) -> &[T] {
        unsafe { slice::from_raw_parts(self.as_ptr(), self.len) }
    }

    #[inline]
    pub const fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe { slice::from_raw_parts_mut(self.as_mut_ptr(), self.len) }
    }

    #[inline]
    pub unsafe fn set_len(&mut self, new_len: usize) {
        self.len = new_len;
    }

    pub const fn len(&self) -> usize {
        unsafe { intrinsics::assume(self.len <= T::MAX_SLICE_LEN) };
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    
    #[inline]
    pub fn leak<'a>(self) -> &'a mut [T]
    {
        let mut me = ManuallyDrop::new(self);
        unsafe { slice::from_raw_parts_mut(me.as_mut_ptr(), me.len) }
    }

    // leaf method to which various SpecFrom/SpecExtend implementations delegate when
    // they have no further optimizations to apply
    fn extend_desugared<I: Iterator<Item = T>>(&mut self, mut iterator: I) {
        while let Some(element) = iterator.next() {
            if self.needs_to_grow(1) {
                let (lower, _) = iterator.size_hint();
                self.reserve(lower.saturating_add(1));
            }
            unsafe {
                let len = self.len();
                ptr::write(self.as_mut_ptr().add(len), element);
                self.set_len(len + 1);
            }
        }
    }

    fn extend_trusted(&mut self, iterator: impl core::iter::TrustedLen<Item = T>) {
        let (low, high) = iterator.size_hint();
        if let Some(additional) = high {
            debug_assert_eq!(
                low,
                additional,
                "TrustedLen iterator's size hint is not exact: {:?}",
                (low, high)
            );
            self.reserve(additional);
            unsafe {
                let ptr = self.as_mut_ptr();
                let mut local_len = SetLenOnDrop::new(&mut self.len);
                iterator.for_each(move |element| {
                    ptr::write(ptr.add(local_len.current_len()), element);
                    local_len.increment_len(1);
                });
            }
        } else {
            panic!("capacity overflow");
        }
    }

    // #[inline]
    // pub fn splice<R, I>(&mut self, range: R, replace_with: I) -> Splice<'_, I::IntoIter>
    // where
    //     R: RangeBounds<usize>,
    //     I: IntoIterator<Item = T>,
    // {
    //     Splice { drain: self.drain(range), replace_with: replace_with.into_iter() }
    // }

    // pub fn extract_if<F, R>(&mut self, range: R, filter: F) -> ExtractIf<'_, T, F, A>
    // where
    //     F: FnMut(&mut T) -> bool,
    //     R: RangeBounds<usize>,
    // {
    //     ExtractIf::new(self, filter, range)
    // }
}

impl<T: Clone + MovableObject> LPVec<T> {
    pub fn resize(&mut self, new_len: usize, value: T) {
        let len = self.len();

        if new_len > len {
            self.extend_with(new_len - len, value)
        } else {
            self.truncate(new_len);
        }
    }

    pub fn extend_from_slice(&mut self, other: &[T]) {
        self.spec_extend(other.iter())
    }

    pub fn extend_from_within<R>(&mut self, src: R)
    where
        R: RangeBounds<usize>,
    {
        let range = slice::range(src, ..self.len());
        self.reserve(range.len());

        unsafe {
            self.spec_extend_from_within(range);
        }
    }

    fn extend_with(&mut self, n: usize, value: T) {
        self.reserve(n);

        unsafe {
            let mut ptr = self.as_mut_ptr().add(self.len());
            // Use SetLenOnDrop to work around bug where compiler
            // might not realize the store through `ptr` through self.set_len()
            // don't alias.
            let mut local_len = SetLenOnDrop::new(&mut self.len);

            // Write all elements except the last one
            for _ in 1..n {
                ptr::write(ptr, value.clone());
                ptr = ptr.add(1);
                // Increment the length in every step in case clone() panics
                local_len.increment_len(1);
            }

            if n > 0 {
                // We can write the last element directly without cloning needlessly
                ptr::write(ptr, value);
                local_len.increment_len(1);
            }

            // len set by scope guard
        }
    }
}

trait ExtendFromWithinSpec {
    unsafe fn spec_extend_from_within(&mut self, src: Range<usize>);
}

impl<T: Clone + MovableObject> ExtendFromWithinSpec for LPVec<T> {
    default unsafe fn spec_extend_from_within(&mut self, src: Range<usize>) {
        let (this, spare, len) = unsafe { self.split_at_spare_mut_with_len() };
        let to_clone = unsafe { this.get_unchecked(src) };

        iter::zip(to_clone, spare)
            .map(|(src, dst)| dst.write(src.clone()))
            .for_each(|_| *len += 1);
    }
}

impl<T: Copy + MovableObject> ExtendFromWithinSpec for LPVec<T> {
    unsafe fn spec_extend_from_within(&mut self, src: Range<usize>) {
        let count = src.len();
        {
            let (init, spare) = self.split_at_spare_mut();
            let source = unsafe { init.get_unchecked(src) };
            unsafe { ptr::copy_nonoverlapping(source.as_ptr(), spare.as_mut_ptr() as _, count) };
        }
        self.len += count;
    }
}

// Specialization trait used for Vec::extend
pub(super) trait SpecExtend<T, I> {
    fn spec_extend(&mut self, iter: I);
}

impl<T : MovableObject, I> SpecExtend<T, I> for LPVec<T>
where
    I: Iterator<Item = T>,
{
    default fn spec_extend(&mut self, iter: I) {
        self.extend_desugared(iter)
    }
}

impl<T: MovableObject, I> SpecExtend<T, I> for LPVec<T>
where
    I: iter::TrustedLen<Item = T>,
{
    default fn spec_extend(&mut self, iterator: I) {
        self.extend_trusted(iterator)
    }
}

// impl<T: MovableObject> SpecExtend<T, IntoIter<T>> for LPVec<T> {
//     fn spec_extend(&mut self, mut iterator: IntoIter<T>) {
//         unsafe {
//             self.append_elements(iterator.as_slice() as _);
//         }
//         iterator.forget_remaining_elements();
//     }
// }

impl<'a, T: 'a, I> SpecExtend<&'a T, I> for LPVec<T>
where
    I: Iterator<Item = &'a T>,
    T: Clone + MovableObject,
{
    default fn spec_extend(&mut self, iterator: I) {
        self.spec_extend(iterator.cloned())
    }
}

impl<'a, T: 'a> SpecExtend<&'a T, slice::Iter<'a, T>> for LPVec<T>
where
    T: Copy + MovableObject,
{
    fn spec_extend(&mut self, iterator: slice::Iter<'a, T>) {
        let slice = iterator.as_slice();
        unsafe { self.append_elements(slice) };
    }
}

impl<T: PartialEq + MovableObject> LPVec<T> {
    #[inline]
    pub fn dedup(&mut self) {
        self.dedup_by(|a, b| a == b)
    }
}

impl<T : MovableObject, const N: usize> LPVec<[T; N]> {
    pub fn into_flattened(self) -> LPVec<T> {
        let (ptr, len, cap) = self.into_raw_parts();
        let (new_len, new_cap) = if T::IS_ZST {
            (len.checked_mul(N).expect("vec len overflow"), usize::MAX)
        } else {
            unsafe { (len.unchecked_mul(N), cap.unchecked_mul(N)) }
        };
        unsafe { LPVec::<T>::from_raw_parts(ptr.cast(), new_len, new_cap) }
    }
}

impl<T : MovableObject> MovableObject for LPVec<T> {
    #[cfg(feature = "has-lp-core")]
    unsafe fn move_to_main(&self, dest : *mut u8) {
        let dest = dest as * mut Self;
        let dst_ptr = crate::lpbox::LPBox::<[T]>::write_to_main(self.as_slice());
        unsafe {
            dest.write_volatile(LPVec {
                vec_inner : LPVecInner::from_raw_parts(dst_ptr as * mut u8, self.capacity()),
                len : self.len(),
                _marker : PhantomData
            });
        }
    }
    #[cfg(not(feature = "has-lp-core"))]
    unsafe fn move_to_main(&self, _dest : *mut u8) {
        unimplemented!()
    }

    #[cfg(feature = "has-lp-core")]
    unsafe fn move_to_lp(&self, dest : *mut u8) {
        let dest = dest as * mut Self;
        let dst_ptr = unsafe {
            let src = self.as_slice();
            let (mut addr, lay) =
                crate::lpbox::lpbox_static::ADDRESS_TRANSLATION_TABLE.borrow_mut()
                    .remove_by_lp(src as * const [T] as * const () as usize)
                    .map_or_else(|| {
                            let lay = core::alloc::Layout::for_value(src);
                            (crate::lpbox::lpbox_alloc(lay) as usize, lay)
                        },
                        |a| a);
            // Check the layout is unmodified
            if lay != core::alloc::Layout::for_value(src) {
                // extend the main's.
                addr = crate::lpbox::lpbox_realloc(addr as * mut u8, lay, core::alloc::Layout::for_value(src).size()) as usize;
            }
            src.move_to_main(addr as * mut u8);
            addr as * mut u8
        };
        unsafe {
            dest.write_volatile(LPVec {
                vec_inner : LPVecInner::from_raw_parts(dst_ptr as * mut u8, self.capacity()),
                len : self.len(),
                _marker : PhantomData
            });
        }
    }
    #[cfg(not(feature = "has-lp-core"))]
    unsafe fn move_to_lp(&self, _dest : *mut u8) {
        unimplemented!()
    }
}

impl<T : MovableObject> Drop for LPVec<T> {
    fn drop(&mut self) {
        unsafe { ptr::drop_in_place(ptr::slice_from_raw_parts_mut(self.as_mut_ptr(), self.len)); }
        self.vec_inner.deallocate(T::LAYOUT);
    }
}

impl<T : MovableObject> core::ops::Deref for LPVec<T> {
    type Target = [T];

    #[inline]
    fn deref(&self) -> &[T] {
        self.as_slice()
    }
}

impl<T : MovableObject> core::ops::DerefMut for LPVec<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut [T] {
        self.as_mut_slice()
    }
}

// unsafe impl<T> core::ops::DerefPure for LPVec<T> {}

impl<T : MovableObject, const N: usize> TryFrom<LPVec<T>> for [T; N] {
    type Error = LPVec<T>;

    fn try_from(mut vec: LPVec<T>) -> Result<[T; N], LPVec<T>> {
        if vec.len() != N {
            return Err(vec);
        }
        unsafe { vec.set_len(0) };
        let array = unsafe { ptr::read(vec.as_ptr() as *const [T; N]) };
        Ok(array)
    }
}


impl<T : MovableObject> const Default for LPVec<T> {
    fn default() -> LPVec<T> {
        LPVec::new()
    }
}

impl<T: fmt::Debug + MovableObject> fmt::Debug for LPVec<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<T : MovableObject> AsRef<LPVec<T>> for LPVec<T> {
    fn as_ref(&self) -> &LPVec<T> {
        self
    }
}

impl<T : MovableObject> AsMut<LPVec<T>> for LPVec<T> {
    fn as_mut(&mut self) -> &mut LPVec<T> {
        self
    }
}

impl<T : MovableObject> AsRef<[T]> for LPVec<T> {
    fn as_ref(&self) -> &[T] {
        self.as_slice()
    }
}

impl<T : MovableObject> AsMut<[T]> for LPVec<T> {
    fn as_mut(&mut self) -> &mut [T] {
        self.as_mut_slice()
    }
}

impl<T: Clone + MovableObject> From<&[T]> for LPVec<T> {
    fn from(s: &[T]) -> LPVec<T> {
        unsafe {
            let len = s.len();
            let mut v : LPVec<T> = LPVec::try_with_capacity(len).unwrap();
            v.len = len;
            for i in 0..len {
                ptr::write(v.as_mut_ptr().add(i), s.get_unchecked(i).clone());
            }
            v
        }
    }
}

impl<T: Clone + MovableObject> From<&mut [T]> for LPVec<T> {
    fn from(s: &mut [T]) -> LPVec<T> {
        Self::from(s as &[T])
    }
}

impl<T: Clone + MovableObject, const N: usize> From<&[T; N]> for LPVec<T> {
    fn from(s: &[T; N]) -> LPVec<T> {
        Self::from(s.as_slice())
    }
}

impl<T: Clone + MovableObject, const N: usize> From<&mut [T; N]> for LPVec<T> {
    fn from(s: &mut [T; N]) -> LPVec<T> {
        Self::from(s.as_mut_slice())
    }
}

impl<T : MovableObject, const N: usize> From<[T; N]> for LPVec<T> {
    fn from(s: [T; N]) -> LPVec<T> {
        unsafe {
            let mut ret = LPVec::<T>::try_with_capacity(s.len()).unwrap();
            ret.as_mut_ptr().copy_from_nonoverlapping(&s as *const [T; N] as *const T, s.len());
            ret.set_len(s.len());
            ret
        }
    }
}

// impl<'a, T> From<Cow<'a, [T]>> for Vec<T>
// where
//     [T]: ToOwned<Owned = Vec<T>>,
// {
//     fn from(s: Cow<'a, [T]>) -> Vec<T> {
//         s.into_owned()
//     }
// }

impl<T : MovableObject> From<LPBox<[T]>> for LPVec<T> {
    fn from(s: LPBox<[T]>) -> Self {
        unsafe {
            let len = s.len();
            let b = LPBox::into_raw(s);
            LPVec::from_raw_parts(b as *mut T, len, len)
        }
    }
}
impl<T : MovableObject> From<Box<[T]>> for LPVec<T> {
    fn from(s: Box<[T]>) -> Self {
        unsafe {
            let len = s.len();
            let b = Box::into_raw(s);
            LPVec::from_raw_parts(b as *mut T, len, len)
        }
    }
}
impl<T : MovableObject> From<LPVec<T>> for LPBox<[T]> {
    fn from(v: LPVec<T>) -> Self {
        v.into_boxed_slice()
    }
}
impl<T: MovableObject + Clone> Clone for LPVec<T> {
    fn clone(&self) -> Self {
        Self::from(&**self)
    }

    fn clone_from(&mut self, source: &Self) {
        if source.capacity() > self.capacity()  {
            self.reserve(source.capacity() - self.capacity());
            unsafe { self.set_len(source.len()); }
        } else {
            self.truncate(source.len());
        }
        for i in 0..source.len() {
            self[i].clone_from(&source[i]);
        }
    }
}

impl<T: core::hash::Hash + MovableObject> core::hash::Hash for LPVec<T> {
    #[inline]
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        core::hash::Hash::hash(&**self, state)
    }
}

impl<T : MovableObject, I: SliceIndex<[T]>> Index<I> for LPVec<T> {
    type Output = I::Output;

    #[inline]
    fn index(&self, index: I) -> &Self::Output {
        Index::index(&**self, index)
    }
}

impl<T : MovableObject, I: SliceIndex<[T]>> IndexMut<I> for LPVec<T> {
    #[inline]
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        IndexMut::index_mut(&mut **self, index)
    }
}


macro_rules! __impl_slice_eq1 {
    ([$($vars:tt)*] $lhs:ty, $rhs:ty $(where $ty:ty: $bound:ident)?) => {
        impl<T, U : MovableObject, $($vars)*> PartialEq<$rhs> for $lhs
        where
            T: PartialEq<U> + MovableObject, // T and U must implement MovableObject
            $($ty: $bound)?
        {
            #[inline]
            fn eq(&self, other: &$rhs) -> bool { self[..] == other[..] }
            #[inline]
            fn ne(&self, other: &$rhs) -> bool { self[..] != other[..] }
        }
    }
}

__impl_slice_eq1! { [] LPVec<T>, LPVec<U> }
__impl_slice_eq1! { [] LPVec<T>, &[U] }
__impl_slice_eq1! { [] LPVec<T>, &mut [U] }
__impl_slice_eq1! { [] &[T], LPVec<U> }
__impl_slice_eq1! { [] &mut [T], LPVec<U> }
__impl_slice_eq1! { [] LPVec<T>, [U]  }
__impl_slice_eq1! { [] [T], LPVec<U>  }
__impl_slice_eq1! { [const N: usize] LPVec<T>, [U; N] }
__impl_slice_eq1! { [const N: usize] LPVec<T>, &[U; N] }

impl<T: PartialOrd + MovableObject> PartialOrd for LPVec<T>
{
    #[inline]
    fn partial_cmp(&self, other: &LPVec<T>) -> Option<core::cmp::Ordering> {
        PartialOrd::partial_cmp(&**self, &**other)
    }
}

impl<T: Eq + MovableObject> Eq for LPVec<T> {}

impl<T: Ord + MovableObject> Ord for LPVec<T> {
    #[inline]
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        Ord::cmp(&**self, &**other)
    }
}

struct SetLenOnDrop<'a> {
    len: &'a mut usize,
    local_len: usize,
}

impl<'a> SetLenOnDrop<'a> {
    #[inline]
    fn new(len: &'a mut usize) -> Self {
        SetLenOnDrop { local_len: *len, len }
    }

    #[inline]
    fn increment_len(&mut self, increment: usize) {
        self.local_len += increment;
    }

    #[inline]
    fn current_len(&self) -> usize {
        self.local_len
    }
}

impl Drop for SetLenOnDrop<'_> {
    #[inline]
    fn drop(&mut self) {
        *self.len = self.local_len;
    }
}