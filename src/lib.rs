// -*- coding: utf-8 -*-
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (C) 2025 Michael Büsch <m@bues.ch>

#![cfg_attr(target_arch = "avr", no_std)]

pub mod cell;
pub mod context;

pub use crate::{
    cell::{InitCtxCell, MainCtxCell},
    context::{InitCtx, IrqCtx, MainCtx},
};

pub type CriticalSection<'cs> = bare_metal::CriticalSection<'cs>;
pub type Mutex<T> = bare_metal::Mutex<T>;

// vim: ts=4 sw=4 expandtab
