/// LICENSE Information
/// Many parts of this file comes from Rust Project, licensed under Apache License 2.0 or MIT License.
/// Copyright (c) The Rust Project Contributors.
/// https://github.com/rust-lang/rust

use core::{slice, fmt, iter, mem::{ManuallyDrop, MaybeUninit}, ops::{Index, IndexMut, Range, RangeBounds}, ptr::{self, NonNull}, slice::SliceIndex};
use crate::{collections::lpvec::{ExtendFromWithinSpec, LPTryReserveError, LPVec, SpecExtend}, lpadapter::{LPAdapter, LPAdapterSliceConvert}, lpbox::LPBox, movableobject::MovableObject};

#[cfg(feature = "nottest")]
use ::alloc::{boxed::Box, vec::Vec};
#[cfg(not(feature = "nottest"))]
use std::{boxed::Box};

type Cap = core::num::niche_types::UsizeNoHighBit;

pub struct LPVecCopy<T : Copy> {
    vec_inner : LPVec<LPAdapter<T>>,
}

impl<T : Copy> LPVecCopy<T> {
    #[inline]
    pub const fn new() -> Self {
        LPVecCopy {
            vec_inner : LPVec::new()
        }
    }

    pub fn try_with_capacity(capacity : usize) -> Option<Self> {
        LPVec::try_with_capacity(capacity).map(|v| LPVecCopy { vec_inner : v })
    }

    #[inline]
    pub const unsafe fn from_raw_parts(ptr : * mut T, len : usize, capacity : usize) -> Self {
        LPVecCopy { vec_inner : unsafe{ LPVec::from_raw_parts(ptr as * mut LPAdapter<T>, len, capacity) } }
    }

    #[inline]
    pub unsafe fn from_parts(ptr : NonNull<T>, len : usize, capacity : usize) -> Self {
        unsafe { Self::from_raw_parts(ptr.as_ptr(), len, capacity) }
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
        self.vec_inner.as_mut_ptr() as * mut T
    }
    #[rustc_never_returns_null_ptr]
    #[rustc_as_ptr]
    #[inline]
    pub const fn as_ptr(&self) -> *const T {
        self.vec_inner.as_ptr() as * const T
    }

    #[inline]
    pub const fn capacity(&self) -> usize {
        self.vec_inner.capacity()
    }

    #[inline]
    const fn needs_to_grow(&self, additional : usize) -> bool {
        self.len() + additional > self.capacity()
    }

    pub fn reserve_exact(&mut self, additional : usize) {
        self.vec_inner.reserve_exact(additional);
    }
    pub fn reserve(&mut self, additional : usize) {
        self.vec_inner.reserve_exact(additional);
    }

    pub fn try_reserve_exact(&mut self, additional : usize) -> Result<(), LPTryReserveError> {
        self.vec_inner.try_reserve_exact(additional)
    }
    pub fn try_reserve(&mut self, additional : usize) -> Result<(), LPTryReserveError>{
        self.vec_inner.try_reserve(additional)
    }

    #[inline]
    pub fn shrink_to_fit(&mut self) {
        self.vec_inner.shrink_to_fit();
    }

    #[inline]
    pub fn shrink_to(&mut self, min_capacity : usize) {
        self.vec_inner.shrink_to(min_capacity)
    }

    pub fn try_remove(&mut self, index: usize) -> Option<T> {
        self.vec_inner.try_remove(index).map(|v| *v)
    }

    #[track_caller]
    #[rustc_confusables("delete", "take")]
    pub fn remove(&mut self, index: usize) -> T {
        *self.vec_inner.remove(index)
    }

    #[must_use = "if you don't need a reference to the value, use `Vec::push` instead"]
    pub fn push_mut(&mut self, value : T) -> &mut T{
        self.vec_inner.push_mut(LPAdapter::new(value)).as_mut()
    }

    pub fn push(&mut self, value : T){
        self.vec_inner.push(LPAdapter::new(value))
    }

    pub fn push_mut_within_capacity(&mut self, value: T) -> Result<&mut T, T> {
        match self.vec_inner.push_mut_within_capacity(LPAdapter::new(value)) {
            Ok(slot) => Ok(slot.as_mut()),
            Err(value) => Err(*value)
        }
    }

