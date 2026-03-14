#[cfg(all(feature = "is-lp-core", feature = "esp32c6"))]
pub fn lp_core_halt() -> ! {
    use esp_lp_hal::pac;

    unsafe { &*pac::PMU::PTR }
        .lp_cpu_pwr1().write(|x| { x.lp_cpu_sleep_req().set_bit() });
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