// The following code is originally based on code from the esp-rs/esp-hal project,
// licensed under the Apache License, Version 2.0 (the "License").
#[allow(unused)]
use proc_macro::TokenStream;
use quote::quote;
use syn::LitInt;

#[proc_macro]
pub fn esp_rs_copro_statics(_attr: TokenStream) -> TokenStream {
    use syn::{parse::Error, Ident};
    use proc_macro::Span;
    use proc_macro2::Literal;
    use proc_macro_crate::{FoundCrate, crate_name};
    let copro_crate = if let Ok(FoundCrate::Name(ref name)) = crate_name("esp-rs-copro") {
        let ident = Ident::new(name, Span::call_site().into());
        quote!{#ident}
    } else { quote!{crate} };
    
    let heap_size = if _attr.is_empty() {4*1024} else {match syn::parse::<LitInt>(_attr).and_then(|v| {v.base10_parse::<usize>()}) {
        Ok(size) => size,
        Err(_) => return Error::new(Span::call_site().into(), "The argument must be an integer.").to_compile_error().into(),
    }};
    let export_name = format!("__COPRO_ALLOCATOR_{}", heap_size);
    let export_name_lit = Literal::string(&export_name);
    let expanded = quote! {
        #[unsafe(export_name="__COPRO_TRANSFER")]
        static mut TRANSFER : *mut u8 = 0 as * mut u8;
        #[used]
        #[unsafe(export_name=#export_name_lit)]
        static mut allocator : #copro_crate::lpalloc::ImplLPAllocator<#heap_size> = #copro_crate::lpalloc::ImplLPAllocator::new();
        fn get_transfer<T : #copro_crate::movableobject::MovableObject>() -> Option<&'static mut T> {
            if(unsafe{!allocator.free_ptr.get().is_null()}) {
                Some(unsafe { &mut *(TRANSFER as * mut T) })
            } else {
                None
            }
        }
        struct LPAllocator {}
        #[global_allocator]
        static mut ALLOCATOR : LPAllocator = LPAllocator {};
        unsafe impl core::alloc::GlobalAlloc for LPAllocator {
            unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
                unsafe { allocator.alloc(layout) }
            }
            unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
                unsafe { allocator.dealloc(ptr, layout); }
            }
        }
        unsafe impl Sync for LPAllocator {}
    };
    expanded.into()
}

