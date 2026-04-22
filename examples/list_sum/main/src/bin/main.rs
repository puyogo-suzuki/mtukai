#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

use esp_alloc as _;
use esp_hal::rtc_cntl::Rtc;

use list_sum_shared::{MainLPParcel, SimpleList};

#[cfg(feature="esp32c6")]
use esp_hal::lp_core::{LpCore, LpCoreWakeupSource};
#[cfg(feature = "esp32s3")]
use esp_hal::ulp_core::{UlpCore, UlpCoreWakeupSource};
use esp_rs_copro::lpbox::LPBox;
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

fn gen_list() -> (SimpleList, i32) {
    fn go(val : i32) -> (SimpleList, i32) {
        let mut sl = SimpleList::new(1, None);
        for i in 2..val {
            sl = SimpleList::new(i, Some(LPBox::new(sl)));
        }
        let sum = sl.sum();
        (sl, sum)
    }
    go(10)
}
fn print_list(list : &SimpleList) {
    print!("list: ");
    let mut current = list;
    loop {
        print!("{} ", current.value);
        match &current.next {
            Some(next) => current = next,
            None => break
        }
    }
    println!();
}

#[esp_hal::main]
fn main() -> ! {
    // generator version: 0.5.0
    esp_alloc::heap_allocator!(size: 72 * 1024);
    esp_println::logger::init_logger_from_env();
    let delay = esp_hal::delay::Delay::new();
    let peripherals = esp_hal::init(esp_hal::Config::default());
    
    #[cfg(feature = "esp32c6")]
    let mut lp_core = LpCore::new(peripherals.LP_CORE);
    #[cfg(feature = "esp32s3")]
    let mut lp_core = UlpCore::new(peripherals.ULP_RISCV_CORE);
    lp_core.stop();
    println!("lp core stopped");

    // load code to LP core
    #[cfg(feature = "esp32c6")]
    let lp_core_code = load_lp_code2!(
        "../lp/target/riscv32imac-unknown-none-elf/release/list-sum-lp"
    );
    #[cfg(feature = "esp32s3")]
    let lp_core_code = load_lp_code2!(
        "../lp/target/riscv32imc-unknown-none-elf/release/list-sum-lp"
    );
    {
        let (list, expected_sum) = gen_list();
        print_list(&list);
        let mut parcel = MainLPParcel {
            data : LPBox::new(list),
            result : 0
        };
        println!("lpcore run");
        #[cfg(feature = "esp32c6")]
        let wakeupsource = LpCoreWakeupSource::HpCpu;
        #[cfg(feature = "esp32s3")]
        let wakeupsource = UlpCoreWakeupSource::Timer(esp_hal::ulp_core::UlpCoreTimerCycles::new(200));
        delay.delay_millis(1000); // FOR ESP32-S3 because the UART stuck after the HP core wake up without the delay.
        if let Err(e) = lp_core_code.run_light_sleep(&mut lp_core, wakeupsource, &mut Rtc::new(peripherals.LPWR), &mut parcel) {
            println!("Error running LP core: {}", e);
        }
        println!("result: {} (expected: {})", parcel.result, expected_sum);
        print_list(&parcel.data);
        println!("result: {} (expected: {})", parcel.data.sum(), expected_sum + 10000)
    }
    loop {}
}