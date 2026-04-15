// ORIGINAL Source: ESP-HAL 
// ESP-HAL v1.0 does not implement the ULP wakeup source.
// These code will be removed after the future version of ESP-HAL is released.

#[cfg(all(feature = "is-lp-core", feature = "esp32c6"))]
pub fn lp_core_halt() -> ! {
    use esp_lp_hal::pac;

    unsafe { &*pac::PMU::PTR }
        .lp_cpu_pwr1().modify(|_, w| { w.lp_cpu_sleep_req().set_bit(); unsafe{ w.lp_cpu_wakeup_en().bits(0) } });
    loop{}
}
#[cfg(all(feature = "is-lp-core", feature = "esp32s3"))]
pub fn lp_core_halt() -> ! {
    use esp_lp_hal::pac;
    use core::arch::asm;
    unsafe { &*pac::RTC_CNTL::PTR }
        .cocpu_ctrl()
        .modify(|_, w| unsafe { w.cocpu_shut_2_clk_dis().bits(0x3f).cocpu_done().set_bit() });

    loop {
        unsafe { asm!("wfi"); }
    }
}

#[cfg(feature = "is-lp-core")]
pub fn wake_hp_core() {
    #[cfg(feature = "esp32c6")]
    esp_lp_hal::wake_hp_core();

    #[cfg(feature = "esp32s3")]
    {
        let rtc_cntl_rtc_state0_reg = esp_lp_hal::pac::RTC_CNTL::PTR.addr() + 0x18;
        let ptr = rtc_cntl_rtc_state0_reg as *mut u32;
        unsafe { ptr.write_volatile(ptr.read_volatile() | 1) }
    }
}

#[cfg(all(feature = "has-lp-core", feature = "esp32s3"))]
use esp_hal::rtc_cntl::{Rtc, sleep::{WakeSource, WakeTriggers, RtcSleepConfig}};

/// ULP wakeup source
///
/// Wake up from ULP software interrupt, and/or ULP-RISCV Trap condition.
/// Both of these triggers are enabled by default.
/// This source will clear any outstanding software interrupts prior to entering sleep, by default.
///
/// S2 supports the following triggers (Refer to ESP32-S2 Technical Reference Manual, Table 9.4-3.
/// Wakeup Source)
///  - ULP-FSM software interrupt (unsure if this ALSO supports ULP-RISCV software interrupt)
///  - ULP-RISCV Trap
///
/// S3 supports the following triggers (Refer to ESP32-S3 Technical Reference Manual, Table 10.4-3.
/// Wakeup Source)
///  - ULP-FSM software interrupt and ULP-RISCV software interrupt
///  - ULP-RISCV Trap
///
/// This wakeup source can be used to wake up from both light and deep sleep.
#[cfg(all(feature = "has-lp-core", feature = "esp32s3"))]
pub struct UlpWakeupSource {
    wake_on_interrupt: bool,
    wake_on_trap: bool,
    clear_interrupts_on_sleep: bool,
}

#[cfg(all(feature = "has-lp-core", feature = "esp32s3"))]
impl UlpWakeupSource {
    /// Create a new instance of `WakeFromUlpWakeupSource`
    pub const fn new() -> Self {
        Self {
            wake_on_interrupt: true,
            wake_on_trap: true,
            clear_interrupts_on_sleep: true,
        }
    }

    /// Enable wakeup triggered by software interrupt from ULP-FSM or ULP-RISCV
    pub fn set_wake_on_interrupt(mut self, value: bool) -> Self {
        self.wake_on_interrupt = value;
        self
    }

    /// Enable wakeup triggered by ULP-RISCV Trap
    pub fn set_wake_on_trap(mut self, value: bool) -> Self {
        self.wake_on_trap = value;
        self
    }

    /// Enable clearing of latched wake-up interrupts prior to entering sleep
    pub fn set_clear_interrupts_on_sleep(mut self, value: bool) -> Self {
        self.clear_interrupts_on_sleep = value;
        self
    }

    /// Clears the wake-up interrupts
    pub fn clear_interrupts(&self) {
        esp_hal::peripherals::LPWR::regs().int_clr().write(|w| {
            w.cocpu_trap().clear_bit_by_one();
            w.cocpu().clear_bit_by_one();
            w.ulp_cp().clear_bit_by_one()
        });
    }
}

#[cfg(all(feature = "has-lp-core", feature = "esp32s3"))]
impl Default for UlpWakeupSource {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(all(feature = "has-lp-core", feature = "esp32s3"))]
impl WakeSource for UlpWakeupSource {
    fn apply(
        &self,
        _rtc: &Rtc<'_>,
        triggers: &mut WakeTriggers,
        sleep_config: &mut RtcSleepConfig,
    ) {
        // Currently we only consider interruptions from RISC-V ULP Coprocessor.
        // triggers.set_ulp_fsm(self.wake_on_interrupt);
        triggers.set_ulp(self.wake_on_interrupt);
        // triggers.set_ulp_riscv(self.wake_on_interrupt);
        triggers.0 = if self.wake_on_interrupt { triggers.0 | (1 << 11) } else { triggers.0 & !(1 << 11) };
        // triggers.set_ulp_riscv_trap(self.wake_on_trap);
        triggers.0 = if self.wake_on_trap { triggers.0 | (1 << 13) } else { triggers.0 & !(1 << 13) };
        if self.clear_interrupts_on_sleep {
            self.clear_interrupts();
        }

        // This one needs to be false to keep the ULP timer and ULP GPIO happy!
        // Possibly relevant issue: https://github.com/espressif/esp-idf/issues/10595
        sleep_config.set_rtc_peri_pd_en(false);
    }
}