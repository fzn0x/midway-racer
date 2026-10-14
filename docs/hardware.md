# The Midway 8080 racing board

*Laguna Racer* (Midway, 1977, PCB #622) and *280-ZZZAP* (Dave Nutting
Associates / Midway, 1976, PCB #610) run on Midway's 8080-based
black-and-white board, the same family as *Space Invaders*. This page
summarises what the emulator models and where each detail comes from.

## Clocks and timing

Everything derives from a 19.968 MHz crystal:

| Signal      | Divider | Frequency    |
| ----------- | ------- | ------------ |
| CPU clock   | ÷ 10    | 1.9968 MHz   |
| Pixel clock | ÷ 4     | 4.992 MHz    |
| Scan line   | 320 px  | 15.6 kHz     |
| Frame       | 262 ln  | 59.54 Hz     |

A scan line is therefore exactly 128 CPU cycles and a frame 33,536 cycles.
`mw8080bw::Machine` runs the CPU line by line against absolute cycle
targets, so instruction overshoot never accumulates into drift.

## Memory map

Only address lines A0–A14 are decoded, so the map repeats every 32 KiB.

| Address       | Contents                                             |
| ------------- | ---------------------------------------------------- |
| `0000`–`1FFF` | Program ROM                                          |
| `2000`–`3FFF` | 8 KiB RAM; `2400`–`3FFF` is scanned out as video RAM |
| `4000`–`5FFF` | Second ROM area, unpopulated on these boards         |
| `6000`–`7FFF` | Mirror of the RAM                                    |

## I/O ports

Port addresses use three bits; reads of ports 4–7 mirror ports 0–3.

| Port | Read                                   | Write                         |
| ---- | -------------------------------------- | ----------------------------- |
| 0    | Pedal, gear, coin, start (see below)   |                               |
| 1    | Steering wheel                         |                               |
| 2    | DIP switches                           | Sound latch 1 (engine)        |
| 3    | Barrel shifter result                  | Barrel shifter data           |
| 4    |                                        | Barrel shifter amount         |
| 5    |                                        | Sound latch 2 (effects)       |
| 7    |                                        | Watchdog reset                |

Input port 0 holds the 4-bit accelerator position in bits 0–3, the gear
shifter in bit 4 (set in high gear), the coin switch in bit 6 and the start
button in bit 7, both active low. The steering wheel is a potentiometer
read as `7F` when centred, falling to `01` at full right lock and rising to
`FE` at full left lock.

## Interrupts

A flip-flop drives the CPU's INT line twice per frame, and only latches
while the CPU's INTE output is high, so requests made with interrupts
disabled are lost:

| Line | Vertical counter       | Instruction    |
| ---- | ---------------------- | -------------- |
| 96   | `80h`, outside blanking | `RST 1` (`CF`) |
| 224  | `DAh`, inside blanking  | `RST 2` (`D7`) |

## Video

Video RAM is clocked out a byte per eight pixels, least significant bit
first. The shift register is first loaded four pixels into each line and
keeps shifting for four pixels after blanking starts, so the raster is 260
pixels wide with the bitmap starting at column 4, as MAME draws it. Laguna
Racer's monitor is mounted turned 90° clockwise; 280-ZZZAP's is upright,
with a blue film over its bottom 43 lines.

## Barrel shifter

The MB14241 (or its TTL equivalent) keeps the last two bytes written to
port 3 and returns the eight bits found the port 4 amount below the top of
that 16-bit register.

## Watchdog

A 555 monostable (1 MΩ, 1 µF: 1.1 s) resets the CPU unless the program
writes to port 7 in time.

## Sound

The sound board is analogue. MAME simulates it component by component
with a netlist; this emulator models each stage's behaviour instead, from
the component values in that netlist and the schematics.

Latch 1 (port 2):

| Bits | Meaning                                       |
| ---- | --------------------------------------------- |
| 0–3  | Accelerator level, setting the engine's pitch |
| 4    | High gear                                     |
| 5    | Low gear                                      |

Latch 2 (port 5):

| Bit | Meaning                        |
| --- | ------------------------------ |
| 0   | Boom, on crashing              |
| 1   | Engine sound off               |
| 2   | Tyre skid                      |
| 3   | Screech that follows a crash   |
| 5   | Coin counter                   |

- **Engine.** The accelerator level charges capacitor C18 (22 µF on Laguna
  Racer, 47 µF on 280-ZZZAP) through 47 kΩ and discharges it faster through
  a diode and 10 kΩ, which gives the engine its inertia. Op-amp `H5_4`
  turns its voltage into a drive for three triangle-wave oscillators that
  peak at 87, 343 and 511 Hz (465 Hz on 280-ZZZAP). The two slower ones are
  combined through diodes, and the fastest sweeps the control pin of an
  MC3340 attenuator, chopping the signal into the familiar buzz. In high
  gear, capacitor C19 first halves the drive and then lets it recover over
  a couple of seconds.
- **Noise.** A zener diode's noise, amplified until it flips between the
  rails, feeds three active filters: a 1 kHz band-pass with a Q of 25 for
  skids, a saturated 1 kHz low-pass for the post-crash screech, and a 60 Hz
  band-pass with a Q of 19 and a gain of 200 for the boom, which keeps
  ringing after its input is switched off.
- **Mixing.** The effects are summed through their mixing resistors and
  AC-coupled to the output amplifier.

Latch writes are time-stamped with the CPU cycle they happen at, and the
synthesizer applies each one at the matching sample.

## Sources

- MAME's `mw8080bw` driver (BSD-3-Clause), by Nicola Salmoria, Tormod
  Tjaberg, Mirko Buffoni, Lee Taylor, Valerio Verrando, Zsolt Vasvari and
  others, for the memory and I/O maps, timing, interrupts and ROM sets.
- MAME's `nl_280zzzap` netlist (CC0) and its notes, by Colin Douglas
  Howell, for the sound circuits.
- Intel's *8080 Microcomputer Systems User's Manual* for the CPU.
