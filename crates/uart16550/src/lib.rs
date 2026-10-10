//! Minimal polled driver for 16550-compatible UARTs (PC COM ports), used by the New OS kernel
//! for early boot diagnostics.
//!
//! Scope of this crate (milestone 1):
//!
//! - program one UART at a caller-supplied I/O port base for 8 data bits, no parity, 1 stop
//!   bit and a requested baud rate;
//! - transmit bytes by polling the line status register;
//! - never block forever: a missing or faulty UART causes bytes to be dropped rather than
//!   hanging the kernel.
//!
//! Not implemented here, and therefore not claimed: interrupts, receive paths, hardware flow
//! control, break/error reporting, DMA and multi-UART management.
//!
//! The baud-rate arithmetic is deliberately separated from the port I/O so that it can be unit
//! tested on the host without touching hardware.

// The kernel and this driver are `no_std`. Host `cargo test` needs `std` for the test harness,
// so `no_std` is only applied when the crate is not being compiled for tests. This affects the
// test profile only; every real build (kernel or a future user-mode tool) is `no_std`.
#![cfg_attr(not(test), no_std)]
#![forbid(unsafe_op_in_unsafe_fn)]

use core::arch::asm;

/// Clock of the standard PC 16550 UART (1.8432 MHz). Baud divisor = clock / (16 * baud).
pub const PC_UART_CLOCK_HZ: u32 = 1_843_200;

/// Upper bound on line-status polls before a transmit attempt gives up and reports a drop.
///
/// Bounded on purpose: if the UART is absent or broken, the line status register can read as
/// permanently busy. A bounded wait turns that hardware fault into a reported dropped byte
/// instead of an unbootable hang.
const TX_READY_POLL_LIMIT: u32 = 1_000_000;

/// Register offsets from the UART base port.
mod register {
    /// Transmit/receive data (or divisor latch low byte when DLAB = 1).
    pub const DATA: u16 = 0;
    /// Interrupt enable (or divisor latch high byte when DLAB = 1).
    pub const INTERRUPT_ENABLE: u16 = 1;
    /// FIFO control.
    pub const FIFO_CONTROL: u16 = 2;
    /// Line control (DLAB is bit 7).
    pub const LINE_CONTROL: u16 = 3;
    /// Modem control.
    pub const MODEM_CONTROL: u16 = 4;
    /// Line status.
    pub const LINE_STATUS: u16 = 5;
}

/// Line status register bit: transmit holding register empty (ready to accept a byte).
const LSR_THR_EMPTY: u8 = 0x20;

/// Computes the 16-bit baud divisor for `baud` at `clock_hz`.
///
/// `divisor = clock_hz / (16 * baud)`. Returns `None` rather than panicking when the request
/// cannot be represented by the hardware:
///
/// - `baud == 0` is invalid (there is no meaningful divisor);
/// - a divisor of `0` means the requested baud rate is faster than the UART clock allows;
/// - a divisor above `u16::MAX` means the requested baud rate is slower than the UART can express.
///
/// The widening to `u64` keeps the arithmetic exact for every `u32` input: `16 * u32::MAX` fits
/// in a `u64`, so no overflow or truncation is possible.
pub const fn divisor_for_baud(clock_hz: u32, baud: u32) -> Option<u16> {
    if baud == 0 {
        return None;
    }
    let divisor = clock_hz as u64 / (16 * baud as u64);
    if divisor == 0 || divisor > u16::MAX as u64 {
        None
    } else {
        Some(divisor as u16)
    }
}

/// Outcome of a single transmit attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub enum TxOutcome {
    /// The byte was handed to the UART's transmit register.
    Sent,
    /// The UART never reported a free transmit register within `TX_READY_POLL_LIMIT` polls, so
    /// the byte was dropped. The UART is probably absent or faulty.
    Dropped,
}

/// Writes `value` to the I/O port `port`.
///
/// # Safety
///
/// The caller must be running with the privilege to perform port I/O (ring 0 on x86-64) and must
/// be the only agent using `port`, because the I/O address space is shared global state outside
/// Rust's memory model.
unsafe fn outb(port: u16, value: u8) {
    // SAFETY: `out` writes to the separate x86 I/O address space, not to memory, so it cannot
    // violate any Rust memory invariant. `nomem` and `nostack` assert that no memory is read or
    // written; `preserves_flags` asserts that the instruction leaves EFLAGS unchanged, which is
    // true for `out`. Privilege and ownership are the caller's responsibility (see this
    // function's safety contract).
    unsafe {
        asm!(
            "out dx, al",
            in("dx") port,
            in("al") value,
            options(nomem, nostack, preserves_flags)
        );
    }
}

/// Reads one byte from the I/O port `port`.
///
/// # Safety
///
/// Same contract as [`outb`]: privileged port I/O, and `port` must be owned by the caller.
unsafe fn inb(port: u16) -> u8 {
    let value: u8;
    // SAFETY: `in` reads the x86 I/O address space, not memory. It writes only to the register
    // named by `out("al")`, which the compiler accounts for. Privilege and ownership are the
    // caller's responsibility (see this function's safety contract).
    unsafe {
        asm!(
            "in al, dx",
            in("dx") port,
            out("al") value,
            options(nomem, nostack, preserves_flags)
        );
    }
    value
}

