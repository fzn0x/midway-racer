//! A cycle-accurate Intel 8080 CPU emulator core.
//!
//! The crate is `#![no_std]`, has no dependencies and knows nothing about
//! the machine it runs in: memory and I/O devices are reached through the
//! [`Bus`] trait, which the host system implements.
//!
//! Every documented and undocumented opcode is implemented with its exact
//! timing, and the core passes the classic CP/M diagnostics (`TST8080`,
//! `8080PRE`, `CPUTEST` and `8080EXM`) with the same cycle counts as other
//! accurate emulators.
//!
//! # Example
//!
//! ```
//! use i8080::{Bus, Cpu, Instruction};
//!
//! /// 64 KiB of RAM and a single output port.
//! struct Computer {
//!     memory: Vec<u8>,
//!     port: u8,
//! }
//!
//! impl Bus for Computer {
//!     fn read(&mut self, address: u16) -> u8 {
//!         self.memory[usize::from(address)]
//!     }
//!
//!     fn write(&mut self, address: u16, value: u8) {
//!         self.memory[usize::from(address)] = value;
//!     }
//!
//!     fn input(&mut self, _port: u8) -> u8 {
//!         0xff
//!     }
//!
//!     fn output(&mut self, _port: u8, value: u8) {
//!         self.port = value;
//!     }
//! }
//!
//! // MVI A,2AH ; OUT 01H ; HLT
//! let mut computer = Computer { memory: vec![0; 0x1_0000], port: 0 };
//! computer.memory[..5].copy_from_slice(&[0x3e, 0x2a, 0xd3, 0x01, 0x76]);
//!
//! let mut cpu = Cpu::new();
//! while !cpu.is_halted() {
//!     cpu.step(&mut computer);
//! }
//!
//! assert_eq!(computer.port, 0x2a);
//! assert_eq!(cpu.cycles(), 7 + 10 + 7);
//! assert_eq!(Instruction::decode(&computer.memory[2..]).to_string(), "OUT 01H");
//! ```
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
