// ORIGINAL Source: ESP-HAL 
// ESP-HAL v1.0 does not implement the ULP wakeup source.
// These code will be removed after the future version of ESP-HAL is released.

#[cfg(all(feature = "is-lp-core"))]
pub fn lp_core_halt() -> ! {
    use esp_lp_hal::pac;

    unsafe { &*pac::PMU::PTR }
        .lp_cpu_pwr1().modify(|_, w| { w.lp_cpu_sleep_req().set_bit(); unsafe{ w.lp_cpu_wakeup_en().bits(0) } });
    loop{}
}