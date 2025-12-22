use crate::movableobject::MovableObject;

pub struct LPI2C {
    #[cfg(feature = "has-lp-core")]
    i2c : esp_hal::i2c::lp_i2c::LpI2c,
    #[cfg(feature = "is-lp-core")]
    i2c : esp_lp_hal::i2c::LpI2c,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LPI2CError {
    ExceedingFifo,
    AckCheckFailed,
    TimeOut,
    ArbitrationLost,
    ExecIncomplete,
    CommandNrExceeded,
    InvalidResponse,
}

impl LPI2CError {
    #[cfg(feature = "is-lp-core")]
    fn convert_from_result<T>(val : Result<T, esp_lp_hal::i2c::Error>) -> Result<T, LPI2CError> {
        match val {
            Ok(v) => Ok(v),
            Err(e) => match e {
                esp_lp_hal::i2c::Error::ExceedingFifo => Err(LPI2CError::ExceedingFifo),
                esp_lp_hal::i2c::Error::AckCheckFailed => Err(LPI2CError::AckCheckFailed),
                esp_lp_hal::i2c::Error::TimeOut => Err(LPI2CError::TimeOut),
                esp_lp_hal::i2c::Error::ArbitrationLost => Err(LPI2CError::ArbitrationLost),
                esp_lp_hal::i2c::Error::ExecIncomplete => Err(LPI2CError::ExecIncomplete),
                esp_lp_hal::i2c::Error::CommandNrExceeded => Err(LPI2CError::CommandNrExceeded),
                esp_lp_hal::i2c::Error::InvalidResponse => Err(LPI2CError::InvalidResponse),
            }
        }
    }
    #[cfg(feature = "has-lp-core")]
    fn convert_from_result<T>(val : Result<T, esp_hal::i2c::lp_i2c::Error>) -> Result<T, LPI2CError> {
        match val {
            Ok(v) => Ok(v),
            Err(e) => match e {
                esp_hal::i2c::lp_i2c::Error::ExceedingFifo => Err(LPI2CError::ExceedingFifo),
                esp_hal::i2c::lp_i2c::Error::AckCheckFailed => Err(LPI2CError::AckCheckFailed),
                esp_hal::i2c::lp_i2c::Error::TimeOut => Err(LPI2CError::TimeOut),
                esp_hal::i2c::lp_i2c::Error::ArbitrationLost => Err(LPI2CError::ArbitrationLost),
                esp_hal::i2c::lp_i2c::Error::ExecIncomplete => Err(LPI2CError::ExecIncomplete),
                esp_hal::i2c::lp_i2c::Error::CommandNrExceeded => Err(LPI2CError::CommandNrExceeded),
                esp_hal::i2c::lp_i2c::Error::InvalidResponse => Err(LPI2CError::InvalidResponse),
            }
        }
    }
}

impl LPI2C {
    #[cfg(feature = "has-lp-core")]
    pub fn new(i2c : esp_hal::i2c::lp_i2c::LpI2c) -> Self { LPI2C { i2c } }
    #[cfg(feature = "is-lp-core")]
    pub fn new(i2c : esp_lp_hal::i2c::LpI2c) -> Self { LPI2C { i2c } }
    #[cfg(feature = "is-lp-core")]
    pub fn write(&mut self, address : u8, bytes : &[u8]) -> Result<(), LPI2CError> {
        LPI2CError::convert_from_result(self.i2c.write(address, bytes))
    }
    #[cfg(feature = "is-lp-core")]
    pub fn read(&mut self, address : u8, buffer : &mut [u8]) -> Result<(), LPI2CError> {
        LPI2CError::convert_from_result(self.i2c.read(address, buffer))
    }
    #[cfg(feature = "is-lp-core")]
    pub fn write_read(
        &mut self,
        address: u8,
        bytes: &[u8],
        buffer: &mut [u8],
    ) -> Result<(), LPI2CError> {
        LPI2CError::convert_from_result(self.i2c.write_read(address, bytes, buffer))
    }
}

impl MovableObject for LPI2C {
    unsafe fn rewrite_pointers_to_main(&self, _dest : *mut u8) {
        // Do nothing, LPI2C is a zero sized type.
    }

    unsafe fn rewrite_pointers_to_lp(&self, _dest : *mut u8) {
        // Do nothing, LPI2C is a zero sized type.
    }
}