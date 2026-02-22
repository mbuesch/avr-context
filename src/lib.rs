// -*- coding: utf-8 -*-
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (C) 2025 - 2026 Michael Büsch <m@bues.ch>

#![cfg_attr(target_arch = "avr", no_std)]

//! # avr-context: Static checking of execution context
//!
//! This crate provides context-aware cells and context markers for AVR microcontrollers.
//! It helps to safely manage access to data from different execution contexts, such as the main loop and interrupt handlers.
//!
//! The main goal is to provide a zero-cost abstraction for safe and zero overhead data access from the main loop.
//!
//! This crate is built upon `Mutex` and `CriticalSection` from the `bare-metal` and `avr-device` crates.
//!
//! ## Example
//!
//! ```
//! use avr_context::{InitCtx, IrqCtx, MainCtx, MainCtxCell};
//! # #[cfg(target_arch = "avr")]
//! use avr_device::atmega328p as mcu;
//! # #[cfg(not(target_arch = "avr"))]
//! # mod mcu { pub struct PORTB(()); pub struct PORTC(()); pub struct PORTD(()); }
//!
//! /// This global variable can only be accessed from `main()` context.
//! static COUNTER: MainCtxCell<u16> = MainCtxCell::new(0);
//!
//! /// Example user function.
//! fn increment_counter(c: &MainCtx<'_>) {
//!     // Read the global counter variable.
//!     // We do not have to disable interrupts, because possession of the `MainCtx`
//!     // reference guarantees that no interrupt can touch the `COUNTER`.
//!     let mut counter = COUNTER.get(c);
//!
//!     // In this example we just increment the counter for demonstration.
//!     counter = counter.wrapping_add(1);
//!
//!     // Write the counter back to global storage without disabling interrupts.
//!     COUNTER.set(c, counter);
//! }
//!
//! struct MainPeripherals {
//!     PORTC: mcu::PORTC,
//! }
//!
//! /// Main program loop; With interrupts enabled.
//! fn main_loop(c: &MainCtx<'_>, dp: MainPeripherals) -> ! {
//!     loop {
//!         // Put your main loop code here.
//!
//!         increment_counter(c);
//!     }
//! }
//!
//! struct InitPeripherals {
//!     PORTB: mcu::PORTB,
//!     PORTC: mcu::PORTC,
//! }
//!
//! /// Initialization routine before main loop.
//! /// This function runs with interrupts disabled.
//! fn init_function(c: &InitCtx<'_>, dp: InitPeripherals) -> MainPeripherals {
//!
//!     // Put your initialization code here.
//!
//!     // Return the remaining peripherals for use by `main_loop()`.
//!     MainPeripherals { PORTC: dp.PORTC }
//! }
//!
//! avr_context::define_main! {
//!     device: atmega328p,
//!     main: main_loop,
//!     enable_interrupts: true, // main_loop shall run with interrupts enabled.
//!     init: init_function(ctx, InitPeripherals { PORTB, PORTC }) -> MainPeripherals,
//!     static_peripherals: {
//!         static STATIC_PORTD: PORTD, // move PORTD peripheral into a static variable.
//!     },
//! }
//!
//! /// Interrupt entry point.
//! fn timer1_compa_isr(c: &IrqCtx<'_>) {
//!     // The following access to `COUNTER` will not compile.
//!     // We don't have a reference to the `MainCtx` here.
//!
//!     //COUNTER.set(c, 42);
//! }
//!
//! avr_context::define_isr! {
//!     device: atmega328p,
//!     interrupt: TIMER1_COMPA,
//!     isr: timer1_compa_isr,
//! }
//! ```

pub mod cell;
pub mod context;

pub use crate::{
    cell::{InitCtxCell, MainCtxCell},
    context::{InitCtx, IrqCtx, MainCtx},
};

/// Re-export of `bare_metal::CriticalSection`.
pub type CriticalSection<'cs> = bare_metal::CriticalSection<'cs>;
/// Re-export of `bare_metal::Mutex`.
pub type Mutex<T> = bare_metal::Mutex<T>;

#[cfg(test)]
#[allow(clippy::undocumented_unsafe_blocks)]
mod test {
    use super::*;

    #[test]
    fn test_main_ctx() {
        let ctx = unsafe { MainCtx::new() };

        let a: MainCtxCell<u16> = MainCtxCell::new(42);
        let c = a.get(&ctx);
        assert_eq!(c, 42);

        a.set(&ctx, 43);
        let c = a.get(&ctx);
        assert_eq!(c, 43);

        let c: &u16 = a.as_ref(&ctx);
        assert_eq!(*c, 43);

        let c = a.replace(&ctx, 44);
        assert_eq!(c, 43);
        let c = a.get(&ctx);
        assert_eq!(c, 44);
    }

    #[repr(transparent)]
    struct Dropme<T>(pub T);

    impl<T> Drop for Dropme<T> {
        fn drop(&mut self) {
            core::hint::black_box(self);
        }
    }

    static INIT_CELL: InitCtxCell<u16> = unsafe { InitCtxCell::uninit() };

    #[test]
    fn test_init_ctx() {
        fn init(ctx: &InitCtx<'_>, arg: u16) -> u32 {
            let _: CriticalSection<'_> = ctx.cs();
            let _: &MainCtx<'_> = ctx.main_ctx();
            assert_eq!(arg, 15);

            assert_eq!(core::mem::size_of_val(&INIT_CELL), 2);
            INIT_CELL.init(ctx, 14);
            assert_eq!(*INIT_CELL.initctx(ctx), 14);

            let init_cell_zst: InitCtxCell<()> = unsafe { InitCtxCell::uninit() };
            assert_eq!(core::mem::size_of_val(&init_cell_zst), 0);
            init_cell_zst.init(ctx, ());

            let init_cell_drop: InitCtxCell<Dropme<u64>> = unsafe { InitCtxCell::uninit() };
            assert_eq!(core::mem::size_of_val(&init_cell_drop), 8);
            init_cell_drop.init(ctx, Dropme(11));
            drop(init_cell_drop);

            16
        }
        let (ctx, ret) = unsafe { MainCtx::new_with_init(init, 15_u16) };
        assert_eq!(ret, 16);
        let _: u32 = ret;
        let _: MainCtx<'_> = ctx;
    }

    #[test]
    fn test_irq_ctx() {
        let ctx = unsafe { IrqCtx::new() };
        let _: CriticalSection<'_> = ctx.cs();
    }
}

// vim: ts=4 sw=4 expandtab