    pub fn push_within_capacity(&mut self, value: T) -> Result<(), T> {
        self.push_mut_within_capacity(value).map(|_| ())
    }

    pub fn pop(&mut self) -> Option<T> {
        self.vec_inner.pop().map(|value| *value)
    }

    pub fn into_boxed_slice(mut self) -> LPBox<[LPAdapter<T>]> {
        self.vec_inner.shrink_to_fit();
        let me = ManuallyDrop::new(self);
        let vec = unsafe { ptr::read(&me.vec_inner) };
        let (ptr, len, _) = vec.into_raw_parts();
        LPBox::from_raw(ptr::slice_from_raw_parts_mut(ptr as *mut LPAdapter<T>, len))
    }

    pub fn truncate(&mut self, len: usize) {
        self.vec_inner.truncate(len);
    }

    #[inline]
    pub fn swap_remove(&mut self, index: usize) -> T {
        *self.vec_inner.swap_remove(index)
    }

    #[track_caller]
    pub fn insert(&mut self, index: usize, element: T) {
        let _ = self.insert_mut(index, element);
    }

    #[inline]
    #[track_caller]
    #[must_use = "if you don't need a reference to the value, use `Vec::insert` instead"]
    pub fn insert_mut(&mut self, index: usize, element: T) -> &mut T {
        self.vec_inner.insert_mut(index, LPAdapter::new(element)).as_mut()
    }

    
    pub fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(&T) -> bool,
    {
        self.vec_inner.retain(|elem| f(elem.as_ref()));
    }

    pub fn retain_mut<F>(&mut self, mut f: F)
    where
        F: FnMut(&mut T) -> bool,
    {
        self.vec_inner.retain_mut(|elem| f(elem.as_mut()));
    }

    #[inline]
    pub fn dedup_by_key<F, K>(&mut self, mut key: F)
    where
        F: FnMut(&mut T) -> K,
        K: PartialEq,
    {
        self.vec_inner.dedup_by_key(|value| key(value.as_mut()))
    }

    pub fn dedup_by<F>(&mut self, mut same_bucket: F)
    where
        F: FnMut(&mut T, &mut T) -> bool,
    {
        self.vec_inner
            .dedup_by(|left, right| same_bucket(left.as_mut(), right.as_mut()));
    }

    pub fn pop_if(&mut self, predicate: impl FnOnce(&mut T) -> bool) -> Option<T> {
        self.vec_inner
            .pop_if(|value| predicate(value.as_mut()))
            .map(|value| *value)
    }

    #[inline]
    pub fn append(&mut self, other: &mut Self) {
        self.vec_inner.append(&mut other.vec_inner);
    }

    #[inline]
    unsafe fn append_elements(&mut self, other: *const [T]) {
        unsafe { self.vec_inner.append_elements(other as * const [LPAdapter<T>]) };
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
        self.vec_inner.clear();
    }

    #[inline]
    #[must_use = "use `.truncate()` if you don't need the other half"]
    #[track_caller]
    pub fn split_off(&mut self, at: usize) -> Self
    {
        Self { vec_inner: self.vec_inner.split_off(at) }
    }

    pub fn resize_with<F>(&mut self, new_len: usize, mut f: F)
    where
        F: FnMut() -> T,
    {
        self.vec_inner
            .resize_with(new_len, move || LPAdapter::new(f()));
    }

    #[inline]
    pub fn spare_capacity_mut(&mut self) -> &mut [MaybeUninit<T>] {
        unsafe {
            slice::from_raw_parts_mut(
                self.as_mut_ptr().add(self.len()) as *mut MaybeUninit<T>,
                self.capacity() - self.len(),
            )
        }
    }

    #[inline]
    pub fn split_at_spare_mut(&mut self) -> (&mut [T], &mut [MaybeUninit<T>]) {
        let (l, r) = self.vec_inner.split_at_spare_mut();
        unsafe {
            (slice::from_raw_parts_mut(l.as_mut_ptr() as *mut T, l.len()), slice::from_raw_parts_mut(r.as_mut_ptr() as *mut MaybeUninit<T>, r.len()))
        }
    }

    #[inline]
    pub const fn as_slice(&self) -> &[T] {
        unsafe { slice::from_raw_parts(self.as_ptr(), self.len()) }
    }

