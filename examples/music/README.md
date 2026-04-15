# A music example
This example plays a score provided by the main processor.

## Requirements
 - ESP32-C6
 - Buzzer

Currently, this project cannot be compiled for ESP32-S3, because rustc (the Rust compiler) stuck.

## Connection
### MPU6050 Accelerometer
ESP32-C6 outputs a PWM wave.
 - GPIO1 - Buzzer
