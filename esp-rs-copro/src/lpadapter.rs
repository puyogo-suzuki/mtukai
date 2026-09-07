use core::{ops::{AddAssign, BitAndAssign, BitOrAssign, BitXorAssign, Deref, DerefMut, DivAssign, MulAssign, RemAssign, ShlAssign, ShrAssign, SubAssign}, ptr, slice};
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
    pub fn as_lpadapter(inner : &T) -> &LPAdapter<T> {
        unsafe { &*(inner as *const T as *const LPAdapter<T>) }
    }
    pub fn as_lpadapter_mut(inner : &mut T) -> &mut LPAdapter<T> {
        unsafe { &mut *(inner as *mut T as *mut  LPAdapter<T>) }
    }
    pub fn as_inner_ref(&self) -> &T {
        &self.inner
    }
    pub fn as_inner_mut(&mut self) -> &mut T {
        &mut self.inner
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

macro_rules! impl_assign {
    ($trait:ident, $method:ident, $op:tt) => {
        impl<T: $trait<T> + Copy> $trait<T> for LPAdapter<T> {
            fn $method(&mut self, rhs: T) {
                self.inner $op rhs;
            }
        }

        impl<T: $trait<T> + Copy> $trait<LPAdapter<T>> for LPAdapter<T> {
            fn $method(&mut self, rhs: LPAdapter<T>) {
                self.inner $op rhs.inner;
            }
        }
    };
}

impl_assign!(AddAssign, add_assign, +=);
impl_assign!(SubAssign, sub_assign, -=);
impl_assign!(MulAssign, mul_assign, *=);
impl_assign!(DivAssign, div_assign, /=);
impl_assign!(RemAssign, rem_assign, %=);
impl_assign!(ShlAssign, shl_assign, <<=);
impl_assign!(ShrAssign, shr_assign, >>=);
impl_assign!(BitOrAssign, bitor_assign, |=);
impl_assign!(BitAndAssign, bitand_assign, &=);
impl_assign!(BitXorAssign, bitxor_assign, ^=);