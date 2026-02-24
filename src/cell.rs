// -*- coding: utf-8 -*-
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (C) 2025 - 2026 Michael Büsch <m@bues.ch>

//! Cell container types.

use crate::{
    CriticalSection, Mutex,
    context::{InitCtx, IrqCtx, MainCtx},
};
use core::{
    cell::{Cell, UnsafeCell},
    mem::{MaybeUninit, transmute_copy},
};

/// Lazy initialization of static variables.
#[repr(transparent)]
pub struct InitCtxCell<T>(UnsafeCell<MaybeUninit<T>>);

impl<T> InitCtxCell<T> {
    /// Get an uninitialized instance of [InitCtxCell].
    ///
    /// # SAFETY
    ///
    /// It must be ensured that the returned instance is initialized
    /// with a call to [Self::init] during construction of the [MainCtx].
    /// See [MainCtx::new_with_init].
    ///
    /// Using this object in any way before initializing it will
    /// result in Undefined Behavior.
    ///
    /// It must be ensured that the returned instance is initialized
    /// with a call to [Self::init] before the object is dropped.
    ///
    /// Not initializing this object before dropping will
    /// result in Undefined Behavior.
    #[inline(always)]
    pub const unsafe fn uninit() -> Self {
        Self(UnsafeCell::new(MaybeUninit::uninit()))
    }

    /// Initialize the cell with `inner` data and return a reference to it.
    ///
    /// This must be called *once* during construction of the [MainCtx] to initialize the cell.
    #[inline(always)]
    pub fn init<'ctx>(&self, _: &'ctx InitCtx, inner: T) -> &'ctx T {
        // SAFETY:
        // Initialize the MaybeUninit with `inner` data.
        //
        // This does not drop the previous inner value.
        // In case the previous inner value was uninitialized, then this is correct.
        // In case the previous inner value was initialized (init was called multiple times),
        // then this is a (safe) memory leak. :-/
        //
        // This function can only be called from single threaded `InitCtx`
        // with interrupts disabled. The `InitCtx` argument ensures that.
        // Therefore, we can overwrite the cell without data races.
        unsafe { *self.0.get() = MaybeUninit::new(inner) };

        // SAFETY: We can now access the initialized inner field.
        unsafe { (*self.0.get()).assume_init_ref() }
    }

    /// Get a reference to the inner data with the given critical section.
    #[inline(always)]
    pub fn as_ref_with_cs<'cs>(&self, _: CriticalSection<'cs>) -> &'cs T {
        // SAFETY:
        // The [Self::uninit] safety contract ensures that [Self::init] is called before us.
        // That ensures that the inner field is initialized.
        unsafe { (*self.0.get()).assume_init_ref() }
    }

    /// Get a reference to the inner data from an initialization context `InitCtx`.
    #[inline(always)]
    pub fn as_ref_with_initctx<'ctx>(&self, c: &'ctx InitCtx) -> &'ctx T {
        self.as_ref_with_cs(c.cs())
    }

    /// Get a reference to the inner data from an interrupt context `IrqCtx`.
    #[inline(always)]
    pub fn as_ref_with_irqctx<'ctx>(&self, c: &'ctx IrqCtx) -> &'ctx T {
        self.as_ref_with_cs(c.cs())
    }
}

impl<T> Drop for InitCtxCell<T> {
    #[inline(always)]
    fn drop(&mut self) {
        // SAFETY:
        // The [Self::uninit] safety contract ensures that [Self::init] is called before us.
        // That ensures that the inner field is initialized.
        unsafe { (*self.0.get()).assume_init_drop() };
    }
}

// SAFETY: If T is Send, then we can Send the whole object.
// The object only contains T state.
unsafe impl<T: Send> Send for InitCtxCell<T> {}

// SAFETY: The cell only allows access with CriticalSection.
unsafe impl<T: Send> Sync for InitCtxCell<T> {}

/// A cell that can only be accessed from `main()` context.
///
/// There is no way to access `T` from interrupt context.
/// Therefore, all allowed accesses to `T` (from main context)
/// do not need to disable interrupts or take any other measures
/// against interruption.
///
/// All accesses to `T` optimize to simple memory reads/writes.
#[repr(transparent)]
pub struct MainCtxCell<T> {
    inner: Mutex<Cell<T>>,
}

impl<T> MainCtxCell<T> {
    /// Create a new `MainCtxCell` with the given initial value.
    #[inline(always)]
    pub const fn new(inner: T) -> Self {
        Self {
            inner: Mutex::new(Cell::new(inner)),
        }
    }

    /// Replace the inner value with `inner` and return the old value.
    #[inline(always)]
    pub fn replace(&self, m: &MainCtx<'_>, inner: T) -> T {
        // SAFETY: We only use the cs for the main context, where it is allowed to be used.
        self.inner.borrow(unsafe { m.cs() }).replace(inner)
    }

    /// Get a reference to the inner data from a main context `MainCtx`.
    #[inline(always)]
    pub fn as_ref<'cs>(&self, m: &MainCtx<'cs>) -> &'cs T {
        // SAFETY: The returned reference is bound to the
        // lifetime of the CriticalSection.
        // We only use the cs for the main context, where it is allowed to be used.
        unsafe { &*self.inner.borrow(m.cs()).as_ptr() as _ }
    }
}

impl<T: Copy> MainCtxCell<T> {
    /// Create a new `MainCtxCell` array with the given initial value copied into all elements.
    pub const fn new_array<const N: usize>(inner: T) -> [Self; N] {
        let mut ret: [MaybeUninit<Self>; N] = [const { MaybeUninit::uninit() }; N];
        let mut i = 0;
        while i < N {
            ret[i].write(Self::new(inner));
            i += 1;
        }
        // SAFETY:
        // We would like to use MaybeUninit::array_assume_init, but that is not yet stable.
        // MaybeUninit is repr(transparent), doesn't invoke Drop
        // and all elements are initialized, so transmute_copy is safe.
        unsafe { transmute_copy(&ret) }
    }

    /// Get a copy of the inner data from a main context `MainCtx`.
    #[inline(always)]
    pub fn get(&self, m: &MainCtx<'_>) -> T {
        // SAFETY: We only use the cs for the main context, where it is allowed to be used.
        self.inner.borrow(unsafe { m.cs() }).get()
    }

    /// Set the inner data from a main context `MainCtx`.
    #[inline(always)]
    pub fn set(&self, m: &MainCtx<'_>, inner: T) {
        // SAFETY: We only use the cs for the main context, where it is allowed to be used.
        self.inner.borrow(unsafe { m.cs() }).set(inner);
    }
}

// vim: ts=4 sw=4 expandtab
