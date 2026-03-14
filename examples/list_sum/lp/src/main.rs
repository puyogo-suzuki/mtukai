#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

use embedded_hal::delay::DelayNs;
use esp_lp_hal::{delay::Delay, prelude::*};
#[cfg(feature = "esp32c6")]
use esp_lp_hal::wake_hp_core;
use esp_rs_copro::{prelude::lp_core_halt};
use list_sum_shared::{MainLPParcel};
use panic_halt as _;

esp_rs_copro_procmacro::esp_rs_copro_statics!(4096);
#[alloc_error_handler]
fn ignore_alloc_error(_: core::alloc::Layout) -> ! {
    loop{}
}

#[entry]
fn main() -> ! {
    let v: &mut MainLPParcel = get_transfer::<MainLPParcel>().unwrap();
    v.result = v.data.sum();
    v.data.push(10000);
    Delay.delay_ms(100);
    #[cfg(feature = "esp32c6")]
    wake_hp_core();
    #[cfg(feature = "esp32s3")]
    {
        use esp_lp_hal::pac;
        let rtc_cntl_rtc_state0_reg = pac::RTC_CNTL::PTR.addr() + 0x18;
        let ptr = rtc_cntl_rtc_state0_reg as *mut u32;
        unsafe { ptr.write_volatile(0b1) }
    }
    lp_core_halt()
}