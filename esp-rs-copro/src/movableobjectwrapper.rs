use crate::movableobject::MovableObject;
#[cfg(feature = "has-lp-core")]
use {
    crate::lpadapter::{LPAdapter, LPAdapterSliceConvert, LPAdapterSliceConvertFrom},
    core::ptr::NonNull
};
#[cfg(feature = "has-lp-core")]
pub trait MovableObjectWrapperForSlice {
    // We do not intend to support slices on struct fields. This is only for the top-level slice in the entry function arguments.
    fn wrap_transfer_to_lp(&self) -> Result<NonNull<Self>, crate::EspCoproError>;
    unsafe fn wrap_transfer_to_main(&mut self, src : NonNull<Self>) -> Result<(), crate::EspCoproError>;
    unsafe fn wrap_transfer_to_main_sub(&mut self, src : NonNull<Self>) -> Result<(), crate::EspCoproError>;
}
#[doc(hidden)]
#[cfg(feature = "has-lp-core")]
impl<T: Copy> MovableObjectWrapperForSlice for [T] {
    fn wrap_transfer_to_lp(&self) -> Result<NonNull<Self>, crate::EspCoproError> {
        crate::transfer_functions::transfer_to_lp(self.cast_lp_adapter()).map(|mut ret| unsafe{ NonNull::from_ref(LPAdapterSliceConvertFrom::cast_mut_lp_adapter(ret.as_mut())) })
    }
    unsafe fn wrap_transfer_to_main(&mut self, src : NonNull<Self>) -> Result<(), crate::EspCoproError> {
        unsafe{ 
            let src_aslpadapter = NonNull::from_ref(src.as_ref().cast_lp_adapter());
            crate::transfer_functions::transfer_to_main(src_aslpadapter, self.cast_mut_lp_adapter())
        }
    }
    unsafe fn wrap_transfer_to_main_sub(&mut self, src : NonNull<Self>) -> Result<(), crate::EspCoproError> {
        unsafe { 
            let src_aslpadapter = NonNull::from_ref(src.as_ref().cast_lp_adapter());
            crate::transfer_functions::transfer_to_main_sub(src_aslpadapter, self.cast_mut_lp_adapter())
        }
    }
}
#[cfg(feature = "has-lp-core")]
pub trait FallbackMovableObjectWrapperForSlice {
    // We do not intend to support slices on struct fields. This is only for the top-level slice in the entry function arguments.
    fn wrap_transfer_to_lp(&self) -> Result<NonNull<Self>, crate::EspCoproError>;
    unsafe fn wrap_transfer_to_main(&mut self, src : NonNull<Self>) -> Result<(), crate::EspCoproError>;
    unsafe fn wrap_transfer_to_main_sub(&mut self, src : NonNull<Self>) -> Result<(), crate::EspCoproError>;
}
#[doc(hidden)]
#[cfg(feature = "has-lp-core")]
impl<T: MovableObject> FallbackMovableObjectWrapperForSlice for [T] {
    fn wrap_transfer_to_lp(&self) -> Result<NonNull<Self>, crate::EspCoproError> {
        crate::transfer_functions::transfer_to_lp(self).map(|mut ret| unsafe{ NonNull::from_ref(ret.as_mut()) })
    }
    unsafe fn wrap_transfer_to_main(&mut self, src : NonNull<Self>) -> Result<(), crate::EspCoproError> {
        unsafe{ crate::transfer_functions::transfer_to_main(NonNull::from_ref(src.as_ref()), self) }
    }
    unsafe fn wrap_transfer_to_main_sub(&mut self, src : NonNull<Self>) -> Result<(), crate::EspCoproError> {
        unsafe { crate::transfer_functions::transfer_to_main_sub(NonNull::from_ref(src.as_ref()), self) }
    }
}

