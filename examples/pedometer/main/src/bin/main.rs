#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

use esp_alloc as _;
use esp_hal::gpio::lp_io::LowPowerOutputOpenDrain;
use esp_hal::i2c::lp_i2c::LpI2c;
use esp_hal::peripherals::{LP_IO};
use esp_hal::rtc_cntl::Rtc;
use esp_hal::time::Rate;
use esp_hal::delay::Delay;
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
    let gpio7 = LowPowerOutputOpenDrain::new(peripherals.GPIO7);
    let gpio6 = LowPowerOutputOpenDrain::new(peripherals.GPIO6);
    unsafe { // This is a workaround for ESP-HAL 1.0.0, which generates an unexpected start bit.
        LP_IO::regs()
            .out_data_w1ts()
            .write(|w| w.out_data_w1ts().bits(1 << 6 | 1 << 7));
    }
    let i2c = LpI2c::new(
        peripherals.LP_I2C0,
        gpio6,
        gpio7,
        Rate::from_khz(20));

    let mut parcel = MainLPParcel {
        button : LPInput::new(peripherals.GPIO0),
        i2c : LPI2C::new(i2c),
        steps : 0
    };
    let mut rtc = Rtc::new(peripherals.LPWR);
    loop {
        if let Err(e) = lp_core_code.run_light_sleep(&mut lp_core, LpCoreWakeupSource::HpCpu, &mut rtc, &mut parcel) {
            println!("Error running LP core: {}", e);
            loop {}
        } else {
            println!("Current steps: {}", parcel.steps);
        }
    }
}