/// Load code to be run on the LP/ULP core.
///
/// ## Example
/// ```rust, no_run
/// let lp_core_code = load_lp_code!("path.elf");
/// lp_core_code.run(&mut lp_core, lp_core::LpCoreWakeupSource::HpCpu, lp_pin);
/// ```
#[cfg(any(feature = "has-lp-core", feature = "has-ulp-core", test))]
#[proc_macro]
pub fn load_lp_code2(input: TokenStream) -> TokenStream {
    use std::{fs, path::Path};

    use object::{File, Object, ObjectSection, ObjectSymbol, Section, SectionKind};
    use parse::Error;
    use proc_macro::Span;
    use proc_macro_crate::{FoundCrate, crate_name};
    use syn::{Ident, LitStr, parse};

    let hal_crate = if cfg!(any(feature = "is-lp-core", feature = "is-ulp-core")) {
        crate_name("esp-lp-hal")
    } else {
        crate_name("esp-hal")
    };
    
    let hal_crate = if let Ok(FoundCrate::Name(ref name)) = hal_crate {
        let ident = Ident::new(name, Span::call_site().into());
        quote!( #ident )
    } else {
        quote!(crate)
    };
    
    let copro_crate_use = if let Ok(FoundCrate::Name(ref name)) = crate_name("esp-rs-copro") {
        let ident = Ident::new(name, Span::call_site().into());
        quote!{use #ident ::{ lpbox::LPBox, lpalloc::ImplLPAllocator, movableobject::MovableObject};}
    } else { quote!{} };

    let lit: LitStr = match syn::parse(input) {
        Ok(lit) => lit,
        Err(e) => return e.into_compile_error().into(),
    };

    let elf_file = lit.value();

    if !Path::new(&elf_file).exists() {
        return Error::new(Span::call_site().into(), "File not found")
            .to_compile_error()
            .into();
    }

    let bin_data = fs::read(elf_file).unwrap();
    let obj_file = File::parse(&*bin_data).unwrap();
    let sections = obj_file.sections();

    let mut sections: Vec<Section> = sections
        .into_iter()
        .filter(|section| {
            matches!(
                section.kind(),
                SectionKind::Text
                    | SectionKind::ReadOnlyData
                    | SectionKind::Data
                    | SectionKind::UninitializedData
            )
        })
        .collect();
    sections.sort_by(|a, b| a.address().partial_cmp(&b.address()).unwrap());

    let mut binary: Vec<u8> = Vec::new();
    let mut last_address = if cfg!(feature = "has-lp-core") {
        0x5000_0000
    } else {
        0x0
    };

    for section in sections {
        if section.address() > last_address {
            let fill = section.address() - last_address;
            binary.extend(std::iter::repeat(0).take(fill as usize));
        }

        binary.extend_from_slice(section.data().unwrap());
        last_address = section.address() + section.size();
    }

    let magic_symbol = obj_file
        .symbols()
        .find(|s| s.name().unwrap().starts_with("__ULP_MAGIC_"));

    let magic_symbol = if let Some(magic_symbol) = magic_symbol {
        magic_symbol.name().unwrap()
    } else {
        return Error::new(
            Span::call_site().into(),
            "Given file doesn't seem to be an LP/ULP core application.",
        )
        .to_compile_error()
        .into();
    };

    let magic_symbol = magic_symbol.trim_start_matches("__ULP_MAGIC_");
    let args: Vec<proc_macro2::TokenStream> = magic_symbol
        .split("$")
        .map(|t| {
            let t = if t.contains("OutputOpenDrain") {
                t.replace("OutputOpenDrain", "LowPowerOutputOpenDrain")
            } else {
                t.replace("Output", "LowPowerOutput")
            };
            let t = t.replace("Input", "LowPowerInput");
            t.parse().unwrap()
        })
        .filter(|v: &proc_macro2::TokenStream| !v.is_empty())
        .collect();

    #[cfg(feature = "has-lp-core")]
    let imports = quote! {
        use #hal_crate::lp_core::LpCore;
        use #hal_crate::lp_core::LpCoreWakeupSource;
        use #hal_crate::gpio::lp_io::LowPowerOutput;
        use #hal_crate::gpio::*;
        use #hal_crate::uart::lp_uart::LpUart;
        use #hal_crate::i2c::lp_i2c::LpI2c;
        #copro_crate_use;
    };
    #[cfg(feature = "has-ulp-core")]
    let imports = quote! {
        use #hal_crate::ulp_core::UlpCore as LpCore;
        use #hal_crate::ulp_core::UlpCoreWakeupSource as LpCoreWakeupSource;
        use #hal_crate::gpio::*;
        #copro_crate_use;
    };

    #[cfg(feature = "has-lp-core")]
    let rtc_code_start = quote! { _rtc_fast_data_start };
    #[cfg(feature = "has-ulp-core")]
    let rtc_code_start = quote! { _rtc_slow_data_start };

    let transfer = if let Some(a) = obj_file.symbols().find(|s| s.name() == Ok("__COPRO_TRANSFER")).map(|s| s.address()) {
        quote!{unsafe {((#a) as *mut *mut u8).write_volatile(LPBox::<T>::write_to_lp(transfer_value));}}
    } else { quote! {}};
    let allocsym = obj_file.symbols().find(|s| s.name().map_or(false, |v| v.starts_with("__COPRO_ALLOCATOR_")));
    let (allocfun, lpalloc) = if let Some(a) = allocsym {
        let addr = a.address();
        let size = a.name().ok().and_then(|v| v["__COPRO_ALLOCATOR_".len()..].parse::<usize>().ok());
        (quote!{
            pub fn get_allocator() -> *mut ImplLPAllocator<#size> {
                #addr as *mut ImplLPAllocator<#size>
            }
        }, quote!{
            struct LPAllocator {}
            static mut LPALLOCATOR : LPAllocator = LPAllocator {};
            impl LPAllocator {
                fn get_allocator() -> *mut ImplLPAllocator<#size> {
                    #addr as *mut ImplLPAllocator<#size>
                }
            }
            unsafe impl core::alloc::GlobalAlloc for LPAllocator {
                unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
                    use core::alloc::GlobalAlloc;
                    unsafe { LPAllocator::get_allocator().as_ref().unwrap().alloc(layout) }
                }
                unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
                    use core::alloc::GlobalAlloc;
                    unsafe { LPAllocator::get_allocator().as_ref().unwrap().dealloc(ptr, layout); }
                }
            }
            #[unsafe(no_mangle)]
            pub extern "Rust" fn __lpcoproc_allocator_alloc(layout: core::alloc::Layout) -> * mut u8 {
                use core::alloc::GlobalAlloc;
                println!("alloc on LP");
                unsafe { LPAllocator::get_allocator().as_ref().unwrap().alloc(layout) }
            }
            #[unsafe(no_mangle)]
            pub extern "Rust" fn __lpcoproc_allocator_dealloc(ptr: * mut u8, layout: core::alloc::Layout) {
                use core::alloc::GlobalAlloc;
                println!("dealloc on LP");
                unsafe { LPAllocator::get_allocator().as_ref().unwrap().dealloc(ptr, layout); }
            }
            unsafe impl Sync for LPAllocator {}
        })
    } else {(quote!{}, quote!())};
    let alloccall = if !allocfun.is_empty() {quote!{
        let all = LpCoreCode::get_allocator();
        unsafe{all.as_mut().unwrap().init()};
        #transfer
    }} else {quote!{}};
    
    quote! {
        {
            #imports

            struct LpCoreCode {}

            static LP_CODE: &[u8] = &[#(#binary),*];

            unsafe extern "C" {
                static #rtc_code_start: u32;
            }

            unsafe {
                core::ptr::copy_nonoverlapping(LP_CODE as *const _ as *const u8, &#rtc_code_start as *const u32 as *mut u8, LP_CODE.len());
            }

            #lpalloc

            impl LpCoreCode {
                pub fn run<T : MovableObject>(
                    &self,
                    lp_core: &mut LpCore,
                    wakeup_source: LpCoreWakeupSource,
                    transfer_value : &mut T,
                    #(_: #args),*
                ) {
                    #alloccall
                    lp_core.run(wakeup_source);
                }
                #allocfun
            }

            LpCoreCode {}
        }
    }
    .into()
}