#[doc(hidden)]
pub trait MovableObjectWrap {
    fn wrap_move_to_main(&self, dest : *mut u8) -> Result<(), crate::EspCoproError>;
    fn wrap_move_to_lp(&self, dest : *mut u8) -> Result<(), crate::EspCoproError>;
    #[cfg(feature = "has-lp-core")]
    fn wrap_transfer_to_lp(&self) -> Result<NonNull<Self>, crate::EspCoproError>;
    #[cfg(feature = "has-lp-core")]
    unsafe fn wrap_transfer_to_main(&mut self, src : NonNull<Self>) -> Result<(), crate::EspCoproError>;
    #[cfg(feature = "has-lp-core")]
    unsafe fn wrap_transfer_to_main_sub(&mut self, src : NonNull<Self>) -> Result<(), crate::EspCoproError>;
}
#[doc(hidden)]
impl<T: MovableObject + ?Sized> MovableObjectWrap for T {
    fn wrap_move_to_main(&self, dest : *mut u8) -> Result<(), crate::EspCoproError> {
        unsafe{ self.move_to_main(dest) } 
    }
    fn wrap_move_to_lp(&self, dest : *mut u8) -> Result<(), crate::EspCoproError> {
        unsafe{ self.move_to_lp(dest) }
    }
    #[cfg(feature = "has-lp-core")]
    fn wrap_transfer_to_lp(&self) -> Result<NonNull<Self>, crate::EspCoproError> {
        crate::transfer_functions::transfer_to_lp(self)
    }
    #[cfg(feature = "has-lp-core")]
    unsafe fn wrap_transfer_to_main(&mut self, src : NonNull<Self>) -> Result<(), crate::EspCoproError> {
        unsafe { crate::transfer_functions::transfer_to_main(src, self) }
    }
    #[cfg(feature = "has-lp-core")]
    unsafe fn wrap_transfer_to_main_sub(&mut self, src : NonNull<Self>) -> Result<(), crate::EspCoproError> {
        unsafe { crate::transfer_functions::transfer_to_main_sub(src, self) }
    }
}

#[doc(hidden)]
pub trait MovableObjectWrapFallback {
    fn wrap_move_to_main(&self, _dest : *mut u8) -> Result<(), crate::EspCoproError>;
    fn wrap_move_to_lp(&self, _dest : *mut u8) -> Result<(), crate::EspCoproError>;
    #[cfg(feature = "has-lp-core")]
    fn wrap_transfer_to_lp(&self) -> Result<NonNull<Self>, crate::EspCoproError>;
    #[cfg(feature = "has-lp-core")]
    unsafe fn wrap_transfer_to_main(&mut self, src : NonNull<Self>) -> Result<(), crate::EspCoproError>; 
    #[cfg(feature = "has-lp-core")]
    unsafe fn wrap_transfer_to_main_sub(&mut self, src : NonNull<Self>) -> Result<(), crate::EspCoproError>; 
}

#[doc(hidden)]
impl<T: Copy + ?Sized> MovableObjectWrapFallback for T {
    fn wrap_move_to_main(&self, dest : *mut u8) -> Result<(), crate::EspCoproError> { 
        unsafe { *(dest as *mut T) = *self; }
        Ok(())
    }
    fn wrap_move_to_lp(&self, dest : *mut u8) -> Result<(), crate::EspCoproError> {
        unsafe { *(dest as *mut T) = *self; }
        Ok(())
    }
    #[cfg(feature = "has-lp-core")]
    fn wrap_transfer_to_lp(&self) -> Result<NonNull<Self>, crate::EspCoproError> {
        crate::transfer_functions::transfer_to_lp(LPAdapter::as_lpadapter(self)).map(|mut ret| unsafe{ NonNull::from_ref(ret.as_mut().as_inner_mut()) })
    }
    #[cfg(feature = "has-lp-core")]
    unsafe fn wrap_transfer_to_main(&mut self, src : NonNull<Self>) -> Result<(), crate::EspCoproError> {
        unsafe{ 
            let src_aslpadapter = NonNull::from_ref(LPAdapter::as_lpadapter(src.as_ref()));
            crate::transfer_functions::transfer_to_main(src_aslpadapter, LPAdapter::as_lpadapter_mut(self))
        }
    }
    
    #[cfg(feature = "has-lp-core")]
    unsafe fn wrap_transfer_to_main_sub(&mut self, src : NonNull<Self>) -> Result<(), crate::EspCoproError> {
        unsafe { 
            let src_aslpadapter = NonNull::from_ref(LPAdapter::as_lpadapter(src.as_ref()));
            crate::transfer_functions::transfer_to_main_sub(src_aslpadapter, LPAdapter::as_lpadapter_mut(self))
        }
    }
}