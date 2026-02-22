// -*- coding: utf-8 -*-
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (C) 2025 - 2026 Michael Büsch <m@bues.ch>

//! Context marker types.

use crate::CriticalSection;
use core::sync::atomic::{Ordering::SeqCst, fence};

/// 'main()' context marker.
///
/// The possession of this marker or a reference to this marker
/// guarantees the execution from `main()` context.
pub struct MainCtx<'cs>(CriticalSection<'cs>);

/// Interrupt context marker.
///
/// The possession of this marker or a reference to this marker
/// guarantees the execution from interrupt context.
pub struct IrqCtx<'cs>(CriticalSection<'cs>);

macro_rules! impl_context {
    ($name:ident) => {
        impl<'cs> $name<'cs> {
            #[inline(always)]
            #[allow(unreachable_code)]
            #[allow(unused_variables)]
            unsafe fn internal_new() -> Self {
                // This crate is unsound in multi processor or multi threading environments.
                #[cfg(not(any(target_arch = "avr", test)))]
                panic!("This crate is only designed to be sound on target_arch=avr");

                // SAFETY:
                // This `cs` is used with the low level `Mutex` primitive.
                // The IRQ safety itself is upheld by the possession of the
                // context objects.
                //
                // If a function takes a `MainCtx` argument, it can only be
                // called from `main()` context.
                // Correspondingly for `IrqCtx`.
                //
                // The `MainCtxCell` can be used to ensure that the contained data
                // can only be accessed from the main context.
                // That in turn means that no interrupts have to be disabled ever
                // during accesses.
                //
                // With this mechanism we can run the main context with IRQs
                // enabled. There cannot be any concurrency in safe code.
                let cs = unsafe { CriticalSection::new() };

                // Barrier to ensure that no memory accesses from inside of the
                // context are moved outside.
                fence(SeqCst);

                Self(cs)
            }
        }

        impl<'cs> Drop for $name<'cs> {
            #[inline(always)]
            fn drop(&mut self) {
                // Barrier to ensure that no memory accesses from inside of the
                // context are moved outside.
                fence(SeqCst);
            }
        }
    };
}

impl_context!(MainCtx);
impl_context!(IrqCtx);

impl<'cs> MainCtx<'cs> {
    /// Create a new `main()` context.
    ///
    /// # Safety
    ///
    /// This constructor may only be called from the `main()` context.
    ///
    /// Interrupts must be disabled while calling this function.
    #[inline(always)]
    pub unsafe fn new() -> Self {
        // SAFETY: The safety contract of the called function is equal to ours.
        unsafe { Self::internal_new() }
    }

    /// Get the `CriticalSection` that belongs to this context.
    /// In the main context interrupts are enabled.
    /// Therefore, this cs can *ONLY* be used together with `MainCtxCell`.
    #[inline(always)]
    pub(crate) unsafe fn cs(&self) -> CriticalSection<'cs> {
        self.0
    }
}

impl<'cs> IrqCtx<'cs> {
    /// Create a new interrupt context.
    ///
    /// # Safety
    ///
    /// This constructor may only be called from interrupt context.
    ///
    /// Interrupts must be disabled while calling this function.
    #[inline(always)]
    pub unsafe fn new() -> Self {
        // SAFETY: The safety contract of the called function is equal to ours.
        unsafe { Self::internal_new() }
    }

    /// Get the `CriticalSection` that belongs to this context.
    /// In IRQ context interrupts are disabled.
    /// Therefore, this cs can be used for any critical section work.
    #[inline(always)]
    pub fn cs(&self) -> CriticalSection<'cs> {
        self.0
    }
}

/// Main context initialization marker.
///
/// This marker does not have a pub constructor.
/// It is only created by [MainCtx].
pub struct InitCtx<'cs>(&'cs MainCtx<'cs>);

impl<'cs> InitCtx<'cs> {
    /// Construct a new `InitCtx`.
    ///
    /// # Safety
    ///
    /// This may only be called from the `MainCtx` constructor.
    #[inline(always)]
    unsafe fn new(main_ctx: &'cs MainCtx<'cs>) -> Self {
        Self(main_ctx)
    }

