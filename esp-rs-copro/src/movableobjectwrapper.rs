use super::movableobject::MovableObject;

#[doc(hidden)]
pub trait MovableObjectWrapFallback {
    fn wrap_move_to_main(&self, _dest : *mut u8) -> Result<(), crate::EspCoproError>;
    fn wrap_move_to_lp(&self, _dest : *mut u8) -> Result<(), crate::EspCoproError>;
    #[cfg(feature = "has-lp-core")]
    fn wrap_transfer_to_lp(&self) -> Result<*mut u8, crate::EspCoproError>;
    #[cfg(feature = "has-lp-core")]
    unsafe fn wrap_transfer_to_main(&mut self, src : * const u8) -> Result<(), crate::EspCoproError>; 
    #[cfg(feature = "has-lp-core")]
    unsafe fn wrap_transfer_to_main_sub(&mut self, src : * const u8) -> Result<(), crate::EspCoproError>; 
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
    #[cfg(feature = "has-lp-core")]
    fn wrap_transfer_to_lp(&self) -> Result<*mut u8, crate::EspCoproError> {
        crate::transfer_functions::transfer_to_lp_copy(self)
    }
    #[cfg(feature = "has-lp-core")]
    unsafe fn wrap_transfer_to_main(&mut self, src : * const u8) -> Result<(), crate::EspCoproError> {
        unsafe{ crate::transfer_functions::transfer_to_main_copy(src, self) }
    }
    #[cfg(feature = "has-lp-core")]
    unsafe fn wrap_transfer_to_main_sub(&mut self, src : * const u8) -> Result<(), crate::EspCoproError> {
        unsafe { crate::transfer_functions::transfer_to_main_sub_copy(src, self) }
    }
}

#[doc(hidden)]
pub trait MovableObjectWrap {
    fn wrap_move_to_main(&self, dest : *mut u8) -> Result<(), crate::EspCoproError>;
    fn wrap_move_to_lp(&self, dest : *mut u8) -> Result<(), crate::EspCoproError>;
    #[cfg(feature = "has-lp-core")]
    fn wrap_transfer_to_lp(&self) -> Result<*mut u8, crate::EspCoproError>;
    #[cfg(feature = "has-lp-core")]
    unsafe fn wrap_transfer_to_main(&mut self, src : * const u8) -> Result<(), crate::EspCoproError>;
    #[cfg(feature = "has-lp-core")]
    unsafe fn wrap_transfer_to_main_sub(&mut self, src : * const u8) -> Result<(), crate::EspCoproError>;
}
#[doc(hidden)]
impl<T: MovableObject> MovableObjectWrap for T {
    fn wrap_move_to_main(&self, dest : *mut u8) -> Result<(), crate::EspCoproError> {
        unsafe{ self.move_to_main(dest) } 
    }
    fn wrap_move_to_lp(&self, dest : *mut u8) -> Result<(), crate::EspCoproError> {
        unsafe{ self.move_to_lp(dest) }
    }
    #[cfg(feature = "has-lp-core")]
    fn wrap_transfer_to_lp(&self) -> Result<*mut u8, crate::EspCoproError> {
        crate::transfer_functions::transfer_to_lp(self)
    }
    #[cfg(feature = "has-lp-core")]
    unsafe fn wrap_transfer_to_main(&mut self, src : * const u8) -> Result<(), crate::EspCoproError> {
        unsafe { crate::transfer_functions::transfer_to_main(src, self) }
    }
    #[cfg(feature = "has-lp-core")]
    unsafe fn wrap_transfer_to_main_sub(&mut self, src : * const u8) -> Result<(), crate::EspCoproError> {
        unsafe { crate::transfer_functions::transfer_to_main_sub(src, self) }
    }
}