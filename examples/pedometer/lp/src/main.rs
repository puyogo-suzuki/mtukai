#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

use embedded_hal::delay::DelayNs;
use esp_lp_hal::{delay::Delay, prelude::*, wake_hp_core};
use esp_rs_copro::{io::i2c::{LPI2C, LPI2CError}, prelude::lp_core_halt};
use pedometer_shared::{MainLPParcel, Vector3D};
use panic_halt as _;

esp_rs_copro_procmacro::esp_rs_copro_statics!(4096);
#[alloc_error_handler]
fn ignore_alloc_error(_: core::alloc::Layout) -> ! {
    loop{}
}

struct CircularBuffer<T, const SIZE : usize> {
    buffer: [T; SIZE],
    head: usize
}

impl <T: Default + Copy, const SIZE: usize> CircularBuffer<T, SIZE> {
    fn new() -> Self {
        CircularBuffer {
            buffer: [T::default(); SIZE],
            head: 0
        }
    }

    #[allow(unused)]
    fn new_with_value(value: T) -> Self {
        CircularBuffer {
            buffer: [value; SIZE],
            head: 0
        }
    }

    fn push(&mut self, value: T) {
        self.buffer[self.head] = value;
        self.head = (self.head + 1) % SIZE;
    }

    fn iter(&self) -> impl Iterator<Item=&T> {
        self.buffer.iter()
    }

    fn get_last(&self) -> &T {
        &self.buffer[self.head]
    }

    fn get_current_idx(&self) -> usize {
        self.head
    }
}

const MPU6050: u8 = 0x68;
const G_TO_RAW: u32 = 16384;

fn mpu6050_enable(i2c: &mut LPI2C, enable: bool) -> Result<(), LPI2CError> {
    i2c.write(MPU6050, &[0x1A, 2])?;
    i2c.write(MPU6050, &[0x6B, if enable { 0x00 } else { 1 << 6 }])
}

fn mpu6050_read(i2c: &mut LPI2C) -> Result<Vector3D, LPI2CError> {
    let mut buffer = [0u8; 6];
    i2c.write(MPU6050, &[0x3B])?;
    i2c.read(MPU6050, &mut buffer)?;
    let x = ((buffer[0] as i16) << 8) | (buffer[1] as i16);
    let y = ((buffer[2] as i16) << 8) | (buffer[3] as i16);
    let z = ((buffer[4] as i16) << 8) | (buffer[5] as i16);
    Ok(Vector3D::new(x, y, z))
}

const _1_SECOND: u32 = 50;
const REGULATION_OFF_TIME : u32 = 2 * _1_SECOND;
const LP_WINDOW_SIZE: usize = 4;
const FILTER_WINDOW_SIZE: usize = LP_WINDOW_SIZE + 1;

const SENSITIVITY: u32 = G_TO_RAW / 20; // 0.05G
#[allow(unused)]
const INIT_OFFSET_VALUE: u32 = G_TO_RAW; // 1G

