#[cfg(all(feature = "has-lp-core", feature="esp32c6"))]
use esp_hal::{gpio::{lp_io::{LowPowerInput, LowPowerOutput, LowPowerOutputOpenDrain}, InputPin, OutputPin, RtcPin}};
#[cfg(all(feature = "has-lp-core", feature="esp32s3"))]
use esp_hal::{gpio::{rtc_io::{LowPowerInput, LowPowerOutput, LowPowerOutputOpenDrain}, InputPin, OutputPin, RtcPin}};

#[cfg(feature = "is-lp-core")]
use esp_lp_hal::gpio::{Input, Output};
use crate::{EspCoproError, movableobject::MovableObject};

/// A wrapper for low-power GPIO input pin that can be used in the LP core.
/// This struct is a movable object and can be transferred between the main core and the LP core.
#[cfg(feature = "has-lp-core")]
pub struct LPInput<'d, const PIN: u8> {
    inner: LowPowerInput<'d, PIN>
}
/// A wrapper for low-power GPIO input pin that can be used in the LP core.
/// This struct is a movable object and can be transferred between the main core and the LP core.
#[cfg(feature = "is-lp-core")]
pub struct LPInput<'d, const PIN: u8> {
    inner: Input<PIN>,
    phantom: core::marker::PhantomData<&'d ()>
}

#[cfg(feature = "has-lp-core")]
impl<'d, const PIN: u8> LPInput<'d, PIN> {
    /// Create a new low-power input pin.
    pub fn new<P>(pin: P) -> Self where P: InputPin + RtcPin + 'd {
        Self { inner: LowPowerInput::new(pin) }
    }

    /// Sets pull-up enable for the pin.
    pub fn pullup_enable(&self, enable: bool) {
        self.inner.pullup_enable(enable);
    }

    /// Sets pull-down enable for the pin.
    pub fn pulldown_enable(&self, enable: bool) {
        self.inner.pulldown_enable(enable);
    }
}

#[cfg(feature = "is-lp-core")]
impl<'d, const PIN: u8> LPInput<'d, PIN> {
    pub fn level(&self) -> bool {
        self.inner.input_state()
    }
}

impl<'d, const PIN: u8> MovableObject for LPInput<'d, PIN> {
    unsafe fn move_to_main(&self, _dest : *mut u8) -> Result<(), EspCoproError> {
        Ok(()) // LPInput is ZST.
    }
    unsafe fn move_to_lp(&self, _dest : *mut u8) -> Result<(), EspCoproError> {
        Ok(()) // LPInput is ZST.
    }
}

/// A wrapper for low-power GPIO output pin that can be used in the LP core.
/// This struct is a movable object and can be transferred between the main core and the LP core.
#[cfg(feature = "has-lp-core")]
pub struct LPOutput<'d, const PIN: u8>  {
    inner: LowPowerOutput<'d, PIN>
}
/// A wrapper for low-power GPIO output pin that can be used in the LP core.
/// This struct is a movable object and can be transferred between the main core and the LP core.
#[cfg(feature = "is-lp-core")]
pub struct LPOutput<'d, const PIN: u8> {
    inner: Output<PIN>,
    phantom: core::marker::PhantomData<&'d ()>
}

#[cfg(feature = "has-lp-core")]
impl<'d, const PIN: u8> LPOutput<'d, PIN> {
    /// Create a new low-power output pin.
    pub fn new<P>(pin: P) -> Self where P: OutputPin + RtcPin + 'd {
        Self { inner: LowPowerOutput::new(pin) }
    }
}

#[cfg(feature = "is-lp-core")]
impl<'d, const PIN: u8> LPOutput<'d, PIN> {
    /// Set the output level of the pin.
    pub fn set_level(&mut self, level : bool) {
        self.inner.set_output(level);
    }

    /// Get the output level of the pin.
    pub fn get_level(&mut self) -> bool {
        self.inner.output_state()
    }

    /// Toggle the output level of the pin.
    pub fn toggle(&mut self) {
        let lv = self.get_level();
        self.set_level(!lv);
    }
}

impl<'d, const PIN: u8> MovableObject for LPOutput<'d, PIN> {
    unsafe fn move_to_main(&self, _dest : *mut u8) -> Result<(), EspCoproError> {
        Ok(()) // LPOutput is ZST.
    }
    unsafe fn move_to_lp(&self, _dest : *mut u8) -> Result<(), EspCoproError> {
        Ok(()) // LPOutput is ZST.
    }
}

/// A wrapper for low-power GPIO open-drain output pin that can be used in the LP core.
/// This struct is a movable object and can be transferred between the main core and the LP core.
#[cfg(feature = "has-lp-core")]
pub struct LPOutputOpenDrain<'d, const PIN: u8> {
    inner: LowPowerOutputOpenDrain<'d, PIN>
}

#[cfg(feature = "has-lp-core")]
impl<'d, const PIN: u8> LPOutputOpenDrain<'d, PIN> {
    /// Create a new low-power output pin.
    pub fn new<P>(pin: P) -> Self where P: InputPin + OutputPin + RtcPin + 'd {
        Self { inner: LowPowerOutputOpenDrain::new(pin) }
    }

    /// Sets pull-up enable for the pin.
    pub fn pullup_enable(&self, enable: bool) {
        self.inner.pullup_enable(enable);
    }

    /// Sets pull-down enable for the pin.
    pub fn pulldown_enable(&self, enable: bool) {
        self.inner.pulldown_enable(enable);
    }
}

// ESP-HAL 1.0.0 does not provide open-drain output pin.
// stay tuned...

#[cfg(feature = "has-lp-core")]
impl<'d, const PIN: u8> MovableObject for LPOutputOpenDrain<'d, PIN> {
    unsafe fn move_to_main(&self, _dest : *mut u8) -> Result<(), EspCoproError> {
        Ok(()) // LPOutputOpenDrain is ZST.
    }
    unsafe fn move_to_lp(&self, _dest : *mut u8) -> Result<(), EspCoproError> {
        Ok(()) // LPOutputOpenDrain is ZST.
    }
}