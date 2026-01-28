use core::{ops::{Deref, DerefMut}, ptr};

use crate::movableobject::MovableObject;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LPAdapter<T> where T : Copy {
    inner: T
}

impl<T : Copy> MovableObject for LPAdapter<T> {
    unsafe fn move_to_main(&self, dest : *mut u8) {
        unsafe {
            ptr::write(dest as *mut LPAdapter<T>, LPAdapter { inner: self.inner });
        }
    }
    unsafe fn move_to_lp(&self, dest : *mut u8) {
        unsafe {
            ptr::write(dest as *mut LPAdapter<T>, LPAdapter { inner: self.inner });
        }
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