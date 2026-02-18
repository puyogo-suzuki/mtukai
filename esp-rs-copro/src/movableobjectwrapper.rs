use super::movableobject::MovableObject;

#[doc(hidden)]
pub trait MovableObjectWrapFallback {
    fn wrap_move_to_main(&self, _dest : *mut u8) -> Result<(), crate::EspCoproError>;
    fn wrap_move_to_lp(&self, _dest : *mut u8) -> Result<(), crate::EspCoproError>;
}
#[doc(hidden)]
impl<T: Copy> MovableObjectWrapFallback for T {
    fn wrap_move_to_main(&self, dest : *mut u8) -> Result<(), crate::EspCoproError> { 
        unsafe { *(dest as *mut T) = *self; }
        Ok(())
    }
    fn wrap_move_to_lp(&self, dest : *mut u8) -> Result<(), crate::EspCoproError> {
        unsafe { *(dest as *mut T) = *self; }
        Ok(())
    }
}

#[doc(hidden)]
pub trait MovableObjectWrap {
    fn wrap_move_to_main(&self, dest : *mut u8) -> Result<(), crate::EspCoproError>;
    fn wrap_move_to_lp(&self, dest : *mut u8) -> Result<(), crate::EspCoproError>;
}
#[doc(hidden)]
impl<T: MovableObject> MovableObjectWrap for T {
    fn wrap_move_to_main(&self, dest : *mut u8) -> Result<(), crate::EspCoproError> {
        unsafe{ self.move_to_main(dest) } 
    }
    fn wrap_move_to_lp(&self, dest : *mut u8) -> Result<(), crate::EspCoproError> {
        unsafe{ self.move_to_lp(dest) }
    }
}