    /// Get the `CriticalSection` that belongs to this context.
    /// In initialization context interrupts are disabled.
    /// Therefore, this cs can be used for any critical section work.
    #[inline(always)]
    pub fn cs(&self) -> CriticalSection<'cs> {
        // SAFETY: [MainCtx::new_with_init] guarantees that interrupts are disabled.
        unsafe { self.0.cs() }
    }

    /// Get a reference to the `MainCtx` that is only valid during the lifetime
    /// of this `InitCtx`.
    /// This is useful for initializing ordinary `MainCtx` protected data.
    #[inline(always)]
    pub fn main_ctx(&self) -> &'cs MainCtx<'cs> {
        self.0
    }
}

impl<'cs> MainCtx<'cs> {
    /// Create a new `main()` context
    /// and run an initialization function `ini_fn` under [InitCtx] context.
    ///
    /// The `ini_fn` can have one argument of arbitary type
    /// and it returns one value of arbitrary type.
    /// If you don't need an argument or return value, just use `()`.
    ///
    /// # Safety
    ///
    /// The safety contract of [MainCtx::new] must be upheld.
    ///
    /// Interrupts must be disabled while calling this function.
    #[inline(always)]
    pub unsafe fn new_with_init<ARG, RET, F: for<'a> FnOnce(&'a InitCtx, ARG) -> RET>(
        ini_fn: F,
        ini_fn_arg: ARG,
    ) -> (Self, RET) {
        // SAFETY: The safety contract of `new_with_init` is equal to `MainCtx::new()`.
        // Our caller must ensure the safety contract.
        let main_ctx = unsafe { MainCtx::new() };

        let ret = {
            // SAFETY: We are still running before the main loop with interrupts disabled,
            // therefore construction of the `InitCtx` is sound.
            let init_ctx = unsafe { InitCtx::new(&main_ctx) };
            ini_fn(&init_ctx, ini_fn_arg)
        };

        (main_ctx, ret)
    }
}

