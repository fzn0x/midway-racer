/// The system bus the CPU uses to reach memory and I/O devices.
///
/// The 8080 has a 16-bit memory address space and a separate 8-bit I/O
/// port space. Implementations decide what lives where: ROM, RAM, mirrors,
/// memory-mapped registers and port-mapped devices.
///
/// Every method takes `&mut self` because, on real hardware, even reads can
/// have side effects (latches, acknowledge strobes, …).
pub trait Bus {
    /// Reads the byte at `address` in the memory space.
    fn read(&mut self, address: u16) -> u8;

    /// Writes `value` at `address` in the memory space.
    fn write(&mut self, address: u16, value: u8);

    /// Reads a byte from I/O `port` (the `IN` instruction).
    fn input(&mut self, port: u8) -> u8;

    /// Writes `value` to I/O `port` (the `OUT` instruction).
    fn output(&mut self, port: u8, value: u8);
}
