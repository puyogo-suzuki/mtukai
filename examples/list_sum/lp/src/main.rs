#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

use embedded_hal::{delay::DelayNs, digital::OutputPin};
use esp_lp_hal::{delay::Delay, prelude::*, gpio::Output};
use esp_rs_copro::{prelude::*};
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
    Delay.delay_ms(1000);
    wake_hp_core();
    lp_core_halt()
}