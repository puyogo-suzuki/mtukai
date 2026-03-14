#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

use embedded_hal::{delay::DelayNs, digital::OutputPin};
use esp_lp_hal::{delay::Delay, gpio::Output, prelude::*, wake_hp_core};
use esp_rs_copro::{io::i2c::{LPI2C, LPI2CError}, prelude::lp_core_halt};
use pedometer_shared::{MainLPParcel, Vector3D};
use panic_halt as _;

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

fn adxl367_softreset(i2c: &mut LPI2C) -> Result<(), LPI2CError> {
    let addr = 0x53;
    i2c.write(addr, &[0x1F, 0x52])
    // Delay.delay_ms(1000);
    // i2c.write(addr, &[0x2C, 0x13])
}

fn adxl367_enable(i2c: &mut LPI2C, enable: bool) -> Result<(), LPI2CError> {
    let addr = 0x53;
    let cmd = [0x2D, if enable { 0x02 } else { 0x00 }];
    i2c.write(addr, &cmd)
}

fn adxl367_read(i2c: &mut LPI2C) -> Result<Vector3D, LPI2CError> {
    let addr = 0x53;
    let mut buffer = [0u8; 6];
    i2c.write(addr, &[0x33])?;
    i2c.read(addr, &mut buffer)?;
    let x = ((buffer[1] as i16) << 8) | (buffer[0] as i16);
    let y = ((buffer[3] as i16) << 8) | (buffer[2] as i16);
    let z = ((buffer[5] as i16) << 8) | (buffer[4] as i16);
    Ok(Vector3D::new(x, y, z))
}

#[entry]
fn main(mut gpio5: Output<5>) -> ! {
    let v: &mut MainLPParcel = get_transfer::<MainLPParcel>().unwrap();
    adxl367_softreset(&mut v.i2c).unwrap();
    // Delay.delay_ms(100);
    // adxl367_enable(&mut v.i2c, true).unwrap();
    // Delay.delay_ms(10);
    // for i in 0..v.measurement_count {
    //     match adxl367_read(&mut v.i2c) {
    //         Ok(value) => v.result.push(Some(value)),
    //         Err(e) => v.result.push(Some(Vector3D::new(e as i16, -1, -1))),
    //     }
    //     Delay.delay_ms(1000);
    // }
    Delay.delay_ms(10000);
    wake_hp_core();
    lp_core_halt()
}