#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

use embedded_hal::{delay::DelayNs, digital::OutputPin};
use esp_lp_hal::{delay::Delay, gpio::Output, prelude::*};
use esp_rs_copro::TestList;
use panic_halt as _;

esp_rs_copro_procmacro::esp_rs_copro_statics!(4096);
#[alloc_error_handler]
fn ignore_alloc_error(_: core::alloc::Layout) -> ! {
    loop{}
}

const ADDRESS: u32 = 0x5000_2000;

// cfg_if::cfg_if! {
//     if #[cfg(feature = "esp32c6")] {
//         const ADDRESS: u32 = 0x5000_2000;
//     } else if #[cfg(any(feature = "esp32s2", feature = "esp32s3"))] {
//         const ADDRESS: u32 = 0x400;
//     }
// }

fn sum_testlist(testlist : &TestList) -> i32 {
    let mut sum = 0;
    let mut current = Some(testlist);
    while let Some(node) = current {
        sum += node.value;
        current = node.next.as_deref();
    }
    sum
}

fn double_testlist(testlist : &mut TestList) {
    let mut current = Some(testlist);
    while let Some(node) = current {
        node.value *= 2;
        current = node.next.as_deref_mut();
    }
}

#[entry]
fn main(mut gpio1: Output<6>) -> ! {
    let v = get_transfer::<TestList>().unwrap();
    let val = sum_testlist(&v);
    double_testlist(v);
    let ptr = ADDRESS as *mut u32;
    unsafe {
        ptr.write_volatile(val as u32);
    }
    loop {
        // i = i.wrapping_add(1u32);

        gpio1.set_high().unwrap();
        Delay.delay_ms(500);

        gpio1.set_low().unwrap();
        Delay.delay_ms(500);
    }
}

