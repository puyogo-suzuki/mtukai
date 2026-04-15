# esp-rs-copro

A supporting library for ESP32 LP coprocessor in Rust.

## Overview

This project provides utilities for working with the RISC-V LP (Low Power) coprocessor on ESP32 microcontrollers. It includes functions for memory allocation, smart pointers, and traits for movable objects that can be transferred between the main and the LP coprocessors **without unsafe**.

The crate is designed to be used in `no_std` environments and provides a custom allocator for the LP memory.

## Supported Platforms

Currently supports the following ESP32 microcontrollers:
- **ESP32-C6**: Enable the `esp32c6` feature
  - GPIO and I2C are supported.
- **ESP32-S3**: Enable the `esp32s3` feature

## Key Features

### An Allocator for the LP Coprocessor

A dynamic memory allocator designed specifically for the LP coprocessor in `no_std` environments. The allocator works seamlessly with `esp-rs-copro-procmacro` to transfer values between processors.

### `LPBox<T>`: A Smart Pointer for the LP Coprocessor

A smart pointer that supports allocations on LP memory and can be transferred between the main and LP coprocessors. It provides similar functionality to `Box<T>` but is designed for the LP coprocessor's memory and transfer semantics.

### `MovableObject`: A Trait for Movable Objects

A trait that defines the interface for types that can be moved between the main and LP coprocessors. It includes methods for moving values to and from both processors. This trait can be implemented using the `esp-rs-copro-procmacro` derive macro.

**Note**: Each type contained in transferred objects (including nested fields) must implement this trait separately due to Rust's limitations.

### `LPAdapter<T>`: An Adapter for `Copy` Types

An adapter that automatically implements the `MovableObject` trait for types that implement `Copy`. This allows you to easily transfer simple data types between processors without manual implementation.

## Project Structure

This repository contains three components:

### 1. `esp-rs-copro`
The main library providing the core functionality for transferring data between the main and LP coprocessors.

### 2. `esp-rs-copro-procmacro`
A macro crate that provides derive macros for implementing `MovableObject` and procedural macros for sharing allocation information between processors.

### 3. `examples`
Example projects demonstrating how to use the library:
- `temp_sensor`: A temperature and humidity sensor example

## Getting Started

### Creating Your Project

You should prepare three separate projects:
1. **Shared code** - Define structures for shared values
2. **Main coprocessor** - Code running on the main processor
3. **LP coprocessor** - Code running on the LP processor

Each project should include `esp-rs-copro` and `esp-rs-copro-procmacro` as dependencies.

### Example: Shared Code

Define structures that will be shared between processors:

```rust
use esp_rs_copro_procmacro::MovableObject;

#[derive(Clone, Copy, MovableObject)]
pub struct TempAndHumid {
    pub temperature: i32,
    pub humidity: i32,
}

impl TempAndHumid {
    pub fn new(temperature: i32, humidity: i32) -> Self {
        TempAndHumid { temperature, humidity }
    }
}
```

### Using the Library

Transfer data between processors using the `MovableObject` trait:

```rust
// On the main processor
let data = TempAndHumid::new(25, 60);
let lp_box = esp_rs_copro::lpbox::LPBox::new(data);

// Transfer to LP coprocessor
lp_box.move_to_lp()?;
```

## Features

- Processor selection: either features must be enabled.
  - `is-lp-core`: Build for the LP coprocessor
  - `has-lp-core`: Build for the main processor with LP coprocessor support
- Platform: either features must be enabled.
  - `esp32c6`: Enable ESP32-C6 specific features
  - `esp32s3`: Enable ESP32-S3 specific features
- `custom_range`: Use custom memory range configuration
<!-- - `unsafe-vtable`: Enable unsafe vtable operations -->

## Documentation

Generate the documentation with:

```bash
cargo doc --all-features --open
```

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

This project incorporates code from:
- Rust Standard Library
- ESP-RS

which are also available under Apache 2.0 and MIT licenses.

## References

- [ESP-HAL](https://github.com/esp-rs) - ESP-RS Project by Espressif.
