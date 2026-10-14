//! Runs the classic CP/M CPU diagnostics against the core, checking both
//! their verdicts and the exact instruction and cycle counts that other
//! cycle-accurate emulators report for them.
//!
//! The programs are not redistributed with this repository. Fetch them with
//! `scripts/fetch-cpu-tests.sh`, then run (the exerciser takes minutes
//! without optimizations):
//!
//! ```sh
//! cargo test --release -p i8080 --test cpm -- --ignored
//! ```
//!
//! Set `I8080_TEST_ROMS` to read the programs from another directory.

use std::path::PathBuf;
use std::{env, fs};

use i8080::{Bus, Cpu};

/// Where CP/M loads programs: the start of the transient program area.
const TPA: usize = 0x0100;
/// The entry point programs call for BDOS services.
const BDOS: u16 = 0x0005;

/// A bare CP/M environment: 64 KiB of RAM whose warm boot vector and BDOS
/// entry are patched with `OUT` instructions, like the harness of
/// <https://github.com/superzazu/8080>, so that cycle counts compare.
struct CpmMachine {
    memory: Vec<u8>,
    finished: bool,
}

impl Bus for CpmMachine {
    fn read(&mut self, address: u16) -> u8 {
        self.memory[usize::from(address)]
    }

    fn write(&mut self, address: u16, value: u8) {
        self.memory[usize::from(address)] = value;
    }

    fn input(&mut self, _port: u8) -> u8 {
        0
    }

    fn output(&mut self, port: u8, _value: u8) {
        // Port 0 is the warm boot: the program is done. Port 1 marks a BDOS
        // call, which the harness serviced just before the OUT executed.
        if port == 0 {
            self.finished = true;
        }
    }
}

struct Outcome {
    console: String,
    instructions: u64,
    cycles: u64,
}

fn run_program(name: &str) -> Outcome {
    let directory = env::var_os("I8080_TEST_ROMS").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/roms"),
        PathBuf::from,
    );
    let path = directory.join(name);
    let program = fs::read(&path).unwrap_or_else(|error| {
        panic!(
            "cannot read {}: {error}\nfetch the CP/M test programs with scripts/fetch-cpu-tests.sh",
            path.display()
        )
    });

    let mut memory = vec![0; 0x1_0000];
    memory[TPA..TPA + program.len()].copy_from_slice(&program);
    memory[0x0000..0x0002].copy_from_slice(&[0xd3, 0x00]); // OUT 0
    memory[0x0005..0x0008].copy_from_slice(&[0xd3, 0x01, 0xc9]); // OUT 1 ; RET

    let mut machine = CpmMachine {
        memory,
        finished: false,
    };
    let mut cpu = Cpu::new();
    cpu.registers.pc = TPA as u16;
    let mut console = String::new();
    let mut instructions = 0;
    while !machine.finished {
        if cpu.registers.pc == BDOS {
            bdos(&cpu, &machine.memory, &mut console);
        }
        cpu.step(&mut machine);
        instructions += 1;
    }
    println!("{console}");

    Outcome {
        console,
        instructions,
        cycles: cpu.cycles(),
    }
}

/// Implements the two console functions the diagnostics use.
fn bdos(cpu: &Cpu, memory: &[u8], console: &mut String) {
    match cpu.registers.c {
        // C_WRITE: print the character in E.
        2 => console.push(char::from(cpu.registers.e)),
        // C_WRITESTR: print the string at DE, terminated by '$'.
        9 => console.extend(
            memory[usize::from(cpu.registers.de())..]
                .iter()
                .take_while(|&&byte| byte != b'$')
                .map(|&byte| char::from(byte)),
        ),
        _ => {}
    }
}

#[test]
#[ignore = "needs the CP/M test programs, see scripts/fetch-cpu-tests.sh"]
fn tst8080_reports_an_operational_cpu() {
    let outcome = run_program("TST8080.COM");
    assert!(outcome.console.contains("CPU IS OPERATIONAL"));
    assert_eq!((outcome.instructions, outcome.cycles), (651, 4_924));
}

#[test]
#[ignore = "needs the CP/M test programs, see scripts/fetch-cpu-tests.sh"]
fn preliminary_exerciser_completes() {
    let outcome = run_program("8080PRE.COM");
    assert!(outcome.console.contains("8080 Preliminary tests complete"));
    assert_eq!((outcome.instructions, outcome.cycles), (1_061, 7_817));
}

#[test]
#[ignore = "needs the CP/M test programs, see scripts/fetch-cpu-tests.sh"]
fn supersoft_diagnostic_passes() {
    let outcome = run_program("CPUTEST.COM");
    assert!(outcome.console.contains("CPU TESTS OK"));
    assert_eq!(
        (outcome.instructions, outcome.cycles),
        (33_971_311, 255_653_383)
    );
}

#[test]
#[ignore = "needs the CP/M test programs, see scripts/fetch-cpu-tests.sh"]
fn instruction_exerciser_passes_every_group() {
    let outcome = run_program("8080EXM.COM");
    assert!(!outcome.console.contains("ERROR"));
    assert_eq!(outcome.console.matches("PASS!").count(), 25);
    assert_eq!(
        (outcome.instructions, outcome.cycles),
        (2_919_050_698, 23_803_381_171)
    );
}
