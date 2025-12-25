use super::movableobject::MovableObject;

#[doc(hidden)]
pub trait MovableObjectWrapFallback {
    fn wrap_move_to_main(&self, _dest : *mut u8) { }
    fn wrap_move_to_lp(&self, _dest : *mut u8) { }
}
#[doc(hidden)]
impl<T: Copy> MovableObjectWrapFallback for T {
    fn wrap_move_to_main(&self, dest : *mut u8) { 
        unsafe { *(dest as *mut T) = *self; }
    }
    fn wrap_move_to_lp(&self, dest : *mut u8) {
        unsafe { *(dest as *mut T) = *self; }
    }
}

#[doc(hidden)]
pub trait MovableObjectWrap {
    fn wrap_move_to_main(&self, dest : *mut u8);
    fn wrap_move_to_lp(&self, dest : *mut u8);
}
#[doc(hidden)]
impl<T: MovableObject> MovableObjectWrap for T {
    fn wrap_move_to_main(&self, dest : *mut u8) {
        unsafe{ self.move_to_main(dest); }
    }
    fn wrap_move_to_lp(&self, dest : *mut u8) {
        unsafe{ self.move_to_lp(dest); }
    }
}