#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

use esp_alloc as _;
use esp_hal::clock::CpuClock;
use esp_hal::gpio::lp_io::LowPowerOutputOpenDrain;
use esp_hal::i2c::lp_i2c::LpI2c;
use esp_hal::peripherals::{LP_IO};
use esp_hal::rtc_cntl::Rtc;
use esp_hal::time::Rate;
use esp_hal::delay::Delay;

use esp_rs_copro::{io::i2c::LPI2C, collections::lpvec::LPVec};
use shared::MainLPParcel;

use esp_hal::{
    gpio::lp_io::LowPowerOutput,
    lp_core::{LpCore, LpCoreWakeupSource},
};

use esp_rs_copro_procmacro::{define_lp_allocator, load_lp_code2};

use esp_println::{print, println};

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

define_lp_allocator!();

fn sht30_main() -> !{
    let peripherals = esp_hal::init(esp_hal::Config::default());
    // configure GPIO 1 as LP output pin
    let lp_pin = LowPowerOutput::new(peripherals.GPIO5);

    let mut lp_core = LpCore::new(peripherals.LP_CORE);
    lp_core.stop();
    println!("lp core stopped");
    // load code to LP core
    let lp_core_code = load_lp_code2!(
        "../lp/target/riscv32imac-unknown-none-elf/release/esp-rs-copro-lp"
    );
    {
        // let i2c = LpI2c::new(peripherals.LP_I2C0, 
        //     LowPowerOutputOpenDrain::new(peripherals.GPIO6),
        //     LowPowerOutputOpenDrain::new(peripherals.GPIO7),
        //     Rate::from_khz(2));
        let gpio6 = LowPowerOutputOpenDrain::new(peripherals.GPIO6);
        let gpio7 = LowPowerOutputOpenDrain::new(peripherals.GPIO7);
        
        let i2c = LpI2c::new(
            peripherals.LP_I2C0,
            gpio6,
            gpio7,
            Rate::from_khz(2));

        // disable pull-up.
        LP_IO::regs().gpio(6).modify(|_, w| w.fun_wpu().clear_bit());
        LP_IO::regs().gpio(7).modify(|_, w| w.fun_wpu().clear_bit());

        let mut parcel = MainLPParcel {
            i2c : LPI2C::new(i2c),
            result : LPVec::new(),
            measurement_count : 5
        };
        println!("lpcore run");
        if let Err(e) = lp_core_code.run_light_sleep(&mut lp_core, LpCoreWakeupSource::HpCpu, &mut Rtc::new(peripherals.LPWR), &mut parcel, lp_pin) {
            println!("Error running LP core: {}", e);
        }
        for i in parcel.result.iter() {
            match i {
                Some(i) => {
                    println!("Temp: {} C, Hum: {} %", i.temperature, i.humidity);
                },
                None => {
                    println!("Failed to read from sensor");
                }
            }
        }
    }
    loop {}
}

fn refresh_gpio() -> ! {
    
    let peripherals = esp_hal::init(esp_hal::Config::default());
    // configure GPIO 1 as LP output pin
    let pin6: LowPowerOutput<'_, 6> = LowPowerOutput::new(peripherals.GPIO6);
    let pin7: LowPowerOutput<'_, 7> = LowPowerOutput::new(peripherals.GPIO7);
    let lp_io = LP_IO::regs();
    let delay = Delay::new();
    loop { unsafe {
        lp_io
            .out_data_w1ts()
            .write(|w| w.out_data_w1ts().bits(1 << 6 | 1 << 7));
        delay.delay_millis(500);
        lp_io
            .out_data_w1tc()
            .write(|w| w.out_data_w1tc().bits(1 << 6 | 1 << 7));
        delay.delay_millis(500);
    }}
}
#[esp_hal::main]
fn main() -> ! {
    // generator version: 0.5.0
    esp_alloc::heap_allocator!(size: 72 * 1024);
    esp_println::logger::init_logger_from_env();
    // let delay = Delay::new();
    sht30_main();
    // refresh_gpio();
    // start LP core
}