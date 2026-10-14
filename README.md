# Midway Racer

[![CI](https://github.com/fzn0x/midway-racer/actions/workflows/ci.yml/badge.svg)](https://github.com/fzn0x/midway-racer/actions/workflows/ci.yml)
[![Deploy](https://github.com/fzn0x/midway-racer/actions/workflows/pages.yml/badge.svg)](https://github.com/fzn0x/midway-racer/actions/workflows/pages.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

A cycle-accurate Intel 8080 emulator of Midway's black-and-white arcade
board, running the 1970s racers **Laguna Racer** (1977) and **280-ZZZAP**
(1976), in the browser and on the desktop.

**Play it in your browser: <https://fzn0x.github.io/midway-racer/>**

The game ROMs are copyrighted and not included. Load your own dump, such
as MAME's `lagunar.zip`; it is checked against MAME's checksums and never
leaves your machine.

## Features

- **An exact 8080.** Every documented and undocumented opcode with its
  real timing. The core passes the classic CP/M diagnostics, `TST8080`,
  `8080PRE`, `CPUTEST` and the full `8080EXM` exerciser, with exactly the
  instruction and cycle counts other accurate emulators report (23.8
  billion cycles for `8080EXM`).
- **The board, timed like the hardware.** Frames of 33,536 cycles at
  59.54 Hz, run scan line by scan line; `RST 1` at line 96 and `RST 2` at
  line 224, gated by the CPU's INTE pin; the MB14241 barrel shifter; the
  1.1 s watchdog; and the 260-pixel raster with its four-pixel shift
  register delay. See [the hardware notes](docs/hardware.md).
- **The sound board.** Engine, tyre skid, crash boom and post-crash
  screech, modelled from the schematics: engine inertia, the high-gear
  pitch drop, the MC3340's chopped buzz and the ringing 60 Hz boom. Latch
  writes take effect at the exact sample they happen at.
- **Real controls.** A steering wheel and a 16-step accelerator, read like
  the cabinet's potentiometer and encoder, driven by keys, gamepad sticks
  and triggers, or a touch strip that steers like a wheel.
- **Faithful picture.** Laguna Racer's monitor turned on its side,
  280-ZZZAP's blue overlay, the monitor's 4:3 shape (or square pixels) and
  optional scanlines that follow the raster.
- **Friendly ROM handling.** Zip archives, loose dumps, folders or a
  single combined image; chips are recognised by checksum whatever their
  names, and every chip's status is reported.
- **Operator settings.** Every DIP switch with its real labels.
- **A CPU debugger** in the browser: registers, disassembly and
  frame-by-frame stepping.

## Playing

### Controls

| Action      | Keyboard              | Gamepad                    | Touch          |
| ----------- | --------------------- | -------------------------- | -------------- |
| Steer       | `←` `→` or `A` `D`    | Left stick, d-pad          | Drag the strip |
| Accelerate  | `↑` or `W`            | Right trigger, A           | Gas            |
| Shift gear  | `Space` or `Shift`    | X, or LB and RB for low/high | Gear         |
| Insert coin | `C` or `5`            | Back                       | Coin           |
| Start       | `1` or `Enter`        | Start                      |                |
| Pause       | `P`                   |                            |                |
| Reset       | `F3`                  |                            |                |
| Sound       | `M` mutes, `-` `=` set volume |                    |                |

In the browser, `H` shows the help, `` ` `` the CPU debugger, and `N` runs
one frame while paused.

### ROMs

| Game                | MAME set    | Files                                               |
| ------------------- | ----------- | --------------------------------------------------- |
| Laguna Racer        | `lagunar`   | `lagunar.h`, `lagunar.g`, `lagunar.f`, `lagunar.e`  |
| 280-ZZZAP           | `280zzzap`  | `zzzaph`, `zzzapg`, `zzzapf`, `zzzape`, `zzzapd`, `zzzapc` |
| 280-ZZZAP (set 2)   | `280zzzapa` | `h`, `g`, `f`                                       |

## Desktop app

Install [Rust](https://rustup.rs). On Linux, also install the ALSA, udev
and xkbcommon/Wayland development packages, for instance
`sudo apt install libasound2-dev libudev-dev libxkbcommon-dev libwayland-dev`.

```sh
cargo run --release -- path/to/lagunar.zip
```

Without a path, the app looks in a `roms` directory. Run with `--help` for
every option, such as `--scale`, `--square-pixels`, `--dip-switches`,
`--volume`, `--mute` and `--verify`, which only checks the ROMs.

## Building the web version

You need Rust, Node.js 22 and the `wasm-bindgen` CLI at the version locked
in `Cargo.lock`:

```sh
cargo install wasm-bindgen-cli --version 0.2.129
cd web
npm ci
npm run dev      # builds the WebAssembly module and serves the page
npm run build    # production build in web/dist
```

## Project layout

| Path                     | Contents                                                       |
| ------------------------ | -------------------------------------------------------------- |
| `crates/i8080`           | The 8080 CPU: `no_std`, dependency-free, with a disassembler    |
| `crates/mw8080bw`        | The board: memory, I/O, video, sound, ROM sets, controls        |
| `crates/midway-racer`    | The desktop app: window, audio, keyboard and gamepads           |
| `crates/midway-racer-web`| WebAssembly bindings for the browser                            |
| `web`                    | The browser frontend, in TypeScript                             |
| `docs/hardware.md`       | How the board works                                             |

The cores know nothing about their frontends: the CPU reaches the rest of
the machine through a `Bus` trait, and the board takes cabinet controls in
and hands a raster and audio samples out.

## Testing

```sh
cargo test --workspace
scripts/fetch-cpu-tests.sh   # downloads the CP/M diagnostics, checksum-verified
cargo test --release -p i8080 --test cpm -- --ignored
cd web && npm test
```

CI runs the test suite on Linux, Windows and macOS, and the CPU
diagnostics, rustfmt, Clippy's pedantic lints, rustdoc, a build with the
minimum supported Rust version (1.85), ESLint, Prettier and TypeScript's
strict mode on Linux.

## Acknowledgements

- [mustafasegf/intel-8080-emu](https://github.com/mustafasegf/intel-8080-emu),
  the Space Invaders emulator that inspired this project.
- MAME's `mw8080bw` driver and Colin Douglas Howell's 280-ZZZAP and Laguna
  Racer sound netlist, the main hardware references.
- [superzazu/8080](https://github.com/superzazu/8080), whose test harness
  and published cycle counts the CPU tests compare against.

## License

Licensed under either of the [Apache License, Version 2.0](LICENSE-APACHE)
or the [MIT license](LICENSE-MIT), at your option. Contributions are
accepted under the same terms. Laguna Racer and 280-ZZZAP belong to their
respective rights holders; no part of them is distributed here.
