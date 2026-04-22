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