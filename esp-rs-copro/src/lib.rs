//! # A supporting library for ESP32 LP coprocessor in Rust
//! ## Overview
//! This crate provides utilities for working with the RISC-V LP coprocessor on ESP32 microcontrollers.
//! It includes functions for memory allocation, smart pointers, and traits for movable objects that can be transferred between the main and the LP coprocessors **without unsafe**.
//! The crate is designed to be used in no_std environments, and it provides a custom allocator for the LP memory.
//! `esp-rs-copro-procmacro` provides essential macros for using this crate. Please use with `esp-rs-copro-procmacro`.
//! 
//! ## Supported Platforms
//! Currently, it supports the following ESP32 microcontrollers:
//! - ESP32-C6: Enable `esp32c6` feature.
//! 
//! ## Key Features
//! ### An allocator for the LP coprocessor
//! This allocator allows you to allocate memory on the LP coprocessor dynamically.
//! It is designed for use in `no_std` environments. (We expect that running the std environment on the LP coprocessor without the main memory is extremely hard.)
//! `esp-rs-copro-procmacro` cooperates with this allocator to seamlessly transfer values between the processors.
//! 
//! ### [`LPBox<T>`][crate::lpbox::LPBox]: A smart pointer for the LP coprocessor
//! This is a smart pointer type that supports allocations on the LP memory and can be transferred between the main and the LP coprocessors.
//! It provides similar functionality to [`Box<T>`], but it is designed to work with the LP coprocessor's memory and transfer semantics.
//! In the current implementation, it must contain an address of the running processor, however it is designed for handling both addresses of the processors.
//! 
//! ### [`MovableObject`][crate::movableobject::MovableObject]: A trait for movable objects
//! This trait defines the interface for types that can be moved between the main and the LP coprocessors.
//! It includes methods for moving values to and from the main and LP coprocessors.
//! This trait can be implemented using `esp-rs-copro-procmacro`.
//! Due to Rust's limitations, each type contained in transferred objects (including nested fields) must implement this trait separately.
//! 
//! ### [`LPAdapter<T>`][crate::lpadapter::LPAdapter]: An adapter for [`Copy`] types, which implements [`MovableObject`][crate::movableobject::MovableObject]
//! This adapter automatically implements the [`MovableObject`][crate::movableobject::MovableObject] trait for types that implement [`Copy`].
//! This allows you to easily transfer simple data types between the main and the LP coprocessors without needing to manually implement the [`MovableObject`][crate::movableobject::MovableObject] trait for each type.
//! 
//! ## Examples
//! ### Project Structure
//! You must prepare three projects: one for the main coprocessor, one for the LP coprocessor, and one for shared code.
//! Each project should include `esp-rs-copro` and `esp-rs-copro-procmacro` as dependencies.
//! 
//! ### Shared Code
//! This project defines the structures for the shared values.
//! Example: 
//! ```rust,ignore
//! #[derive(Clone, Copy, esp_rs_copro_procmacro::MovableObject)]
//! pub struct TempAndHumid {
//!     pub temperature : i32,
//!     pub humidity : i32
//! }
//! impl TempAndHumid {
//!     pub fn new(temperature : i32, humidity : i32) -> Self { TempAndHumid { temperature, humidity}}
//! }
//! #[derive(esp_rs_copro_procmacro::MovableObject)]
//! pub struct MainLPParcel{
//!     pub i2c : LPI2C,
//!     pub measurement_count : usize,
//!     pub result : LPVec<Option<TempAndHumid>>
//! }
//! ```
//! 
//! ### LP Project
//! This project contains the code that runs on the LP coprocessor. It should include the shared code as a dependency:
//! ```toml
//! esp-rs-copro = { version = ..., features=["is-lp-core", "esp32c6"] }
//! esp-rs-copro-procmacro = { version = ..., features=["is-lp-core"] }
//! your_shared_project = ...
//! ```
//! You must also prepare `build.rs` and a linker script for linking. Please see the example project.
//! 
//! Toplevel example:
//! ```rust,ignore
//! #![no_std]
//! #![no_main]
//! esp_rs_copro_procmacro::esp_rs_copro_statics!(4096);
//! #[alloc_error_handler]
//! fn ignore_alloc_error(_: core::alloc::Layout) -> ! { loop{} }
//! #[entry]
//! fn main() -> !{
//!   let v: &mut MainLPParcel = get_transfer::<MainLPParcel>().unwrap();
//!   // ...
//!   wake_hp_core();
//!   lp_core_halt();
//! }
//! ```
//! 
//! First, you need to prepare the static variables for interacting with the main processor using `esp-rs-copro-procmacro::esp_rs_copro_statics!`.
//! The argument is the size of the static variables in bytes.
//! The processors share information about allocation with these static variables.
//! 
//! Second, you must define a main function like `esp-lp-hal`.
//! It requires the `#[entry]` attribute, and it should never return.
//! Inside the main function, you can get the transferred value from the main processor using `get_transfer::<T>()`, where `T` is the type of the transferred value.
//! 
//! ### Main Project
//! This project contains the code that runs on the main coprocessor. It should include the shared code as a dependency:
//! ```toml
//! esp-rs-copro = { version = ..., features=["esp32c6", "has-lp-core"] }
//! esp-rs-copro-procmacro = { version = ..., features=["esp32c6", "has-lp-core"] }
//! your_shared_project = ...
//! ```
//! 
//! Toplevel example:
//! ```rust,ignore
//! esp_rs_copro_procmacro::define_lp_allocator!();
//! 
//! fn foo(){
//!   // load code to LP core
//!   let lp_core_code = esp-rs-copro-procmacro::load_lp_code2!(
//!     "../your_lp_project/target/riscv32imac-unknown-none-elf/release/esp-rs-copro-lp"
//!   );
//!   let mut parcel = MainLPParcel { ... };
//!   if let Err(e) = lp_core_code.run_light_sleep(&mut lp_core, LpCoreWakeupSource::HpCpu, &mut Rtc::new(peripherals.LPWR), &mut parcel) {
//!     println!("Error running LP core: {:?}", e);
//!   }
//!   // read parcel value.
//! }
//! ```
//! 
//! First, you need to define the allocator for the LP coprocessor using `esp-rs-copro-procmacro::define_lp_allocator!()`.
//! This is a singleton. You must call this macro only once in your project.
//! 
//! Second, you can load the code for the LP coprocessor using `esp-rs-copro-procmacro::load_lp_code2!()`, which is similar to `esp-hal`'s approach.
//! The argument is the path to the compiled binary for the LP coprocessor.
//! The returned code loader provides a `run_light_sleep()` method that takes the LP core, wakeup source, RTC, and the mutable reference to the value to be transferred.
//! The transferred value will be moved to the LP coprocessor, where you can read it using `get_transfer::<T>()`.
//! 
//! **Limitation**: you must call `define_lp_allocator!()` and `load_lp_code2!()` in the same module.
//! This is because `load_lp_code2!()` needs to access the static variable defined by `define_lp_allocator!()`.
//! We are planning to remove this limitation in the future.

