use core::{ops::{Deref, DerefMut}, ptr, slice};
use crate::movableobject::MovableObject;

/// This struct is a wrapper around a type `T` that allows it to be transferred between the main and the LP processors without requiring any special handling.
/// It is designed to be used with types that are [`Copy`], as it simply copies the inner value when transferring.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct LPAdapter<T> where T : Copy {
    inner: T
}

impl<T: Copy> LPAdapter<T> {
    /// Creates a new [`LPAdapter`] wrapping the given value.
    pub fn new(inner: T) -> Self {
        Self { inner }
    }
}

/// This trait allows for converting between slices of `T` and slices of [`LPAdapter<T>`].
/// This is safe because the memory layout of [`LPAdapter<T>`] is the same as `T`, and both types are [`Copy`].
pub trait LPAdapterSliceConvert<T : Copy> {
    fn cast_lp_adapter(&self) -> &[LPAdapter<T>];
    fn cast_mut_lp_adapter(&mut self) -> &mut [LPAdapter<T>];
}

impl<T : Copy> LPAdapterSliceConvert<T> for [T] {
    fn cast_lp_adapter(&self) -> &[LPAdapter<T>] {
        unsafe { slice::from_raw_parts(self.as_ptr() as *const LPAdapter<T>, self.len()) }
    }
    fn cast_mut_lp_adapter(&mut self) -> &mut [LPAdapter<T>] {
        unsafe { slice::from_raw_parts_mut(self.as_mut_ptr() as *mut LPAdapter<T>, self.len()) }
    }
}

/// This trait allows for converting between slices of `T` and slices of [`LPAdapter<T>`].
/// This is safe because the memory layout of [`LPAdapter<T>`] is the same as `T`, and both types are [`Copy`].
pub trait LPAdapterSliceConvertFrom<T : Copy> {
    fn cast_lp_adapter(&self) -> &[T];
    fn cast_mut_lp_adapter(&mut self) -> &mut [T];
}

impl<T : Copy> LPAdapterSliceConvertFrom<T> for [LPAdapter<T>] {
    fn cast_lp_adapter(&self) -> &[T] {
        unsafe { slice::from_raw_parts(self.as_ptr() as *const T, self.len()) }
    }
    fn cast_mut_lp_adapter(&mut self) -> &mut [T] {
        unsafe { slice::from_raw_parts_mut(self.as_mut_ptr() as *mut T, self.len()) }
    }
}

impl<T: Copy + PartialEq> PartialEq<T> for LPAdapter<T> {
    fn eq(&self, other: &T) -> bool {
        self.inner == *other
    }
}

impl<T: Copy + PartialOrd> PartialOrd<T> for LPAdapter<T> {
    fn partial_cmp(&self, other: &T) -> Option<core::cmp::Ordering> {
        self.inner.partial_cmp(other)
    }
}

impl<T : Copy> MovableObject for LPAdapter<T> {
    unsafe fn move_to_main(&self, dest : *mut u8) -> Result<(), crate::EspCoproError> {
        unsafe {
            ptr::write(dest as *mut LPAdapter<T>, LPAdapter { inner: self.inner });
        }
        Ok(())
    }
    unsafe fn move_to_lp(&self, dest : *mut u8) -> Result<(), crate::EspCoproError> {
        unsafe {
            ptr::write(dest as *mut LPAdapter<T>, LPAdapter { inner: self.inner });
        }
        Ok(())
    }
}

impl<T: Copy> Deref for LPAdapter<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.inner
    }
}

impl<T: Copy> DerefMut for LPAdapter<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}

impl<T: Copy> AsRef<T> for LPAdapter<T> {
    fn as_ref(&self) -> &T {
        &self.inner
    }
}

impl<T: Copy> AsMut<T> for LPAdapter<T> {
    fn as_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}

impl<T: Copy> From<T> for LPAdapter<T> {
    fn from(value: T) -> Self {
        Self { inner: value }
    }
}