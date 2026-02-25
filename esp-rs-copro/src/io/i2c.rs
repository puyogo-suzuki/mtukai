use crate::{movableobject::MovableObject, EspCoproError};

/// A wrapper for low-power I2C peripheral that can be used in the LP core.
/// This struct is a movable object and can be transferred between the main core and the LP core.
/// It provides basic I2C operations such as read, write, and write_read.
pub struct LPI2C {
    #[cfg(feature = "has-lp-core")]
    #[allow(unused)]
    i2c : esp_hal::i2c::lp_i2c::LpI2c,
    #[cfg(feature = "is-lp-core")]
    #[allow(unused)]
    i2c : esp_lp_hal::i2c::LpI2c,
}

/// Possible errors that can occur during I2C operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LPI2CError {
    /// The buffer size exceeds the FIFO limit of the I2C controller.
    ExceedingFifo,
    /// The I2C controller did not receive an acknowledgment from the device.
    AckCheckFailed,
    /// The I2C controller timed out while waiting for a response from the device.
    TimeOut,
    /// The I2C controller lost arbitration while trying to access the bus.
    ArbitrationLost,
    /// The I2C controller did not complete the execution of the command.
    ExecIncomplete,
    /// The number of commands exceeds the limit of the I2C controller.
    CommandNrExceeded,
    /// The I2C controller received an invalid response from the device.
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
    /// Write data to the I2C device at the specified address.
    #[cfg(feature = "is-lp-core")]
    pub fn write(&mut self, address : u8, bytes : &[u8]) -> Result<(), LPI2CError> {
        LPI2CError::convert_from_result(self.i2c.write(address, bytes))
    }
    /// Read data from the I2C device at the specified address.
    #[cfg(feature = "is-lp-core")]
    pub fn read(&mut self, address : u8, buffer : &mut [u8]) -> Result<(), LPI2CError> {
        LPI2CError::convert_from_result(self.i2c.read(address, buffer))
    }
    /// Write data to the I2C device and then read data from it in a single transaction.
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
    unsafe fn move_to_main(&self, _dest : *mut u8) -> Result<(), EspCoproError> {
        // Do nothing, LPI2C is a zero sized type.
        Ok(())
    }

    unsafe fn move_to_lp(&self, _dest : *mut u8) -> Result<(), EspCoproError> {
        // Do nothing, LPI2C is a zero sized type.
        Ok(())
    }
}