#![cfg_attr(feature="nottest", no_std)]
#![feature(layout_for_ptr)]
#![feature(ptr_internals)]
#![feature(ptr_as_ref_unchecked)] // Xtensa toolchain is old. Do not remove this.
#![feature(temporary_niche_types)]
#![feature(sized_type_properties)]
#![feature(rustc_attrs)]
#![feature(const_trait_impl)]
#![feature(const_default)]
#![feature(core_intrinsics)]
#![feature(slice_range)]
#![feature(specialization)]
#![feature(new_range_api)]
#![feature(cast_maybe_uninit)]
#![feature(trusted_len)]
#![feature(exact_size_is_empty)]
#![feature(unsafe_cell_access)]
#![feature(cfg_target_has_atomic)]
#[cfg(feature = "nottest")]
extern crate alloc;
/// This module provides functions and types related to memory allocation in the LP coprocessor. It includes a custom allocator implementation, as well as functions for checking whether an address is in the LP memory range.
pub mod lpalloc;
/// This module provides a smart pointer type, [`LPBox<T>`][crate::lpbox::LPBox], which is similar to [`Box<T>`] but supports allocations on the LP memory and can be transferred between the main and the LP coprocessors.
pub mod lpbox;
/// This module provides an adapter, which automatically implements [`MovableObject`][crate::movableobject::MovableObject] for types that implement [`Copy`].
pub mod lpadapter;
/// This module provides a trait, [`MovableObject`][crate::movableobject::MovableObject], for types that can be moved between the main and the LP coprocessors.
pub mod movableobject;
#[doc(hidden)]
pub mod movableobjectwrapper;
/// This module provides I/O drivers that can be transferred between the main and the LP coprocessors.
#[cfg(feature = "esp32c6")]
pub mod io;
/// This module provides [`LPVec<T>`][crate::collections::lpvec::LPVec] for heap collections that work with the LP memory.
pub mod collections;
pub mod prelude;
#[cfg(not(feature = "is-lp-core"))]
mod addresstranslation;
#[cfg(feature = "has-lp-core")]
#[macro_use]
extern crate esp_println;

