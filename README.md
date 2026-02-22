# avr-context: Static checking of execution context

This crate provides context-aware cells and context markers for AVR microcontrollers.
It helps to safely manage access to data from different execution contexts, such as the main loop and interrupt handlers.

The main goal is to provide a zero-cost or minimal-cost abstraction for safe and minimal overhead data access.
The exclusive access of data from the main loop is zero-cost.

This crate is built upon `Mutex` and `CriticalSection` from the [bare-metal](https://crates.io/crates/bare-metal) and [avr-device](https://crates.io/crates/avr-device) crates.

## Contexts

The crate defines three context markers:

- `MainCtx`: Possession of a reference to this marker guarantees execution from the `main()` context.
- `IrqCtx`: Possession of a reference to this marker guarantees execution from an interrupt context.
- `InitCtx`: A special context for initializing data before the main loop has started.

## Cells

Two main cell types are provided:

- `MainCtxCell`: A cell that can only be accessed from the `main()` context.
  Accesses from interrupt context are prevented at compile time.
  This means that no interrupt disabling is required to access the data, which makes the access very efficient.
- `InitCtxCell`: A cell for lazy initialization of static variables.
  It is guaranteed that the data is initialized before it is accessed from the main loop.
  Note that this guarantee must currently be manually checked and therefore requires one `unsafe` block.

## Usage

`Cargo.toml`:

```toml
[dependencies]
avr-context = "1"
avr-device = { version = "0.8", features = [ "atmega328p", "rt" ] }
```

`main.rs`:

```rust
use avr_context::{InitCtx, IrqCtx, MainCtx, MainCtxCell};

/// This global variable can only be accessed from `main()` context.
static COUNTER: MainCtxCell<u16> = MainCtxCell::new(0);

/// Example user function.
fn increment_counter(c: &MainCtx<'_>) {
    // Read the global counter variable.
    // We do not have to disable interrupts, because possession of the `MainCtx`
    // reference guarantees that no interrupt can touch the `COUNTER`.
    let mut counter = COUNTER.get(c);

    // In this example we just increment the counter for demonstration.
    counter = counter.wrapping_add(1);

    // Write the counter back to global storage without disabling interrupts.
    COUNTER.set(c, counter);
}

struct MainPeripherals { }

/// Main program loop; With interrupts enabled.
fn main_loop(c: &MainCtx<'_>, dp: MainPeripherals) -> ! {
    loop {
        // Put your main loop code here.

        increment_counter(c);
    }
}

struct InitPeripherals { }

/// Initialization routine before main loop.
/// This function runs with interrupts disabled.
fn init(c: &InitCtx<'_>, dp: InitPeripherals) -> MainPeripherals {

    // Put your initialization code here.

    MainPeripherals { }
}

avr_context::define_main! {
    device: atmega328p,
    main: main_loop,
    enable_interrupts: true,
    init: init_function(ctx, InitPeripherals { }) -> MainPeripherals,
    static_peripherals: { },
}

/// Interrupt entry point.
fn timer1_compa_isr(c: &IrqCtx<'_>) {
    // The following access to `COUNTER` will not compile.
    // We don't have a reference to the `MainCtx` here.

    //COUNTER.set(c, 42);
}

avr_context::define_isr! {
    device: atmega328p,
    interrupt: TIMER1_COMPA,
    isr: timer1_compa_isr,
}
```

## Passing data between interrupt service routine and main.

This crate does not provide primitives for synchronizing or sending data between ISR and main contexts.
The purpose of this crate is the opposite use case:
If you have data that shall never be accessed from interrupt context, then put it under `MainCtxCell` protection.

However, communication between ISR and main context is often required of course.
There are multiple safe ways to do that.

For example an `avr-device` `Mutex` can be used for interrupt safe synchronized access.
Note that `avr-context`'s `IrqCtx` does provide a `cs()` method to obtain a `CriticalSection` that can be used with `Mutex` to access isr/main shared variables.
See the `avr-device` documentation for more information and examples.

One other option would be to use an atomic.
For example an atomic from the [avr-atomic](https://crates.io/crates/avr-atomic) crate or from the `core` library.
Atomics from the `core` library are heavier on runtime and code size than `avr-atomic`, but they also have more features.

## Non-AVR target architectures

This crate is currently only designed to run on `target_arch = "avr"`.
It will compile on other architectures, but it will **not** work (it will panic).

This crate will never work on architectures that are multi-processor or multi-threaded.

It is possible to make this crate work on non-AVR single processor architectures.
But that is currently not planned.
If you want to work on this, please let me know by opening an issue.

## License

This crate is licensed under either of the following, at your option:

- Apache License, Version 2.0
- MIT license

Copyright (C) 2025 - 2026 Michael Büsch
