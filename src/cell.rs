// -*- coding: utf-8 -*-
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (C) 2025 Michael Büsch <m@bues.ch>

use crate::{
    Mutex,
    context::{InitCtx, MainCtx},
};
use core::{
    cell::{Cell, UnsafeCell},
    mem::MaybeUninit,
};

/// Lazy initialization of static variables.
pub struct InitCtxCell<T>(UnsafeCell<MaybeUninit<T>>);

impl<T> InitCtxCell<T> {
    /// # SAFETY
    ///
    /// It must be ensured that the returned instance is initialized
    /// with a call to [Self::init] during construction of the [MainCtx].
    /// See [MainCtx::new_with_init].
    ///
    /// Using this object in any way before initializing it will
    /// result in Undefined Behavior.
    #[inline(always)]
    pub const unsafe fn uninit() -> Self {
        Self(UnsafeCell::new(MaybeUninit::uninit()))
    }

    #[inline(always)]
    pub fn init(&self, _m: &InitCtx, inner: T) -> &T {
        // SAFETY: Initialization is required for the `assume_init` calls.
        unsafe {
            *self.0.get() = MaybeUninit::new(inner);
            (*self.0.get()).assume_init_ref()
        }
    }
}

impl<T> core::ops::Deref for InitCtxCell<T> {
    type Target = T;

    #[inline(always)]
    fn deref(&self) -> &T {
        // SAFETY: the [Self::uninit] safety contract ensures that [Self::init] is called before us.
        unsafe { (*self.0.get()).assume_init_ref() }
    }
}

impl<T> core::ops::DerefMut for InitCtxCell<T> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut T {
        // SAFETY: the [Self::uninit] safety contract ensures that [Self::init] is called before us.
        unsafe { (*self.0.get()).assume_init_mut() }
    }
}

// SAFETY: If T is Send, then we can Send the whole object.
// The object only contains T state.
unsafe impl<T: Send> Send for InitCtxCell<T> {}

// SAFETY: The `deref` and `deref_mut` functions ensure that they can only be called
// from `MainCtx` compatible contexts.
unsafe impl<T> Sync for InitCtxCell<T> {}

/// A cell that can only be accessed from `main()` context.
///
/// There is no way to access `T` from interrupt context.
/// Therefore, all allowed accesses to `T` (from main context)
/// do not need to disable interrupts or take any other measures
/// against interruption.
///
/// All accesses to `T` optimize to simple memory reads/writes.
pub struct MainCtxCell<T> {
    inner: Mutex<Cell<T>>,
}

impl<T> MainCtxCell<T> {
    #[inline(always)]
    pub const fn new(inner: T) -> Self {
        Self {
            inner: Mutex::new(Cell::new(inner)),
        }
    }

    #[inline(always)]
    pub fn replace(&self, m: &MainCtx<'_>, inner: T) -> T {
        // SAFETY: We only use the cs for the main context, where it is allowed to be used.
        self.inner.borrow(unsafe { m.cs() }).replace(inner)
    }

    #[inline(always)]
    pub fn as_ref<'cs>(&self, m: &MainCtx<'cs>) -> &'cs T {
        // SAFETY: The returned reference is bound to the
        //         lifetime of the CriticalSection.
        //         We only use the cs for the main context, where it is allowed to be used.
        unsafe { &*self.inner.borrow(m.cs()).as_ptr() as _ }
    }
}

impl<T: Copy> MainCtxCell<T> {
    #[inline(always)]
    pub fn get(&self, m: &MainCtx<'_>) -> T {
        // SAFETY: We only use the cs for the main context, where it is allowed to be used.
        self.inner.borrow(unsafe { m.cs() }).get()
    }

    #[inline(always)]
    pub fn set(&self, m: &MainCtx<'_>, inner: T) {
        // SAFETY: We only use the cs for the main context, where it is allowed to be used.
        self.inner.borrow(unsafe { m.cs() }).set(inner);
    }
}

// vim: ts=4 sw=4 expandtab