/// This represents an error that can occur during the transfer of a value between the main and the LP coprocessors.
#[derive(Debug, Hash, Clone, Copy, Eq, PartialEq)]
pub enum EspCoproError {
    /// The transfer is not allowed. This can occur when you try to transfer on the LP coprocessor.
    NotAllowed,
    /// Something wrong. This is bug of this library.
    IncorrectlyTransferred,
    /// Out of memory. This can occur when the LP memory is out of memory.
    OutOfMemory,
    /// The LP coprocessor is in use. Have you started the LP coprocessor on the different thread?
    InUse
}

/// This is for internal-use.
/// This is used to prevent multiple threads from running the LP coprocessor at the same time.
#[cfg(feature = "has-lp-core")]
#[cfg(target_has_atomic = "8")]
static ESP_COPRO_MUTEX : core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);

/// This is for internal-use.
/// Try to acquire the lock for the LP coprocessor. If the lock is already acquired, return an error.
/// If the MCU does not support atomic operations, this function always succeeds.
#[cfg(feature = "has-lp-core")]
pub fn try_copro_lock() -> Result<(), EspCoproError> {
    #[cfg(target_has_atomic = "8")]
    if let Err(_) = ESP_COPRO_MUTEX.compare_exchange(false, true, core::sync::atomic::Ordering::Relaxed, core::sync::atomic::Ordering::Relaxed) {
        return Err(EspCoproError::InUse);
    }
    Ok(())
}

/// This is for internal-use.
/// Release the lock for the LP coprocessor. This should be called after you finish using the LP coprocessor.
/// If the MCU does not support atomic operations, this function does nothing.
#[cfg(feature = "has-lp-core")]
pub fn copro_unlock() {
    #[cfg(target_has_atomic = "8")]
    ESP_COPRO_MUTEX.store(false, core::sync::atomic::Ordering::Relaxed);
}

impl core::fmt::Display for EspCoproError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            EspCoproError::NotAllowed => write!(f, "EspCoproError::NotAllowed"),
            EspCoproError::IncorrectlyTransferred => write!(f, "T  EspCoproError::IncorrectlyTransferred"),
            EspCoproError::OutOfMemory => write!(f, "EspCoproError::OutOfMemory"),
            EspCoproError::InUse => write!(f, "EspCoproError::InUse")
        }
    }
}

#[cfg(feature = "has-lp-core")]
pub mod transfer_functions {
    use crate::{lpbox::{LPBox, cleanup, remove_by_main}, movableobject::MovableObject};
    use crate::EspCoproError;
    /// This is used in esp-rs-copro-procmacro.
    /// Transfers a value from the main coprocessor to the LP coprocessor.
    /// The value is moved, and the ownership is transferred to the LP coprocessor.
    /// The caller must ensure that the value is not used on the main coprocessor after this function is called.
    pub fn transfer_to_lp<T : MovableObject>(src : &T) -> Result<*mut u8, EspCoproError> {
        LPBox::<T>::write_to_lp(src).map(|ptr| crate::lpalloc::address_translate_to_lp(ptr))
    }

    /// This is used in esp-rs-copro-procmacro.
    /// Transfers a value from the LP coprocessor to the main coprocessor.
    /// The value is moved, and the ownership is transferred to the main coprocessor.
    /// The caller must ensure that the value is not used on the LP coprocessor after this function is called.
    pub unsafe fn transfer_to_main<T : MovableObject>(src : * const u8, dst : &mut T) -> Result<(), EspCoproError> {
        if let Some(v) = unsafe{(crate::lpalloc::address_translate_to_main_const(src) as * const T).as_ref()} {
            unsafe{v.move_to_main(dst as * mut T as * mut u8)?;}
            remove_by_main(dst as * mut T as usize);
            cleanup();
            Ok(())
        } else {
            Err(EspCoproError::IncorrectlyTransferred)
        }
    }
}
