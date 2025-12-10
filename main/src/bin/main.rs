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

use esp_rs_copro::io::i2c::LPI2C;
use esp_rs_copro::lpbox::{LPBox, new_array_uninitialized};
use shared::{MainLPParcel, TestList};

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

fn testlist_main() -> !{
    // configure GPIO 1 as LP output pin
    let peripherals: esp_hal::peripherals::Peripherals = esp_hal::init(esp_hal::Config::default());
    let lp_pin = LowPowerOutput::new(peripherals.GPIO5);

    let mut lp_core = LpCore::new(peripherals.LP_CORE);
    lp_core.stop();
    println!("lp core stopped");
    // load code to LP core
    let lp_core_code = load_lp_code2!(
        "../lp/target/riscv32imac-unknown-none-elf/release/esp-rs-copro-lp"
    );
    {
        let mut test_list = test_listcreate();
        test_listprint(&test_list);
        println!("lpcore run!");
        lp_core_code.run_light_sleep(&mut lp_core, LpCoreWakeupSource::HpCpu, &mut Rtc::new(peripherals.LPWR), &mut test_list, lp_pin);
        test_listprint(&test_list);
    }
    loop {}
}

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
            result : new_array_uninitialized(10)
        };
        println!("lpcore run");
        lp_core_code.run_light_sleep(&mut lp_core, LpCoreWakeupSource::HpCpu, &mut Rtc::new(peripherals.LPWR), &mut parcel, lp_pin);
        for i in parcel.result.iter() {
            println!("Temp: {} C, Hum: {} %", i.temperature, i.humidity);
        }
    }
    loop {}
}

// fn read_sht30<'a, b : DriverMode>(me : &mut I2c<'a, b>) -> Result<TempAndHumid, esp_hal::i2c::master::Error> {
//     let delay = Delay::new();
//     let addr = 0x44;
//     let cmd = [0x2C, 0x06];
//     let mut buffer = [0u8; 6];
//     me.write(addr, &cmd)?;
//     delay.delay_millis(10);
//     me.read(addr, &mut buffer)?;
//     let temp_raw = ((buffer[0] as u16) << 8) | (buffer[1] as u16);
//     let hum_raw = ((buffer[3] as u16) << 8) | (buffer[4] as u16);
//     let temperature = -4500 + ((17500 * (temp_raw as i32)) / 65535);
//     let humidity = (10000 * (hum_raw as i32)) / 65535;
//     Ok(TempAndHumid::new(temperature/100, humidity/100))
// }

// fn sht30_main_main() -> !{
//     let peripherals = esp_hal::init(esp_hal::Config::default());
//     let mut i2c = I2c::new(peripherals.I2C0, Config::default().with_frequency(Rate::from_khz(2)))
//         .unwrap()
//         .with_scl(peripherals.GPIO7)
//         .with_sda(peripherals.GPIO6);
//     Into::<esp_hal::gpio::interconnect::OutputSignal<'_>>::into(unsafe{GPIO7::steal()}).apply_output_config(
//             &esp_hal::gpio::OutputConfig::default()
//                 .with_drive_mode(esp_hal::gpio::DriveMode::OpenDrain)
//                 .with_pull(esp_hal::gpio::Pull::None),
//         );
//     Into::<esp_hal::gpio::interconnect::OutputSignal<'_>>::into(unsafe{peripherals::GPIO6::steal()}).apply_output_config(
//             &esp_hal::gpio::OutputConfig::default()
//                 .with_drive_mode(esp_hal::gpio::DriveMode::OpenDrain)
//                 .with_pull(esp_hal::gpio::Pull::None),
//         );
//     let val = (0..10).map(|_| {
//         read_sht30(&mut i2c).inspect_err(|e| { println!("{:?}", e)}).unwrap_or(TempAndHumid { temperature: -999, humidity: -999 })
//     });
//     for i in val {
//         println!("Temp: {} C, Hum: {} %", i.temperature, i.humidity);
//     }
//     loop {}
// }

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

#[main]
fn main() -> ! {
    // generator version: 0.5.0
    esp_alloc::heap_allocator!(size: 72 * 1024);
    esp_println::logger::init_logger_from_env();
    // let delay = Delay::new();
    sht30_main();
    // sht30_main_main();
    // refresh_gpio();
    // start LP core
    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.0.0-rc.0/examples/src/bin
}