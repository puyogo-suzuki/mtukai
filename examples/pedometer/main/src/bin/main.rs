#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

use esp_alloc as _;
use esp_hal::i2c::lp_i2c::{LpI2c, Config};
use esp_hal::rtc_cntl::sleep::LowPower;
use esp_hal::time::Rate;
use esp_hal::lp_core::{LpCore, LpCoreWakeupSource};

use esp_rs_copro::io::{i2c::LPI2C, gpio::LPInput};
use pedometer_shared::MainLPParcel;

use esp_rs_copro_procmacro::{define_lp_allocator, load_lp_code2};
use esp_println::println;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

define_lp_allocator!();

#[esp_hal::main]
fn main() -> ! {
    // generator version: 0.5.0
    esp_alloc::heap_allocator!(size: 72 * 1024);
    esp_println::logger::init_logger_from_env();

    let peripherals = esp_hal::init(esp_hal::Config::default());

    let mut lp_core = LpCore::new(peripherals.LP_CORE);
    lp_core.stop();
    // load code to LP core
    let lp_core_code = load_lp_code2!(
        "../lp/target/riscv32imac-unknown-none-elf/release/pedometer-lp"
    );
    let i2c = if let Ok(i2c) = LpI2c::new(
        peripherals.LP_I2C0,
        Config::default().with_frequency(Rate::from_khz(2)),
        peripherals.GPIO6,
        peripherals.GPIO7) {
        i2c
    } else {
        panic!("Failed to create LP I2C");
    };

    let mut parcel = MainLPParcel {
        button : LPInput::new(peripherals.GPIO0),
        i2c : LPI2C::new(i2c),
        steps : 0
    };
    let mut lwpw = LowPower::new(peripherals.LPWR);
    loop {
        if let Err(e) = lp_core_code.run_light_sleep(&mut lp_core, LpCoreWakeupSource::HpCpu, &mut lwpw, &mut parcel) {
            println!("Error running LP core: {}", e);
            loop {}
        } else {
            println!("Current steps: {}", parcel.steps);
        }
    }
}