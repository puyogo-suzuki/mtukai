#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

use embedded_hal::{delay::DelayNs, digital::OutputPin};
use esp_lp_hal::{delay::Delay, gpio::Output, prelude::*, wake_hp_core};
use esp_rs_copro::{io::i2c::{LPI2C, LPI2CError}, prelude::lp_core_halt};
use shared::{MainLPParcel, TempAndHumid};
use panic_halt as _;
use core::mem::MaybeUninit;

esp_rs_copro_procmacro::esp_rs_copro_statics!(4096);
#[alloc_error_handler]
fn ignore_alloc_error(_: core::alloc::Layout) -> ! {
    loop{}
}

// cfg_if::cfg_if! {
//     if #[cfg(feature = "esp32c6")] {
//         const ADDRESS: u32 = 0x5000_2000;
//     } else if #[cfg(any(feature = "esp32s2", feature = "esp32s3"))] {
//         const ADDRESS: u32 = 0x400;
//     }
// }

fn read_sht30(me : &mut LPI2C) -> Result<TempAndHumid, LPI2CError> {
    let addr = 0x44;
    let cmd = [0x2C, 0x06];
    let mut buffer = [0u8; 6];
    me.write(addr, &cmd)?;
    Delay.delay_millis(10);
    me.read(addr, &mut buffer)?;
    let temp_raw = ((buffer[0] as u16) << 8) | (buffer[1] as u16);
    let hum_raw = ((buffer[3] as u16) << 8) | (buffer[4] as u16);
    let temperature = -4500 + ((17500 * (temp_raw as i32)) / 65535);
    let humidity = (10000 * (hum_raw as i32)) / 65535;
    Ok(TempAndHumid::new(temperature/100, humidity/100))
}


fn sht30_main() -> !{
    let v: &mut MainLPParcel = get_transfer::<MainLPParcel>().unwrap();
    for i in 0..v.measurement_count {
        v.result.push(read_sht30(&mut v.i2c).ok());
        Delay.delay_ms(1000);
    }
    wake_hp_core();
    lp_core_halt()
}

#[entry]
fn main(mut gpio5: Output<5>) -> ! {
    sht30_main();
}