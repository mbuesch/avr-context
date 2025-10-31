// -*- coding: utf-8 -*-
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (C) 2025 Michael Büsch <m@bues.ch>

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
            unsafe fn internal_new() -> Self {
                #[cfg(not(target_arch = "avr"))]
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
pub struct InitCtx(());

impl InitCtx {
    /// Get the `CriticalSection` that belongs to this context.
    /// In initialization context interrupts are disabled.
    /// Therefore, this cs can be used for any critical section work.
    #[inline(always)]
    pub fn cs<'cs>(&self) -> CriticalSection<'cs> {
        // SAFETY: [MainCtx::new_with_init] guarantees that interrupts are disabled.
        unsafe { CriticalSection::new() }
    }
}

impl<'cs> MainCtx<'cs> {
    /// Create a new `main()` context
    /// and run some initializations under [InitCtx] context.
    ///
    /// # Safety
    ///
    /// The safety contract of [MainCtx::new] must be upheld.
    ///
    /// Interrupts must be disabled while calling this function.
    #[inline(always)]
    pub unsafe fn new_with_init<'a, A, F: FnOnce(&'a InitCtx, A)>(f: F, arg: A) -> Self {
        // SAFETY: We are creating the MainCtx.
        // Therefore, it's safe to construct the InitCtx marker.
        f(&InitCtx(()), arg);

        // SAFETY: The safety contract of the called function is equal to ours.
        unsafe { MainCtx::new() }
    }
}

// vim: ts=4 sw=4 expandtab
