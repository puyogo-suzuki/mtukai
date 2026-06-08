/// LICENSE Information
/// Many parts of this file comes from Rust Project, licensed under Apache License 2.0 or MIT License.
/// Copyright (c) The Rust Project Contributors.
/// https://github.com/rust-lang/rust
use core::fmt;
use core::iter::{FusedIterator, TrustedLen};
use core::mem::{self, ManuallyDrop, SizedTypeProperties};
use core::ptr::{self, NonNull};
use core::slice::{self};

use crate::collections::lpvec::LPVec;
use crate::movableobject::MovableObject;

pub struct LPDrain<
    'a,
    T: 'a + MovableObject,
> {
    /// Index of tail to preserve
    pub(super) tail_start: usize,
    /// Length of tail
    pub(super) tail_len: usize,
    /// Current remaining range to remove
    pub(super) iter: slice::Iter<'a, T>,
    pub(super) vec: NonNull<LPVec<T>>,
}

impl<T: fmt::Debug + MovableObject> fmt::Debug for LPDrain<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("LPDrain").field(&self.iter.as_slice()).finish()
    }
}

impl<'a, T: MovableObject> LPDrain<'a, T> {
    #[must_use]
    pub fn as_slice(&self) -> &[T] {
        self.iter.as_slice()
    }

    pub fn keep_rest(self) {
        let mut this = ManuallyDrop::new(self);

        unsafe {
            let source_vec = this.vec.as_mut();

            let start = source_vec.len();
            let tail = this.tail_start;

            let unyielded_len = this.iter.len();
            let unyielded_ptr = this.iter.as_slice().as_ptr();

            if !T::IS_ZST {
                let start_ptr = source_vec.as_mut_ptr().add(start);
                if unyielded_ptr != start_ptr {
                    let src = unyielded_ptr;
                    let dst = start_ptr;

                    ptr::copy(src, dst, unyielded_len);
                }
                if tail != (start + unyielded_len) {
                    let src = source_vec.as_ptr().add(tail);
                    let dst = start_ptr.add(unyielded_len);
                    ptr::copy(src, dst, this.tail_len);
                }
            }

            source_vec.set_len(start + unyielded_len + this.tail_len);
        }
    }
}

impl<'a, T : MovableObject> AsRef<[T]> for LPDrain<'a, T> {
    fn as_ref(&self) -> &[T] {
        self.as_slice()
    }
}

unsafe impl<T: Sync + MovableObject> Sync for LPDrain<'_, T> {}
unsafe impl<T: Send + MovableObject> Send for LPDrain<'_, T> {}

impl<T: MovableObject> Iterator for LPDrain<'_, T> {
    type Item = T;

    #[inline]
    fn next(&mut self) -> Option<T> {
        self.iter.next().map(|elt| unsafe { ptr::read(elt as *const _) })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<T: MovableObject> DoubleEndedIterator for LPDrain<'_, T> {
    #[inline]
    fn next_back(&mut self) -> Option<T> {
        self.iter.next_back().map(|elt| unsafe { ptr::read(elt as *const _) })
    }
}

impl<T: MovableObject> Drop for LPDrain<'_, T> {
    fn drop(&mut self) {
        struct DropGuard<'r, 'a, T : MovableObject>(&'r mut LPDrain<'a, T>);

        impl<'r, 'a, T : MovableObject> Drop for DropGuard<'r, 'a, T> {
            fn drop(&mut self) {
                if self.0.tail_len > 0 {
                    unsafe {
                        let source_vec = self.0.vec.as_mut();
                        let start = source_vec.len();
                        let tail = self.0.tail_start;
                        if tail != start {
                            let src = source_vec.as_ptr().add(tail);
                            let dst = source_vec.as_mut_ptr().add(start);
                            ptr::copy(src, dst, self.0.tail_len);
                        }
                        source_vec.set_len(start + self.0.tail_len);
                    }
                }
            }
        }

        let iter = mem::take(&mut self.iter);
        let drop_len = iter.len();

        let mut vec = self.vec;

        if T::IS_ZST {
            unsafe {
                let vec = vec.as_mut();
                let old_len = vec.len();
                vec.set_len(old_len + drop_len + self.tail_len);
                vec.truncate(old_len + self.tail_len);
            }
            return;
        }

        let _guard = DropGuard(self);

        if drop_len == 0 {
            return;
        }

        let drop_ptr = iter.as_slice().as_ptr();

        unsafe {
            let vec_ptr = vec.as_mut().as_mut_ptr();
            let drop_offset = drop_ptr.offset_from_unsigned(vec_ptr);
            let to_drop = ptr::slice_from_raw_parts_mut(vec_ptr.add(drop_offset), drop_len);
            ptr::drop_in_place(to_drop);
        }
    }
}

impl<T: MovableObject> ExactSizeIterator for LPDrain<'_, T> {
    fn is_empty(&self) -> bool {
        self.iter.is_empty()
    }
}

unsafe impl<T: MovableObject> TrustedLen for LPDrain<'_, T> {}
impl<T: MovableObject> FusedIterator for LPDrain<'_, T> {}