#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

use esp_lp_hal::{
    delay::Delay,
    prelude::*
};
use esp_rs_copro::{
    io::gpio::LPOutput,
};
use panic_halt as _;
use music_shared::{MainLPParcel, Note};
use core::arch::asm;

esp_rs_copro_procmacro::esp_rs_copro_statics!(4096);
#[alloc_error_handler]
fn ignore_alloc_error(_: core::alloc::Layout) -> ! {
    loop { unsafe { asm!("wfi"); } }
}

#[inline(always)]
fn delay_us(us: u32) {
    Delay.delay_micros(us - 64);
}

fn play_note<const PIN: u8>(outpin : &mut LPOutput<PIN>, note : &Note) {
    let d = (note.duration as u32) * 1000;
    if note.frequency == 0 {
        delay_us(d);
    } else {
        let period = 1000000 / note.frequency as u32;
        for _ in 0..(d / period) {
            outpin.set_level(true);
            delay_us(period / 2);
            outpin.set_level(false);
            delay_us(period / 2);
        }
    }
}

#[entry]
fn main() -> ! {
    let v: &mut MainLPParcel = get_transfer::<MainLPParcel>().unwrap();
    loop{
        v.score.iter().for_each(|note| play_note(&mut v.outpin, note));
    }
}