// Reference:
// AN-2554:  Step Counting Using the ADXL367 by Analog Devices, Inc.
// https://www.analog.com/en/resources/app-notes/an-2554.html
// We disabled dynamic threshold adjustment because it is too conservative in our enviornment.
// In this code, we only evaluate max-min peak difference.
#[entry]
fn main() -> ! {
    let v: &mut MainLPParcel = get_transfer::<MainLPParcel>().unwrap();

    let mut raw_buffer: CircularBuffer<u32, LP_WINDOW_SIZE> = CircularBuffer::new();
    let mut raw_buffer_sum : u32 = 0;

    let mut filtered_buffer: CircularBuffer<u32, FILTER_WINDOW_SIZE> = CircularBuffer::new();
    let mut last_max : u32 = 0;
    let mut flag_max : bool = false;
    let mut flag_max_min_samplecounter : u32 = 0;

    // let mut dynamic_threshold_buffer : CircularBuffer<u32, LP_WINDOW_SIZE> = CircularBuffer::new_with_value(INIT_OFFSET_VALUE);
    // let mut old_threshold : u32 = INIT_OFFSET_VALUE;
    // let mut dynamic_threshold_buffer_sum : u32 = INIT_OFFSET_VALUE * (LP_WINDOW_SIZE as u32);

    let mut step_to_step_samples : u32 = 0;
    let mut regulation_mode : bool = false;

    let mut count_steps : u32 = 0;
    let mut possible_steps : u32 = 0;

    let mut flag_threshold_counter : u32 = 0;

    let _ = mpu6050_enable(&mut v.i2c, true);
    Delay.delay_ms(1000);

    let mut last_button = false;

    loop {
        if let Ok(data) = mpu6050_read(&mut v.i2c) {
            let val = (data.x as i32).abs() as u32 + (data.y as i32).abs() as u32 + (data.z as i32).abs() as u32;
            raw_buffer_sum = raw_buffer_sum + val - raw_buffer.get_last();
            raw_buffer.push(val);
            let filtered_val = raw_buffer_sum / (LP_WINDOW_SIZE as u32);
            filtered_buffer.push(filtered_val);

            let (max_value, max_idx) = filtered_buffer.iter().enumerate().fold((0, 0), |(max_val, max_idx), (idx, &val)| {
                if val > max_val {
                    (val, idx)
                } else {
                    (max_val, max_idx)
                }
            });
            let (min_value, min_idx) = filtered_buffer.iter().enumerate().fold((u32::MAX, 0), |(min_val, min_idx), (idx, &val)| {
                if val < min_val {
                    (val, idx)
                } else {
                    (min_val, min_idx)
                }
            });

            if !flag_max {
                if max_idx == (filtered_buffer.get_current_idx() + FILTER_WINDOW_SIZE - 1) % FILTER_WINDOW_SIZE {
                    flag_max = true;
                    last_max = max_value;
                    flag_max_min_samplecounter = 0;
                }
            } else {
                if min_idx == (filtered_buffer.get_current_idx() + FILTER_WINDOW_SIZE - 1) % FILTER_WINDOW_SIZE {
                    let difference = last_max as i32 - min_value as i32;
                    flag_max = false;
                    flag_max_min_samplecounter = 0;
                    // Too conservative in our environment...
                    // let flag_threshold = (last_max > (old_threshold + SENSITIVITY*2/3)) && (min_value < (old_threshold - SENSITIVITY*2/3));
                    let flag_threshold = true;
                    if flag_threshold { flag_threshold_counter = 0; } else { flag_threshold_counter += 1; }
                    if difference > SENSITIVITY as i32 {
                        // let new_threshold = (last_max + min_value) / 2;
                        // dynamic_threshold_buffer_sum = dynamic_threshold_buffer_sum + new_threshold - dynamic_threshold_buffer.get_last();
                        // dynamic_threshold_buffer.push(new_threshold);
                        // old_threshold = dynamic_threshold_buffer_sum / (LP_WINDOW_SIZE as u32);
                        
                        if flag_threshold {
                            step_to_step_samples = 0;
                            
                            if regulation_mode {
                                count_steps += 1;
                            } else {
                                possible_steps += 1;
                                if possible_steps == 8 {
                                    count_steps += possible_steps;
                                    possible_steps = 0;
                                    regulation_mode = true;
                                }
                            }
                        }
                    }
                    if flag_threshold_counter > 1 {
                        regulation_mode = false;
                        possible_steps = 0;
                        flag_threshold_counter = 0;
                    }
                } else {
                    flag_max_min_samplecounter += 1;
                    if flag_max_min_samplecounter > _1_SECOND {
                        flag_max = false;
                        flag_max_min_samplecounter = 0;
                        possible_steps = 0;
                    }
                }
            }
            step_to_step_samples += 1;
            if step_to_step_samples >= REGULATION_OFF_TIME {
                step_to_step_samples = 0;
                possible_steps = 0;
                regulation_mode = false;
                flag_max_min_samplecounter = 0;
                if regulation_mode {
                    regulation_mode = false;
                    // old_threshold = INIT_OFFSET_VALUE;
                    flag_threshold_counter = 0;
                    // dynamic_threshold_buffer_sum = INIT_OFFSET_VALUE * LP_WINDOW_SIZE as u32;
                    // dynamic_threshold_buffer = CircularBuffer::new_with_value(INIT_OFFSET_VALUE);
                }
            }
        }
        Delay.delay_ms(20);
        let button_level = v.button.level();
        if button_level && !last_button {
            break; // Wake up the main processor.
        }
        last_button = button_level;
    }
    v.steps = count_steps as usize;
    wake_hp_core();
    lp_core_halt()
}