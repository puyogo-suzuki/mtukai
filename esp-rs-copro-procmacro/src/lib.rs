// The following code is originally based on code from the esp-rs/esp-hal project,
// licensed under the Apache License, Version 2.0 (the "License").

//! This crate provides procedural macros for the `esp-rs-copro` crate.
//! # Features
//! ## For LP coprocessor's project: with `is-lp-core` feature
//! - [`esp_rs_copro_statics`]: Defines the necessary static variables and functions to use the heap allocator from the main processor. You call this macro once.
//! ## For main processor's project: with `has-lp-core` feature
//! - [`define_lp_allocator`]: Defines the necessary static variables and functions to use the heap allocator on the LP coprocessor from the `esp-rs-copro` crate. You call this macro once. Moreover, it must be visible from [`load_lp_code2`].
//! - [`load_lp_code2`]: Load code to be run on the LP/ULP core. This macro is similar to `esp-hal`'s, however it transfers the given value to the LP memory, and the main processor sleeps.
//! ## For shared project
//! - `MovableObject` derive macro. This macro can be used to automatically implement the `MovableObject` trait for a struct or enum, which defines how to move the value to and from the LP memory.

#[allow(unused)]
use proc_macro::TokenStream;
use quote::quote;

/// This macro must be called in the LP coprocessor's project.
/// It defines the necessary static variables and functions to use the heap allocator from the main processor.
/// The argument is the size of the heap in bytes, and it defaults to 4096 if not provided.
/// 
/// ## Implementation
/// ### `__COPRO_TRANSFER` : *mut u8
/// The main processor writes the pointer to the transferred values and `get_transfer` function reads.
/// ### `__COPRO_ALLOCATOR_{size}` : ImplLPAllocator<{size}>
/// The implementation of the heap allocator.
/// `load_lp_code2` checks for the presence of this symbol to determine whether the heap allocator is used, and if it is, it initializes it and performs the transfer before running the LP core code.
/// ### `ALLOCATOR` : LPAllocator
/// This is the global allocator, which uses the allocator implementation defined as `__COPRO_ALLOCATOR_{size}`.
/// ### `get_transfer` function
/// This function is used to get a reference to the transferred value (`__COPRO_TRANSFER`). It returns `None` if it is `null`.
#[proc_macro]
#[cfg(feature = "is-lp-core")]
pub fn esp_rs_copro_statics(_attr: TokenStream) -> TokenStream {
    use syn::{parse::Error, Ident, LitInt};
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

/// This macro must be called in the main processor's project.
/// It defines the necessary static variables and functions to use the heap allocator on the LP memory from `esp-rs-copro` crate.
/// ## Implementation
/// This defines three functions: `__lpcoproc_allocator_alloc`, `__lpcoproc_allocator_dealloc`, and `__lpcoproc_allocator_get_lp_mem_begin_and_len`.
/// They are called by `esp-rs-copro` crate when the `LPBox<T>` allocates on the LP memory to transfer.
/// They calls each own function pointer, which are set to the actual allocation and deallocation functions in `load_lp_code2`.
#[cfg(feature = "has-lp-core")]
#[proc_macro]
pub fn define_lp_allocator(_input: TokenStream) -> TokenStream {
    quote!{
        static mut lp_alloc_func : fn(layout: core::alloc::Layout) -> * mut u8 = |lay| {0 as * mut u8 };
        static mut lp_dealloc_func : fn(pts : * mut u8, layout: core::alloc::Layout) -> () = |a, b| { };
        #[allow(unused)]
        static mut lp_get_lp_mem_begin_and_len : fn() -> (usize, usize) = || {(0, 0)};
        #[unsafe(no_mangle)]
        pub extern "Rust" fn __lpcoproc_allocator_alloc(layout: core::alloc::Layout) -> * mut u8 {
            unsafe{lp_alloc_func(layout)}
        }
        #[unsafe(no_mangle)]
        pub extern "Rust" fn __lpcoproc_allocator_dealloc(ptr: * mut u8, layout: core::alloc::Layout) {
            unsafe{lp_dealloc_func(ptr, layout)};
        }
        #[allow(unused)]
        #[unsafe(no_mangle)]
        pub extern "Rust" fn __lpcoproc_allocator_get_lp_mem_begin_and_len() -> (usize, usize) {
            unsafe{lp_get_lp_mem_begin_and_len()}
        }
    }.into()
}

/// Load code to be run on the LP/ULP core.
/// This macro is similar to `esp-hal`'s, however it transfers the given value to the LP memory, and the main processor sleeps.
/// ## Example
/// ```rust, no_run
/// let lp_core_code = load_lp_code2!("path.elf");
/// lp_core_code.run_light_sleep(&mut lp_core, lp_core::LpCoreWakeupSource::HpCpu, transfer_value);
/// ```
/// 
/// ## Arguments
/// You specify the path to the ELF file of the LP project, and optionally you can specify the start address of the code in the LP memory and the length of the code.
/// - `lp_start` : The start address of the code in the LP memory. If not specified, it defaults to the start of the RTC Memory section.
/// - `lp_length` : The length of the code. If not specified, it defaults to the size of the code. This is used to check if the code fits in the LP memory.
/// 
/// ## Implementation
/// `run_light_sleep` function performs the following steps:
/// 1. It tries to acquire the coprocessor lock. If it fails, it returns an error.
/// 2. If the heap allocator is used, it initializes it and performs the transfer of the given value to the LP memory.
/// 3. It runs the LP core code.
/// 4. It puts the main processor to light sleep, waiting for the LP core to wake it up.
/// 5. After waking up, it performs the transfer back of the value from the LP memory to the main processor if the heap allocator is used, and then it releases the coprocessor lock.
/// 
/// The transferred value must implement the `MovableObject` trait, which can be derived using the `#[derive(MovableObject)]` macro.
/// This trait defines how to move the value to and from the LP memory.
/// 
#[cfg(feature = "has-lp-core")]
#[proc_macro]
pub fn load_lp_code2(input: TokenStream) -> TokenStream {
    use std::{fs, path::Path};

    use object::{File, Object, ObjectSection, ObjectSymbol, Section, SectionFlags};
    use parse::Error;
    use proc_macro::Span;
    use proc_macro_crate::{FoundCrate, crate_name};
    use syn::{Ident, LitInt, LitStr, parse, Expr, Token, spanned::Spanned};

    struct LoadLpArgs {
        path: LitStr,
        lp_start: Option<Expr>,
        lp_length: Option<LitInt>,
    }

    impl parse::Parse for LoadLpArgs {
        fn parse(input: parse::ParseStream) -> parse::Result<Self> {
            let path: LitStr = input.parse()?;
            let mut lp_start: Option<Expr> = None;
            let mut lp_length: Option<LitInt> = None;

            while input.peek(Token![,]) {
                let _comma: Token![,] = input.parse()?;
                if input.is_empty() {
                    break;
                }

                let ident: Ident = input.parse()?;
                let _eq: Token![=] = input.parse()?;
                let value: Expr = input.parse()?;

                if ident == "lp_start" {
                    lp_start = Some(value);
                } else if ident == "lp_length" {
                    if lp_length.is_some() {
                        return Err(parse::Error::new(ident.span(), "lp_length specified more than once."));
                    }
                    let lit = match value {
                        Expr::Lit(expr_lit) => match expr_lit.lit {
                            syn::Lit::Int(v) => v,
                            _ => {
                                return Err(parse::Error::new(expr_lit.lit.span(), "lp_length must be an integer literal."));
                            }
                        },
                        _ => {
                            return Err(parse::Error::new(value.span(), "lp_length must be an integer literal."));
                        }
                    };
                    lp_length = Some(lit);
                } else {
                    return Err(parse::Error::new(ident.span(), "Unknown argument. Expected lp_start or lp_length."));
                }
            }

            Ok(Self {
                path,
                lp_start,
                lp_length,
            })
        }
    }

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
        quote!{ use #ident ::{ transfer_functions::*, lpbox::LPBox, lpalloc::ImplLPAllocator, movableobject::MovableObject, EspCoproError, try_copro_lock, copro_unlock}; }
    } else { quote!{} };

    let args: LoadLpArgs = match syn::parse(input) {
        Ok(args) => args,
        Err(e) => return e.into_compile_error().into(),
    };

    let elf_file = args.path.value();

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
            match section.flags() {
                SectionFlags::Elf{sh_flags: sh} => (sh & u64::from(object::elf::SHF_ALLOC)) != 0 ,
                _ => false
            }
        })
        .collect();
    sections.sort_by(|a, b| a.address().partial_cmp(&b.address()).unwrap());

    let mut binary: Vec<u8> = Vec::new();
    let mut last_address = if cfg!(feature = "has-lp-core") {
        0x5000_0000
    } else {
        0x0
    };

    if sections.is_empty() {
        return Error::new(
            Span::call_site().into(),
            "Given file doesn't seem to have any allocatable sections.",
        )
        .to_compile_error()
        .into();
    } else if sections[0].address() < last_address {
        return Error::new(
            Span::call_site().into(),
            "Given file doesn't seem to be a valid LP/ULP core application.",
        )
        .to_compile_error()
        .into();
    }

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
    let run_light_sleep_args : Vec<proc_macro2::TokenStream> = magic_symbol
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
        use #hal_crate::rtc_cntl::Rtc;
        use #hal_crate::rtc_cntl::sleep::WakeFromLpCoreWakeupSource;
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

    let (transfer, transfer_back) =
        if let Some(a) = obj_file.symbols().find(|s| s.name() == Ok("__COPRO_TRANSFER")).map(|s| s.address()) {
            (quote!{
                unsafe {
                    use core::alloc::GlobalAlloc;
                    lp_alloc_func = |layout| { 
                        unsafe { LpCoreCode::get_allocator().as_ref().unwrap().alloc(layout) }
                    }; 
                    lp_dealloc_func = |ptr, layout| {
                        unsafe { LpCoreCode::get_allocator().as_ref().unwrap().dealloc(ptr, layout); }
                    };
                    lp_get_lp_mem_begin_and_len = || {
                        unsafe {
                            let alc = LpCoreCode::get_allocator().as_ref().unwrap();
                            (alc.heap.as_ptr() as usize, alc.heap.len())
                        }
                    };
                }
                let trans = transfer_to_lp(transfer_value)?;
                unsafe {((#a) as *mut *mut u8).write_volatile(trans);}
            },
            quote!{unsafe { transfer_to_main(((#a) as *mut *mut u8).read_volatile(), transfer_value) } })
        } else { (quote! {}, quote! {})};
    let allocsym = obj_file.symbols().find(|s| s.name().map_or(false, |v| v.starts_with("__COPRO_ALLOCATOR_")));
    let allocfun = if let Some(a) = allocsym {
        let addr = a.address();
        let size = a.name().ok().and_then(|v| v["__COPRO_ALLOCATOR_".len()..].parse::<usize>().ok());
        quote!{
            pub fn get_allocator() -> *mut ImplLPAllocator<#size> {
                #addr as *mut ImplLPAllocator<#size>
            }
        }
    } else {quote!{}};
    let alloccall = if !allocfun.is_empty() {quote!{
        let all = LpCoreCode::get_allocator();
        unsafe{all.as_mut().unwrap().init()};
        #transfer
    }} else {quote!{}};

    let copy_dest = if let Some(lp_start) = args.lp_start {
        quote! { (#lp_start as *mut u8) }
    } else {
        quote! { &#rtc_code_start as *const u32 as *mut u8 }
    };

    if let Some(lp_length) = &args.lp_length {
        let limit = match lp_length.base10_parse::<usize>() {
            Ok(v) => v,
            Err(_) => {
                return Error::new(lp_length.span(), "lp_length must be a valid integer literal.")
                    .to_compile_error()
                    .into();
            }
        };
        if binary.len() > limit {
            return Error::new(
                lp_length.span(),
                "LP_CODE length exceeds lp_length.",
            )
            .to_compile_error()
            .into();
        }
    }
    
    quote! {
        {
            #imports

            struct LpCoreCode {}

            static LP_CODE: &[u8] = &[#(#binary),*];

            unsafe extern "C" {
                static #rtc_code_start: u32;
            }

            unsafe {
                core::ptr::copy_nonoverlapping(LP_CODE as *const _ as *const u8, #copy_dest, LP_CODE.len());
            }

            impl LpCoreCode {
                pub fn run_light_sleep<T : MovableObject>(
                    &self,
                    lp_core: &mut LpCore,
                    wakeup_source: LpCoreWakeupSource,
                    rtc : &mut Rtc,
                    transfer_value : &mut T,
                    #(_: #run_light_sleep_args),*
                ) -> Result<(), EspCoproError> {
                    try_copro_lock()?;
                    #alloccall
                    lp_core.run(wakeup_source);
                    rtc.sleep_light(&[&WakeFromLpCoreWakeupSource::new()]);
                    let ret = #transfer_back;
                    copro_unlock();
                    ret
                }
                #allocfun
            }

            LpCoreCode {}
        }
    }
    .into()
}

/// This macro can be used to automatically implement the `MovableObject` trait for a struct or enum, which defines how to move the value to and from the LP memory.
/// ## Implementation
/// ### Overview of structs
/// This macro simply generates code below for each struct member.
/// ```rust,no_run
/// self.member.wrap_move_to_main( (&mut (*dest).member) as * mut _ as * mut u8)?;
/// ```
/// ### Overview of enums
/// We cannot directly get the pointers of the members of the enum, so we need to use `MaybeUninit` to create uninitialized buffers for each member.
/// Then, we call the `wrap_move_to_main` or `wrap_move_to_lp` function for each member to move the value to the buffer, and then we construct the enum using the buffers.
/// ```rust,no_run
/// let mut member_buf = core::mem::MaybeUninit::uninit();
/// self.member.wrap_move_to_main( (&mut member_buf) as * mut _ as * mut u8)?;
/// let member_buf = member_buf.assume_init()
/// ```
/// Finally, we reconstruct the enum using the constructor with the buffers.
/// ```rust,no_run
/// dest.write_volatile(Self {member: member_buf, ...});
/// ```
/// 
/// ### Resolution of `move_to` methods
/// Due to Rust's limitation, we cannot directly call the `move_to_main` or `move_to_lp` function of the member's type, so `esp-rs-copro` defines wrapper functions `wrap_move_to_main` and `wrap_move_to_lp`.
/// `wrap` functions for `MovableObject` simply call the `move_to` functions.
/// In addition, `wrap` functions for [`Copy`] types copies the value.
/// Thus, we can transparently support members typed as [`Copy`], such as [`i32`], and so on.
#[proc_macro_derive(MovableObject)]
pub fn movable_object_derive(input: TokenStream) -> TokenStream {
    use syn::{parse::Error, Ident, Data, punctuated::Punctuated};
    use proc_macro::Span;
    use proc_macro_crate::{FoundCrate, crate_name};

    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    let esp_copro_crate = if let Ok(FoundCrate::Name(ref name)) = crate_name("esp-rs-copro") {
        let ident = Ident::new(name, Span::call_site().into());
        quote!{#ident}
    } else { quote!{crate} };
    let name = &input.ident;
    match input.data {
        Data::Struct(s) => {
            let member_names = s.fields.iter().enumerate().map(|(i, f)| {
                if let Some(ident) = &f.ident {
                    quote! { #ident }
                } else {
                    let index = syn::Index::from(i);
                    quote! { #index }
                }
            }).collect::<Vec<_>>();
            let move_to_mains = member_names.iter().map(|name| {
                quote! {self.#name.wrap_move_to_main( (&mut (*dest).#name) as * mut _ as * mut u8)?;}
            });
            let move_to_lps = member_names.iter().map(|name| {
                quote! {self.#name.wrap_move_to_lp( (&mut (*dest).#name) as * mut _ as * mut u8)?;}
            });
            let expanded = quote! {
                impl #esp_copro_crate::movableobject::MovableObject for #name {
                    unsafe fn move_to_main(&self, dest : *mut u8) -> Result<(), #esp_copro_crate::EspCoproError> {
                        use #esp_copro_crate::movableobjectwrapper::*;
                        let dest = dest as * mut #name;
                        #(#move_to_mains)*
                        Ok(())
                    }
                    unsafe fn move_to_lp(&self, dest : *mut u8) -> Result<(), #esp_copro_crate::EspCoproError> {
                        use #esp_copro_crate::movableobjectwrapper::*;
                        let dest = dest as * mut #name;
                        #(#move_to_lps)*
                        Ok(())
                    }
                }
            };
            TokenStream::from(expanded)
        },
        Data::Enum(e) => {
            let gen_fields_arm_unnamed = |fname_move_to : &Ident, name : &Ident, vname : &Ident, fields : &Punctuated<syn::Field, syn::token::Comma>| {
                let field_names = fields.iter().enumerate().map(|(i, _)| {
                    Ident::new(&format!("field_{}", i), Span::call_site().into())
                }).collect::<Vec<_>>();
                let dsts = fields.iter().enumerate().map(|(i, _)| {
                    Ident::new(&format!("field_dst_{}", i), Span::call_site().into())
                }).collect::<Vec<_>>();
                let bufs = fields.iter().zip(dsts.iter()).map(|(f, ident)| {
                    let ty = &f.ty;
                    quote! { let mut #ident = core::mem::MaybeUninit::<#ty>::uninit(); }
                });
                let move_to_mains = field_names.iter().zip(dsts.iter()).map(|(src, dst)| {
                    quote! { #src.#fname_move_to( (&mut #dst) as * mut _ as * mut u8)?; }
                });
                quote! {
                    #name::#vname ( #(#field_names),* ) => {
                        unsafe{
                            #(#bufs)*
                            #(#move_to_mains)*
                            #name::#vname( #(#dsts.assume_init()),* )
                        }
                    },
                }
            };
            let gen_fields_arm_named = |fname_move_to : &Ident, name : &Ident, vname : &Ident, fields : &Punctuated<syn::Field, syn::token::Comma>| {
                let field_names = fields.iter().filter_map(|n| {
                    n.ident.clone()
                }).collect::<Vec<_>>();
                let dsts = field_names.iter().map(|i| {
                    Ident::new(&format!("field_dst_{}", i.to_string()), Span::call_site().into())
                }).collect::<Vec<_>>();
                let bufs = fields.iter().zip(dsts.iter()).map(|(f, ident)| {
                    let ty = &f.ty;
                    quote! { let mut #ident = core::mem::MaybeUninit::<#ty>::uninit(); }
                });
                let move_to_mains = field_names.iter().zip(dsts.iter()).map(|(src, dst)| {
                    quote! { #src.#fname_move_to( (&mut #dst) as * mut _ as * mut u8)?; }
                });
                let constructions = field_names.iter().zip(dsts.iter()).map(|(src, dst)| {
                    quote! { #src : #dst.assume_init() }
                });
                quote! {
                    #name::#vname{#(#field_names),*} => {
                        unsafe{
                            #(#bufs)*
                            #(#move_to_mains)*
                            #name::#vname{ #(#constructions),* }
                        }
                    },
                }
            };
            let gen_arm = |fname_move_to : &Ident, v : &syn::Variant| {
                let vname = &v.ident;
                match &v.fields {
                    syn::Fields::Unit => { quote! { #name::#vname => { #name::#vname }, } },
                    syn::Fields::Unnamed(fu) => { gen_fields_arm_unnamed(&fname_move_to, name, &vname, &fu.unnamed) },
                    syn::Fields::Named(fn_) => { gen_fields_arm_named(&fname_move_to, name, &vname, &fn_.named) }
                }
            };
            let fname = Ident::new("wrap_move_to_main", Span::call_site().into());
            let arms_main = e.variants.iter().map(|v| {
                gen_arm(&fname, v)
            });
            let fname = Ident::new("wrap_move_to_lp", Span::call_site().into());
            let arms_lp = e.variants.iter().map(|v| {
                gen_arm(&fname, v)
            });
            quote! {
                impl #esp_copro_crate::movableobject::MovableObject for #name {
                    unsafe fn move_to_main(&self, dest : *mut u8) -> Result<(), #esp_copro_crate::EspCoproError> {
                        use #esp_copro_crate::movableobjectwrapper::*;
                        unsafe { (dest as * mut #name).write_volatile(match &self {
                            #(#arms_main)*
                        }); }
                        Ok(())
                    }
                    unsafe fn move_to_lp(&self, dest : *mut u8) -> Result<(), #esp_copro_crate::EspCoproError> {
                        use #esp_copro_crate::movableobjectwrapper::*;
                        unsafe { (dest as * mut #name).write_volatile(match &self {
                            #(#arms_lp)*
                        }); }
                        Ok(())
                    }
                }
            }.into()
        },
        _ => Error::new(
            Span::call_site().into(),
            "Union types are not supported.",
        )
        .to_compile_error()
        .into()
    }
}