/// A polled 16550-compatible serial port.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SerialPort {
    base: u16,
}

impl SerialPort {
    /// Creates a handle for the UART whose register block starts at I/O port `base`.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that (a) it runs with privileged port I/O access, and (b) no
    /// other driver, task or firmware component uses the UART at `base` any more. Once this
    /// handle exists, the safe methods below perform port I/O without further checks, so a
    /// violated contract means two agents driving one device.
    pub const unsafe fn new(base: u16) -> Self {
        Self { base }
    }

    /// The I/O port base this handle was created for.
    pub const fn base(&self) -> u16 {
        self.base
    }

    /// Programs 8 data bits, no parity, 1 stop bit, FIFO enabled, no interrupts, and the given
    /// baud rate.
    ///
    /// Returns `false` if the baud rate cannot be represented (see [`divisor_for_baud`]), in which
    /// case no register is written. No data is transmitted or received by this function.
    pub fn init(&self, baud: u32) -> bool {
        let Some(divisor) = divisor_for_baud(PC_UART_CLOCK_HZ, baud) else {
            return false;
        };
        let low = (divisor & 0x00FF) as u8;
        let high = (divisor >> 8) as u8;

        // SAFETY: `new` established that this handle owns `base` and that port I/O is permitted,
        // so writing this UART's own registers cannot affect another device.
        unsafe {
            outb(self.base + register::INTERRUPT_ENABLE, 0x00); // polled mode: no interrupts
            outb(self.base + register::LINE_CONTROL, 0x80); // DLAB = 1 to program the divisor
            outb(self.base + register::DATA, low);
            outb(self.base + register::INTERRUPT_ENABLE, high);
            outb(self.base + register::LINE_CONTROL, 0x03); // DLAB = 0, 8N1
            outb(self.base + register::FIFO_CONTROL, 0xC7); // enable and clear FIFOs
            outb(self.base + register::MODEM_CONTROL, 0x03); // assert DTR and RTS
        }
        true
    }

    /// Transmits one byte, waiting (bounded) for the transmit holding register to become empty.
    pub fn write_byte(&self, byte: u8) -> TxOutcome {
        for _ in 0..TX_READY_POLL_LIMIT {
            // SAFETY: `new` established that this handle owns `base` and that port I/O is
            // permitted. Reading the line status register of our own UART is side-effect free.
            if unsafe { inb(self.base + register::LINE_STATUS) } & LSR_THR_EMPTY != 0 {
                // SAFETY: as above; the transmit register accepts one byte at a time, and the
                // status check above showed it is free.
                unsafe { outb(self.base + register::DATA, byte) };
                return TxOutcome::Sent;
            }
        }
        TxOutcome::Dropped
    }

    /// Transmits every byte of `text`, ignoring individual drops. Use [`write_byte`] directly when
    /// the caller needs to know whether output was lost.
    ///
    /// [`write_byte`]: SerialPort::write_byte
    pub fn write_str(&self, text: &str) {
        for byte in text.bytes() {
            let _ = self.write_byte(byte);
        }
    }
}

impl core::fmt::Write for SerialPort {
    fn write_str(&mut self, text: &str) -> core::fmt::Result {
        // `fmt::Write` has no way to report a partial/dropped write other than `Err`, and a
        // dropped byte on a broken UART is not worth aborting a log line over. Callers that need
        // transmit assurance use `write_byte` and check `TxOutcome`.
        SerialPort::write_str(self, text);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn divisor_for_standard_baud_rates() {
        // Values for the standard 1.8432 MHz UART clock.
        assert_eq!(divisor_for_baud(PC_UART_CLOCK_HZ, 115_200), Some(1));
        assert_eq!(divisor_for_baud(PC_UART_CLOCK_HZ, 38_400), Some(3));
        assert_eq!(divisor_for_baud(PC_UART_CLOCK_HZ, 9_600), Some(12));
        assert_eq!(divisor_for_baud(PC_UART_CLOCK_HZ, 1_200), Some(96));
    }

    #[test]
    fn divisor_rejects_zero_baud_without_panicking() {
        assert_eq!(divisor_for_baud(PC_UART_CLOCK_HZ, 0), None);
    }

    #[test]
    fn divisor_rejects_baud_rates_outside_the_register_range() {
        // Faster than the UART clock allows: divisor would truncate to 0.
        assert_eq!(divisor_for_baud(PC_UART_CLOCK_HZ, u32::MAX), None);
        assert_eq!(divisor_for_baud(PC_UART_CLOCK_HZ, 1), None);
        // Slower than the UART can express: divisor would exceed the 16-bit register pair.
        assert_eq!(divisor_for_baud(16 * 65_536, 1), None);
    }

    #[test]
    fn divisor_accepts_the_largest_representable_value() {
        assert_eq!(divisor_for_baud(16 * 65_535, 1), Some(u16::MAX));
        // A zero clock cannot produce a valid divisor either.
        assert_eq!(divisor_for_baud(0, 9_600), None);
    }

    #[test]
    fn handle_reports_its_configured_port_without_touching_hardware() {
        // SAFETY: nothing in this test performs port I/O; `new` and `base` only store and read a
        // plain integer, so the ownership contract of `new` is irrelevant for this assertion.
        let port = unsafe { SerialPort::new(0x03F8) };
        assert_eq!(port.base(), 0x03F8);
    }
}