/// Define a new `main()` loop together with corresponding `InitCtx` and `MainCtx`.
///
/// The init function can return a variable of arbitrary type
/// that is passed as-is to the main function.
/// This can typically be used for forwarding of peripheral elements.
///
/// The return type of the init function and the second argument of the main function must be the same type.
///
/// # Simple example use, without peripherals
///
/// ```
/// use avr_context::{InitCtx, MainCtx, define_main};
///
/// struct MainPeripherals { }
///
/// fn my_main_function(c: &MainCtx<'_>, dp: MainPeripherals) -> ! {
///     loop {
///         // ...
///     }
/// }
///
/// struct InitPeripherals { }
///
/// fn my_init_function(c: &InitCtx<'_>, dp: InitPeripherals) -> MainPeripherals {
///     // ...
///
///     MainPeripherals { /* ... */ }
/// }
///
/// define_main! {
///     device: atmega328p,
///     main: my_main_function,
///     enable_interrupts: true,
///     init: my_init_function(ctx, InitPeripherals { }) -> MainPeripherals,
///     static_peripherals: { },
/// }
/// ```
///
/// # Move peripherals into static variables
///
/// ```ignore
/// use avr_context::{InitCtx, MainCtx, define_main};
///
/// struct MainPeripherals { }
///
/// fn my_portb_function() {
///     interrupt::free(|cs| {
///         let portb = DP_PORTB.as_ref_with_cs(cs);
///         // ...
///     });
/// }
///
/// fn my_main_function(c: &MainCtx<'_>, dp: MainPeripherals) -> ! {
///     loop { /* ... */ }
/// }
///
/// struct InitPeripherals { }
///
/// fn my_init_function(c: &InitCtx<'_>, dp: InitPeripherals) -> MainPeripherals {
///     MainPeripherals { /* ... */ }
/// }
///
/// define_main! {
///     device: atmega328p,
///     main: my_main_function,
///     enable_interrupts: true,
///     init: my_init_function(ctx, InitPeripherals { }) -> MainPeripherals,
///     static_peripherals: {
///         static DP_PORTB: PORTB,
///     },
/// }
/// ```
#[macro_export]
macro_rules! define_main {
    (
        device: $microcontroller:ident,
        main: $main_fn:path,
        enable_interrupts: $enable_interrupts:literal,
        init: $init_fn:ident(
            ctx,
            $init_arg_type:ident {
                $(
                    $init_dp_item:ident
                ),* $(,)?
            }
        ) -> $init_fn_ret:path,
        static_peripherals: {
            $(
                static $static_dp_name:ident: $static_dp_item:ident
            ),* $(,)?
        } $(,)?
    ) => {
        // Make all static_peripherals variables visible
        // from this macro's calling namespace.
        $(
            #[cfg(target_arch = "avr")]
            pub use __avr_context__::$static_dp_name;
        )*

        #[cfg(target_arch = "avr")]
        #[doc(hidden)]
        mod __avr_context__ {
            use super::*;
            extern crate avr_device;

            $(
                // SAFETY:
                // This macro ensures that all static_peripherals variables
                // are always initialized below in `init_function`.
                pub static $static_dp_name: $crate::InitCtxCell<
                    avr_device::$microcontroller::$static_dp_item
                > = unsafe { $crate::InitCtxCell::uninit() };
            )*

            #[inline(always)]
            fn init_function(
                c: &$crate::InitCtx<'_>,
                dp: avr_device::$microcontroller::Peripherals
            ) -> $init_fn_ret {
                // Initialize all static_peripherals variables.
                $(
                    $static_dp_name.init(c, dp.$static_dp_item);
                )*

                // Call the user init function.
                $init_fn(
                    c,
                    $init_arg_type {
                        $(
                            $init_dp_item: dp.$init_dp_item,
                        )*
                    }
                )
            }

            #[avr_device::entry]
            fn main() -> ! {
                // SAFETY:
                // We are guaranteed to be the first one to get `Peripherals`
                // because we are in `avr_device::entry`.
                let dp = unsafe { avr_device::$microcontroller::Peripherals::steal() };

                // SAFETY:
                // We are before the main loop with interrupts still disabled.
                let (main_ctx, init_ret) = unsafe {
                    $crate::MainCtx::new_with_init(init_function, dp)
                };

                if $enable_interrupts {
                    // SAFETY:
                    // This is after construction of `MainCtx`
                    // and after initialization of static `InitCtx` variables.
                    unsafe { avr_device::interrupt::enable() };
                }

                // Enter the main loop.
                $main_fn(&main_ctx, init_ret)
            }
        }
    };
}

/// Define an interrupt service routine (ISR).
/// # Example use
///
/// ```
/// use avr_context::{IrqCtx, define_isr};
///
/// #[inline(always)]
/// fn timer1_compa_isr(c: &IrqCtx<'_>) {
///     // ...
/// }
///
/// define_isr! {
///     device: atmega328p,         // The name of the microcontroller
///     interrupt: TIMER1_COMPA,    // The name of the interrupt
///     isr: timer1_compa_isr,      // The interrupt service routine (ISR)
/// }
/// ```
#[macro_export]
macro_rules! define_isr {
    (
        device: $microcontroller:ident,
        interrupt: $interrupt:ident,
        isr: $isr:path $(,)?
    ) => {
        #[cfg(target_arch = "avr")]
        #[allow(non_snake_case)]
        #[doc(hidden)]
        mod $interrupt {
            extern crate avr_device;
            use super::*;

            #[avr_device::interrupt($microcontroller)]
            fn $interrupt() {
                // SAFETY: We are executing in interrupt context.
                // It is safe to construct `IrqCtx` here.
                let c = unsafe { $crate::IrqCtx::new() };

                $isr(&c);
            }
        }
    };
}

// vim: ts=4 sw=4 expandtab
