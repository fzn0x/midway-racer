//! A cycle-accurate Intel 8080 CPU emulator core.
//!
//! The crate is `#![no_std]`, has no dependencies and knows nothing about
//! the machine it runs in: memory and I/O devices are reached through the
//! [`Bus`] trait, which the host system implements.
#![no_std]

mod bus;

pub use bus::Bus;
