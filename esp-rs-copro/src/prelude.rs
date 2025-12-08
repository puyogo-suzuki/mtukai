#[cfg(any(feature = "is-lp-core", test))]
pub fn lp_core_halt() -> ! {
    use esp_lp_hal::pac;

    unsafe { &*pac::PMU::PTR }
        .lp_cpu_pwr1().write(|x| { x.lp_cpu_sleep_req().set_bit() });
    loop{}
}