#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

use esp_alloc as _;
use esp_hal::clock::CpuClock;
use esp_hal::main;
use esp_hal::time::{Duration, Instant};
use esp_hal::delay::Delay;

use esp_rs_copro::lpbox::LPBox;
use shared::TestList;

use esp_hal::{
    gpio::lp_io::LowPowerOutput,
    lp_core::{LpCore, LpCoreWakeupSource},
};

use esp_rs_copro_procmacro::load_lp_code2;

use esp_println::{print, println};

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

fn test_listcreate() -> TestList {
    TestList {
        next: Some(LPBox::new(TestList {
            next: Some(LPBox::new(TestList {
                next: None,
                value: 7
            })),
            value: 21
        })),
        value: 42
    }
}

fn test_listprint(tl : &TestList) {
    let mut current = Some(tl);
    println!("Printing TestList.");
    while let Some(node) = current {
        println!("Addr: {:p} Value: {}", node as *const _ as *const (), node.value);
        current = node.next.as_deref();
    }

}

#[main]
fn main() -> ! {
    // generator version: 0.5.0
    esp_alloc::heap_allocator!(size: 72 * 1024);
    esp_println::logger::init_logger_from_env();
    let peripherals = esp_hal::init(esp_hal::Config::default());
    // let delay = Delay::new();

    // configure GPIO 1 as LP output pin
    let lp_pin = LowPowerOutput::new(peripherals.GPIO6);

    let mut lp_core = LpCore::new(peripherals.LP_CORE);
    lp_core.stop();
    println!("lp core stopped");
    // load code to LP core
    let lp_core_code = load_lp_code2!(
        "../lp/target/riscv32imac-unknown-none-elf/release/esp-rs-copro-lp"
    );

    // start LP core
    {
        let mut test_list = test_listcreate();
        test_listprint(&test_list);
        lp_core_code.run(&mut lp_core, LpCoreWakeupSource::HpCpu, &mut test_list, lp_pin);
        println!("lpcore run!");

        let data = (0x5000_2000) as *mut u32;
        print!("Result {:x}           \u{000d}", unsafe {
            data.read_volatile()
        });
        test_listprint(&test_list);
    }
    loop {}
    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.0.0-rc.0/examples/src/bin
}
