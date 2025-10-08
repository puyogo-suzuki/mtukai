#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

pub mod lpalloc;

use embedded_hal::{delay::DelayNs, digital::OutputPin};
use esp_lp_hal::{delay::Delay, gpio::Output, prelude::*};

use panic_halt as _;

use lpalloc::LPAllocator;
#[global_allocator]
static ALLOCATOR: LPAllocator<4096> = LPAllocator::new();

const ADDRESS: u32 = 0x5000_2000;

// cfg_if::cfg_if! {
//     if #[cfg(feature = "esp32c6")] {
//         const ADDRESS: u32 = 0x5000_2000;
//     } else if #[cfg(any(feature = "esp32s2", feature = "esp32s3"))] {
//         const ADDRESS: u32 = 0x400;
//     }
// }

#[entry]
fn main(mut gpio1: Output<6>) -> ! {
    let mut i: u32 = 0;

    let ptr = ADDRESS as *mut u32;

    loop {
        i = i.wrapping_add(1u32);
        unsafe {
            ptr.write_volatile(i);
        }

        gpio1.set_high().unwrap();
        Delay.delay_ms(500);

        gpio1.set_low().unwrap();
        Delay.delay_ms(500);
    }
}

