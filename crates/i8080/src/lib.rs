//! A cycle-accurate Intel 8080 CPU emulator core.
//!
//! The crate is `#![no_std]`, has no dependencies and knows nothing about
//! the machine it runs in: memory and I/O devices are reached through the
//! [`Bus`] trait, which the host system implements.
#![cfg_attr(not(test), no_std)]

mod bus;
mod cpu;
mod disasm;
mod registers;
mod timing;

pub use bus::Bus;
pub use cpu::Cpu;
pub use disasm::Instruction;
pub use registers::{Flags, Registers};