    #[inline]
    pub const fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe { slice::from_raw_parts_mut(self.as_mut_ptr(), self.len()) }
    }

    pub const fn len(&self) -> usize {
        self.vec_inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    
    #[inline]
    pub fn leak<'a>(self) -> &'a mut [T]
    {
        let mut me = ManuallyDrop::new(self);
        unsafe { slice::from_raw_parts_mut(me.as_mut_ptr(), me.len()) }
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

    pub fn resize(&mut self, new_len: usize, value: T) {
        self.vec_inner.resize(new_len, LPAdapter::new(value));
    }

    pub fn extend_from_slice(&mut self, other: &[T]) {
        self.spec_extend(other.iter());
    }

    pub fn extend_from_within<R>(&mut self, src: R)
    where
        R: RangeBounds<usize>,
    {
        self.vec_inner.extend_from_within(src);
    }
    
    unsafe fn spec_extend_from_within(&mut self, src: Range<usize>) {
        unsafe { self.vec_inner.spec_extend_from_within(src); }
    }
}

impl<T : Copy, I> SpecExtend<T, I> for LPVecCopy<T>
where
    I: Iterator<Item = T>,
{
    default fn spec_extend(&mut self, iter: I) {
        self.vec_inner.spec_extend(iter.map(|v| LPAdapter::new(v)));
    }
}

impl<T: Copy, I> SpecExtend<T, I> for LPVecCopy<T>
where
    I: iter::TrustedLen<Item = T>,
{
    default fn spec_extend(&mut self, iterator: I) {
        self.vec_inner.spec_extend(iterator.map(|v| LPAdapter::new(v)));
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

impl<'a, T: 'a> SpecExtend<&'a T, slice::Iter<'a, T>> for LPVecCopy<T>
where
    T: Copy,
{
    fn spec_extend(&mut self, iterator: slice::Iter<'a, T>) {
        let slice = iterator.as_slice();
        unsafe { self.append_elements(slice) };
    }
}

impl<T: PartialEq + Copy> LPVecCopy<T> {
    #[inline]
    pub fn dedup(&mut self) {
        self.dedup_by(|a, b| a == b)
    }
}

impl<T : Copy> MovableObject for LPVecCopy<T> {
    #[cfg(any(feature = "has-lp-core", not(feature = "nottest")))]
    unsafe fn move_to_lp(&self, dest : *mut u8) {
        use core::ptr::addr_of_mut;
        let dest = dest as * mut Self;
        unsafe { self.vec_inner.move_to_lp(addr_of_mut!((*dest).vec_inner) as * mut u8); }
    }

    #[cfg(any(feature = "has-lp-core", not(feature = "nottest")))]
    unsafe fn move_to_main(&self, dest : *mut u8) {
        use core::ptr::addr_of_mut;
        let dest = dest as * mut Self;
        unsafe { self.vec_inner.move_to_main(addr_of_mut!((*dest).vec_inner) as * mut u8); }
    }
    #[cfg(feature = "is-lp-core")]
    unsafe fn move_to_main(&self, _dest : *mut u8) {
        unimplemented!()
    }
    #[cfg(feature = "is-lp-core")]
    unsafe fn move_to_lp(&self, _dest : *mut u8) {
        unimplemented!()
    }
}

impl<T : Copy> core::ops::Deref for LPVecCopy<T> {
    type Target = [T];

    #[inline]
    fn deref(&self) -> &[T] {
        self.as_slice()
    }
}

impl<T : Copy> core::ops::DerefMut for LPVecCopy<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut [T] {
        self.as_mut_slice()
    }
}

// unsafe impl<T> core::ops::DerefPure for LPVec<T> {}

impl<T : Copy, const N: usize> TryFrom<LPVecCopy<T>> for [T; N] {
    type Error = LPVecCopy<T>;
    #[allow(unused_mut)]
    fn try_from(mut vec: LPVecCopy<T>) -> Result<[T; N], LPVecCopy<T>> {
        match TryFrom::<LPVec<LPAdapter<T>>>::try_from(vec.vec_inner) {
            Ok(array) => Ok(array),
            Err(v) => Err(LPVecCopy { vec_inner: v }),
        }
    }
}

impl<T : Copy, const N: usize> TryFrom<LPVecCopy<T>> for [LPAdapter<T>; N] {
    type Error = LPVecCopy<T>;
    #[allow(unused_mut)]
    fn try_from(mut vec: LPVecCopy<T>) -> Result<[LPAdapter<T>; N], LPVecCopy<T>> {
        match TryFrom::<LPVec<LPAdapter<T>>>::try_from(vec.vec_inner) {
            Ok(array) => Ok(array),
            Err(v) => Err(LPVecCopy { vec_inner: v }),
        }
    }
}

// impl<T : Copy, const N: usize> TryFrom<LPVec<LPAdapter<T>>> for [T; N] {
//     type Error = LPVec<LPAdapter<T>>;

//     fn try_from(mut vec: LPVec<LPAdapter<T>>) -> Result<[T; N], LPVec<LPAdapter<T>>> {
//         if vec.len() != N {
//             return Err(vec);
//         }
//         unsafe { vec.set_len(0) };
//         let array = unsafe { ptr::read(vec.as_ptr() as *const [T; N]) };
//         Ok(array)
//     }
// }

impl<T : Copy> const Default for LPVecCopy<T> {
    fn default() -> LPVecCopy<T> {
        LPVecCopy::new()
    }
}

impl<T: fmt::Debug + Copy> fmt::Debug for LPVecCopy<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<T : Copy> AsRef<LPVecCopy<T>> for LPVecCopy<T> {
    fn as_ref(&self) -> &LPVecCopy<T> {
        self
    }
}

impl<T : Copy> AsMut<LPVecCopy<T>> for LPVecCopy<T> {
    fn as_mut(&mut self) -> &mut LPVecCopy<T> {
        self
    }
}

impl<T : Copy> AsRef<[T]> for LPVecCopy<T> {
    fn as_ref(&self) -> &[T] {
        self.as_slice()
    }
}

impl<T : Copy> AsMut<[T]> for LPVecCopy<T> {
    fn as_mut(&mut self) -> &mut [T] {
        self.as_mut_slice()
    }
}

impl<T : Copy> AsRef<[LPAdapter<T>]> for LPVecCopy<T> {
    fn as_ref(&self) -> &[LPAdapter<T>] {
        self.vec_inner.as_ref()
    }
}

impl<T : Copy> AsMut<[LPAdapter<T>]> for LPVecCopy<T> {
    fn as_mut(&mut self) -> &mut [LPAdapter<T>] {
        self.vec_inner.as_mut()
    }
}

impl<T : Copy> From<Vec<T>> for LPVecCopy<T> {
    fn from(s: Vec<T>) -> LPVecCopy<T> {
        let (buf, len, cap) = s.into_raw_parts();
        unsafe { LPVecCopy::from_raw_parts(buf, len, cap) }
    }
}

impl<T : Copy> From<Vec<LPAdapter<T>>> for LPVecCopy<T> {
    fn from(s: Vec<LPAdapter<T>>) -> LPVecCopy<T> {
        LPVecCopy { vec_inner: LPVec::from(s) }
    }
}

impl<T: Copy> From<&[T]> for LPVecCopy<T> {
    fn from(s: &[T]) -> LPVecCopy<T> {
        let s = s.cast_lp_adapter();
        LPVecCopy { vec_inner: LPVec::from(s) }
    }
}

impl<T: Copy> From<&mut [T]> for LPVecCopy<T> {
    fn from(s: &mut [T]) -> LPVecCopy<T> {
        Self::from(s as &[T])
    }
}

impl<T: Copy, const N: usize> From<&[T; N]> for LPVecCopy<T> {
    fn from(s: &[T; N]) -> LPVecCopy<T> {
        Self::from(s.as_slice())
    }
}

impl<T: Copy, const N: usize> From<&mut [T; N]> for LPVecCopy<T> {
    fn from(s: &mut [T; N]) -> LPVecCopy<T> {
        Self::from(s.as_mut_slice())
    }
}

impl<T : Copy, const N: usize> From<[T; N]> for LPVecCopy<T> {
    fn from(s: [T; N]) -> LPVecCopy<T> {
        Self::from(s.as_slice())
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

impl<T : Copy> From<LPBox<[LPAdapter<T>]>> for LPVecCopy<T> {
    fn from(s: LPBox<[LPAdapter<T>]>) -> Self {
        unsafe {
            let len = s.len();
            let b = LPBox::into_raw(s);
            LPVecCopy::from_raw_parts(b as *mut T, len, len)
        }
    }
}

impl<T : Copy> From<Box<[T]>> for LPVecCopy<T> {
    fn from(s: Box<[T]>) -> Self {
        unsafe {
            let len = s.len();
            let b = Box::into_raw(s);
            LPVecCopy::from_raw_parts(b as *mut T, len, len)
        }
    }
}

impl<T : Copy> From<Box<[LPAdapter<T>]>> for LPVecCopy<T> {
    fn from(s: Box<[LPAdapter<T>]>) -> Self {
        unsafe {
            let len = s.len();
            let b = Box::into_raw(s);
            LPVecCopy::from_raw_parts(b as *mut T, len, len)
        }
    }
}

impl<T : Copy> From<LPVecCopy<T>> for LPBox<[LPAdapter<T>]> {
    fn from(v: LPVecCopy<T>) -> Self {
        v.into_boxed_slice()
    }
}

impl<T : Copy> From<LPVec<LPAdapter<T>>> for LPVecCopy<T> {
    fn from(s: LPVec<LPAdapter<T>>) -> LPVecCopy<T> {
        LPVecCopy { vec_inner: s }
    }
}

impl<T : Copy> Into<LPVec<LPAdapter<T>>> for LPVecCopy<T> {
    fn into(self) -> LPVec<LPAdapter<T>> {
        self.vec_inner
    }
}

impl<T: Copy> Clone for LPVecCopy<T> {
    fn clone(&self) -> Self {
        Self::from(&**self)
    }

    fn clone_from(&mut self, source: &Self) {
        self.vec_inner.clone_from(&source.vec_inner);
    }
}

impl<T: core::hash::Hash + Copy> core::hash::Hash for LPVecCopy<T> {
    #[inline]
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        core::hash::Hash::hash(&**self, state)
    }
}

impl<T : Copy, I: SliceIndex<[T]>> Index<I> for LPVecCopy<T> {
    type Output = I::Output;

    #[inline]
    fn index(&self, index: I) -> &Self::Output {
        Index::index(&**self, index)
    }
}

impl<T : Copy, I: SliceIndex<[T]>> IndexMut<I> for LPVecCopy<T> {
    #[inline]
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        IndexMut::index_mut(&mut **self, index)
    }
}


macro_rules! __impl_slice_eq1 {
    ([$($vars:tt)*] $lhs:ty, $rhs:ty $(where $ty:ty: $bound:ident)?) => {
        impl<T, $($vars)*> PartialEq<$rhs> for $lhs
        where
            T: PartialEq<U> + Copy, // T and U must implement Copy
            $($ty: $bound)?
        {
            #[inline]
            fn eq(&self, other: &$rhs) -> bool { self[..] == other[..] }
            #[inline]
            fn ne(&self, other: &$rhs) -> bool { self[..] != other[..] }
        }
    }
}

__impl_slice_eq1! { [U : Copy] LPVecCopy<T>, LPVecCopy<U> }
__impl_slice_eq1! { [U] LPVecCopy<T>, &[U] }
__impl_slice_eq1! { [U] LPVecCopy<T>, &mut [U] }
__impl_slice_eq1! { [U : Copy] &[T], LPVecCopy<U> }
__impl_slice_eq1! { [U : Copy] &mut [T], LPVecCopy<U> }
__impl_slice_eq1! { [U] LPVecCopy<T>, [U]  }
__impl_slice_eq1! { [U : Copy] [T], LPVecCopy<U>  }
__impl_slice_eq1! { [const N: usize, U] LPVecCopy<T>, [U; N] }
__impl_slice_eq1! { [const N: usize, U] LPVecCopy<T>, &[U; N] }

impl<T: PartialOrd + Copy> PartialOrd for LPVecCopy<T>
{
    #[inline]
    fn partial_cmp(&self, other: &LPVecCopy<T>) -> Option<core::cmp::Ordering> {
        PartialOrd::partial_cmp(&**self, &**other)
    }
}

impl<T: Eq + Copy> Eq for LPVecCopy<T> {}

impl<T: Ord + Copy> Ord for LPVecCopy<T> {
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