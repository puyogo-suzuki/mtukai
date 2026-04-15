#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

use esp_alloc as _;
use esp_hal::{
    rtc_cntl::Rtc,
    gpio::lp_io::LowPowerOutput,
    lp_core::{LpCore, LpCoreWakeupSource},
};
use esp_println::println;

use esp_rs_copro::{io::gpio::LPOutput, collections::lpvec::LPVec};
use esp_rs_copro_procmacro::{define_lp_allocator, load_lp_code2};
use music_shared::{MainLPParcel, Note};

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

define_lp_allocator!();

fn cde() -> LPVec<Note> {
    let mut ret = LPVec::new();
    ret.push(Note::c4(500));
    ret.push(Note::d4(500));
    ret.push(Note::e4(500));
    ret
}

#[esp_hal::main]
fn main() -> ! {
    esp_alloc::heap_allocator!(size: 72 * 1024);
    esp_println::logger::init_logger_from_env();

    let peripherals = esp_hal::init(esp_hal::Config::default());

    let mut lp_core = LpCore::new(peripherals.LP_CORE);
    lp_core.stop();
    // load code to LP core
    let lp_core_code = load_lp_code2!(
        "../lp/target/riscv32imac-unknown-none-elf/release/music-lp"
    );
    {
        // let _gpio1 : LowPowerOutput<'_, 1> = LowPowerOutput::new(peripherals.GPIO1);

        let mut parcel = MainLPParcel::<'_> {
            score: cde(),
            outpin: LPOutput::<1>::new(peripherals.GPIO1)
        };

        if let Err(e) = lp_core_code.run_light_sleep(&mut lp_core, LpCoreWakeupSource::HpCpu, &mut Rtc::new(peripherals.LPWR), &mut parcel) {
            println!("Error running LP core!: {}", e);
        }
    }
    loop {}
}