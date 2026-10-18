//! **The console UART as the kernel grants it**: its receive interrupt, its register page, and the
//! port range `x86_64` has instead of a page.
//!
//! Cut out of `user.rs` unchanged by milestone 872 (the kernel's user-mode file is split along its
//! seams), which left the parent an index.
//!
//! Name: provisional (milestone 872's lane, 2026-10-11 UTC; §281 (the kernel's user-mode file
//! splits into child modules of `user`)).

/// The console UART's receive interrupt on QEMU `virt`. The progenitor routes and delegates it so the input
/// driver it builds (19d.2c) can wait on keystrokes. aarch64's PL011 is SPI 1 = INTID 33; RISC-V's
/// NS16550 is PLIC source 10.
///
/// **The documented fallback, not the answer.** The boot paths ask the machine first
/// ([`uart_irq_and_source`]): on the JH7110, UART0 interrupts on PLIC line 32, and a kernel that
/// armed this constant there enabled an unrelated source, proven on silicon when a key press at
/// boot 13's completed tour reached nothing (notes/visionfive2.md, BUGS). This number is what a
/// tree that does not say falls back to, which on QEMU is also the right answer.
///
/// Name: provisional, flagged 2026-09-25 by the lane that re-derived the x86 port falsifications
/// (design/naming/functions-that-answer-yes-or-no-worklist.md, "`rx` and `tx`"). calef asked what `rx` stands
/// for in his #1255 review; recommended `UART_RECEIVE_INTID`; `INTID` is the GIC's own term and
/// stays.
#[cfg(target_arch = "aarch64")]
pub const UART_RX_INTID: u32 = 33;
/// Name: provisional, flagged 2026-09-25 by the lane that re-derived the x86 port falsifications
/// (design/naming/functions-that-answer-yes-or-no-worklist.md, "`rx` and `tx`"). calef asked what `rx` stands
/// for in his #1255 review; recommended `UART_RECEIVE_INTID`; `INTID` is the GIC's own term and
/// stays.
#[cfg(target_arch = "riscv64")]
pub const UART_RX_INTID: u32 = 10;
/// `x86_64`: COM1 is ISA IRQ 4, which has been true since the PC/AT and is what QEMU's `q35`
/// presents. **What that number means depends on the interrupt controller**, and on x86 that is
/// two questions rather than one: which IO APIC input the legacy IRQ was remapped to (the ACPI
/// MADT's interrupt source overrides say, and this port does not read them), and which IDT vector
/// that input is programmed to raise. 4 is the legacy line, not either of those.
///
/// Name: provisional, flagged 2026-09-25 by the lane that re-derived the x86 port falsifications
/// (design/naming/functions-that-answer-yes-or-no-worklist.md, "`rx` and `tx`"). calef asked what `rx` stands
/// for in his #1255 review; recommended `UART_RECEIVE_INTID`; `INTID` is the GIC's own term and
/// stays.
#[cfg(target_arch = "x86_64")]
pub const UART_RX_INTID: u32 = 4;

/// The console UART's interrupt line and which source decided it: the machine's own answer when
/// it gave one (`memory::uart_irq`, a device tree on aarch64/riscv64 or ACPI on `x86_64`), else
/// [`UART_RX_INTID`], QEMU `virt`'s constant. The source string exists to be printed: a bench
/// transcript that names the number's origin is diagnosable, and the one that did not already
/// cost a boot (notes/visionfive2.md).
pub(crate) fn uart_irq_and_source() -> (u32, &'static str) {
    match crate::memory::uart_irq() {
        Some(n) => (n, "machine description"),
        None => (UART_RX_INTID, "QEMU-virt fallback; the machine did not say"),
    }
}

/// The console UART's registers, physically. aarch64 `virt` puts a PL011 at `0x0900_0000`; RISC-V
/// `virt` puts an NS16550 at `0x1000_0000`. The progenitor holds a device capability for it and delegates it
/// to the console and input drivers it builds. Matches `console::UART_PHYS`.
#[cfg_attr(not(any(test, feature = "system_tests")), allow(dead_code))]
#[cfg(target_arch = "aarch64")]
pub const UART_PHYS: u64 = 0x0900_0000;
#[cfg_attr(not(any(test, feature = "system_tests")), allow(dead_code))]
#[cfg(target_arch = "riscv64")]
pub const UART_PHYS: u64 = crate::arch::machine::CONSOLE_UART_PHYS;
/// `x86_64` has **no physical address for its console at all**: COM1 lives in the I/O port space,
/// which has no page tables in front of it, so there is nothing here for a device capability to be
/// a mapping *of*. The console is reached through [`X86_COM1_PORT_BASE`] as a `PortRange` capability
/// instead (milestone 299, DECISIONS §121 reversed 2026-09-15); this constant stays zero because a
/// port has no physical page, and the predicate below still reads it to keep a fixture from ever
/// granting a device page where there is none. See `arch/x86_64/port.rs` and `segments.rs`.
#[cfg_attr(not(any(test, feature = "system_tests")), allow(dead_code))]
#[cfg(target_arch = "x86_64")]
pub const UART_PHYS: u64 = 0;

/// **COM1's I/O ports** (milestone 299): the eight consecutive ports `0x3F8..=0x3FF` a 16550 UART
/// occupies, which QEMU's `q35` and every PC since the PC/AT put COM1 at. The progenitor is minted a
/// `PortRange(0x3F8, 8)` capability over exactly this range and delegates it to the console and input
/// drivers, so their `in`/`out` reach these ports and no others (enforced by the TSS I/O bitmap). The
/// port analogue of [`UART_PHYS`] on the other two architectures.
#[cfg(target_arch = "x86_64")]
pub(crate) const X86_COM1_PORT_BASE: u16 = 0x3F8;
/// The eight ports a 16550 occupies (data/interrupt-enable/FIFO-control/line-control/modem-control/
/// line-status/modem-status/scratch). A range because a device capability names what the hardware
/// names, the port-space twin of a `DeviceFrame` naming a page.
#[cfg(target_arch = "x86_64")]
pub(crate) const X86_COM1_PORT_COUNT: u16 = 8;

/// **Is there a UART a device capability can be a mapping of on this machine?** (milestone 161.)
///
/// [`UART_PHYS`] is zero on `x86_64` and that zero is the marker for
/// [DECISIONS §121](../../design/decisions/0121-port-io-capability.md) (PROPOSED). Several fixtures
/// need to ask, so they ask here rather than each testing a constant against zero and each writing
/// its own sentence about why.
///
/// **The trap this exists to prevent is a green test rather than a red one.** A fixture that went
/// ahead anyway would grant a device capability over *physical page zero*, map it, and read
/// real-mode interrupt-vector bytes; the read would succeed, a revoke would still fault, and a
/// suite would report that x86 has a userspace device story. It does not.
#[cfg_attr(not(feature = "system_tests"), allow(dead_code))]
pub fn machine_has_no_device_page_for_the_console() -> bool {
    UART_PHYS == 0
}

/// The reason a fixture gives when [`machine_has_no_device_page_for_the_console`] is true.
#[cfg_attr(not(feature = "system_tests"), allow(dead_code))]
pub const NO_UART_PAGE: &str = "this machine's console UART is in the I/O port space, so there is \
                                no page for a device capability to be a mapping of and no \
                                capability shape for a port yet (DECISIONS \u{a7}121)";
