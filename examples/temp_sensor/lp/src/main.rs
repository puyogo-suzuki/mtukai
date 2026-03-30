#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

use core::arch::global_asm;
use embedded_hal::delay::DelayNs;
use esp_lp_hal::{
    delay::Delay,
    prelude::*,
    wake_hp_core,
};
use esp_rs_copro::{
    io::i2c::{LPI2CError, LPI2C},
    prelude::lp_core_halt,
};
use panic_halt as _;
use temp_sensor_shared::{MainLPParcel, TempAndHumid};

esp_rs_copro_procmacro::esp_rs_copro_statics!(4096);
#[alloc_error_handler]
fn ignore_alloc_error(_: core::alloc::Layout) -> ! {
    loop { unsafe { asm!("wfi"); } }
}

#[unsafe(no_mangle)]
static mut sp_bottom: usize = 0;
unsafe extern "C" {
    fn sleep_store();
    fn sleep_restore();
    fn ulp_lp_core_lp_timer_set_wakeup_ticks(ticks: u64);
    fn ulp_lp_core_lp_timer_intr_enable(en: bool);
}
#[unsafe(no_mangle)]
extern "C" fn sleep_store2() -> ! {
    lp_core_halt()
}

global_asm!("
.globl sleep_store
.extern sp_bottom
.extern sleep_store2
sleep_store:
    addi sp, sp, -64
    sw x1, 0(sp)
    sw x3, 4(sp)
    sw x4, 8(sp)
    sw x8, 16(sp)
    sw x9, 20(sp)
    sw x18, 24(sp)
    sw x19, 28(sp)
    sw x20, 32(sp)
    sw x21, 36(sp)
    sw x22, 49(sp)
    sw x23, 44(sp)
    sw x24, 48(sp)
    sw x25, 52(sp)
    sw x26, 56(sp)
    sw x27, 60(sp)
    la a0, sp_bottom
    sw sp, 0(a0)
    j sleep_store2

.globl sleep_restore
sleep_restore:
    la a0, sp_bottom
    lw sp, 0(a0)
    lw x1, 0(sp)
    lw x3, 4(sp)
    lw x4, 8(sp)
    lw x8, 16(sp)
    lw x9, 20(sp)
    lw x18, 24(sp)
    lw x19, 28(sp)
    lw x20, 32(sp)
    lw x21, 36(sp)
    lw x22, 49(sp)
    lw x23, 44(sp)
    lw x24, 48(sp)
    lw x25, 52(sp)
    lw x26, 56(sp)
    lw x27, 60(sp)
    addi sp, sp, 64
    jalr zero, ra

.globl ulp_lp_core_lp_timer_get_cycle_count
ulp_lp_core_lp_timer_get_cycle_count:
    li a5, 0x600b0c00
    lw	a4,16(a5)
    lui	a3,0x10000
    addi	sp,sp,-32
    or	a4,a4,a3
    sw	a4,16(a5)
    lw	a4,20(a5)
    sw	a4,28(sp)
    lw	a0,28(sp)
    lw	a5,24(a5)
    sw	a5,24(sp)
    lhu	a1,24(sp)
    addi	sp,sp,32
    ret

.globl lp_timer_hal_set_alarm_target
lp_timer_hal_set_alarm_target:
    li a5, 0x600b0c00
    lw	a4,68(a5)
    lui	a3,0x80000
    addi	sp,sp,-16
    or	a4,a4,a3
    sw	a4,68(a5)
    lw	a2,12(a5)
    slli	a1,a1,0x10
    srli	a1,a1,0x10
    sw	a2,8(sp)
    sh	a1,8(sp)
    lw	a2,8(sp)
    sw	a2,12(a5)
    lw	a2,8(a5)
    sw	a2,12(sp)
    sw	a0,12(sp)
    lw	a4,12(sp)
    sw	a4,8(a5)
    lw	a4,12(a5)
    or	a4,a4,a3
    sw	a4,12(a5)
    addi	sp,sp,16
    ret

.globl ulp_lp_core_lp_timer_set_wakeup_ticks
ulp_lp_core_lp_timer_set_wakeup_ticks:
    addi	sp,sp,-16
    sw	s0,8(sp)
    sw	s1,4(sp)
    sw	ra,12(sp)
    mv	s0,a0
    mv	s1,a1
    jal	ulp_lp_core_lp_timer_get_cycle_count
    mv	a5,a0
    add	a0,a0,s0
    lw	s0,8(sp)
    lw	ra,12(sp)
    add	a1,a1,s1
    lw	s1,4(sp)
    sltu	a5,a0,a5
    add	a1,a1,a5
    addi	sp,sp,16
    j	lp_timer_hal_set_alarm_target

.globl ulp_lp_core_lp_timer_intr_enable
ulp_lp_core_lp_timer_intr_enable:
    li a4, 0x600b0c00
    lw	a5,64(a4)
    slli	a0,a0,0x1f
    slli	a5,a5,0x1
    srli	a5,a5,0x1
    or	a5,a5,a0
    sw	a5,64(a4)
    ret
");

use core::arch::asm;

const MSTATUS_MIE: usize = 1 << 3;
const MIE_ALL_INTS_MASK: usize = 1 << 30;
fn delay_micros(us: u32) {
    // busy loop version
    if us < 50 {
        Delay.delay_us(us);
        return;
    }
    unsafe {
        ulp_lp_core_lp_timer_intr_enable(true);
        ulp_lp_core_lp_timer_set_wakeup_ticks((us as u64) * 17 / 125); // 136 kHz
        if us < 1000 {
            // wfi version
            // enable interruption
            let mut mstatus_save: usize;
            let mut mie_save: usize;
            asm!("csrrci {ret}, mstatus, {mie}", ret = out(reg) mstatus_save, mie = const MSTATUS_MIE, options(nomem, nostack));
            asm!("csrrs {ret}, mie, {mask}", ret = out(reg) mie_save, mask = in(reg) MIE_ALL_INTS_MASK, options(nomem, nostack));
            asm!("wfi");
            // disable interruption
            asm!("csrrw x0, mie, {mie}", mie = in(reg) mie_save, options(nomem, nostack));
            asm!("csrrw x0, mstatus, {mstatus}", mstatus = in(reg) mstatus_save, options(nomem, nostack));
            (0x600B_0C0C as *mut u32).write_volatile(0); // disable LP timer.
            (0x600b0c44 as *mut u32).write_volatile(1 << 31); // Clear LP timer interrupt
        } else {
            // sleep version
            // lp_core_ll_set_wakeup_source(LP_CORE_LL_WAKEUP_SOURCE_LP_TIMER);
            (0x600B_0180 as *mut u32).write_volatile(1 << 4);
            sleep_store();
        }
    }
}

fn delay_millis(ms: u32) {
    delay_micros(ms * 1000);
}

fn read_sht30(me: &mut LPI2C) -> Result<TempAndHumid, LPI2CError> {
    let addr = 0x44;
    let cmd = [0x24, 0x16];
    let mut buffer = [0u8; 6];
    me.write(addr, &cmd)?;
    delay_millis(20);
    me.read(addr, &mut buffer)?;
    let temp_raw = ((buffer[0] as u16) << 8) | (buffer[1] as u16);
    let hum_raw = ((buffer[3] as u16) << 8) | (buffer[4] as u16);
    let temperature = -4500 + ((17500 * (temp_raw as i32)) / 65535);
    let humidity = (10000 * (hum_raw as i32)) / 65535;
    Ok(TempAndHumid::new(temperature / 100, humidity / 100))
}

#[entry]
fn main() -> ! {
    unsafe {
        (0x600B_0C0C as *mut u32).write_volatile(0); // disable LP timer.
        (0x600b0c44 as *mut u32).write_volatile(1 << 31); // Clear LP timer interrupt
        if sp_bottom != 0 {
            sleep_restore();
        }
    }
    let v: &mut MainLPParcel = get_transfer::<MainLPParcel>().unwrap();
    for _i in 0..v.measurement_count {
        v.result.push(read_sht30(&mut v.i2c).ok());
        delay_millis(1000-20);
    }
    wake_hp_core();
    lp_core_halt()
}
