//! **riscv64's milestone tour**: the demonstrations between the self-test and the hand-over, with
//! the timing helpers only its entropy step uses. `kernel_main` reaches it where aarch64 reaches
//! `milestone_tour.rs`, and the boot around it is `boot.rs`'s on both. See notes/riscv-port.md.
//!
//! **Temporary.** calef ruled so on 2026-10-12 (UTC), in §284 (the kernel crate root splits into
//! child modules, one per boot): this file is the port's route until it joins the shared path in
//! `boot.rs`. What is left is the tour, which calef's ruling of 2026-10-19 (UTC) moves into
//! `self_test::run` as checks; see design/roadmap/0890-riscv64-and-x86-64-join-the-shared-boot-path.md.
//!
//! Name: provisional (milestone 876's lane, 2026-10-12 UTC).

use super::*;

/// riscv64 phase 4: a device interrupt serviced by an unprivileged userspace driver.
#[cfg(target_arch = "riscv64")]
fn riscv64_device_interrupt() {
    // Device interrupts, serviced by an unprivileged userspace driver: the last piece of the
    // interrupt story, in its real form. The kernel loads `driver` from the initial ramdisk, maps
    // the NS16550 into it device-typed, and grants it an Irq capability and a report endpoint. A
    // keystroke raises a line into the PLIC; the PLIC delivers a supervisor external interrupt;
    // riscv_trap_dispatch claims it, masks the source, and notifies the endpoint the Irq cap
    // waits on; the driver (a U-mode process owning no privilege) wakes, reads the byte through
    // its own device mapping, SENDs it, and ACKs, which re-arms the source through arch::irq. The
    // kernel is never in the data path. The `cargo run` harness is non-interactive, so pipe a
    // byte to QEMU's serial *after boot*: `( sleep 4; printf A ) | ...` (the console clears the
    // RX FIFO during init, so a byte sent at t=0 is dropped).
    // The PLIC itself came up in `arch::irq::init`, in the shared bring-up; a machine whose tree
    // names none has no line to route a keystroke through, so the step is skipped there.
    if memory::plic_region().is_some() {
        // The UART's PLIC source, from the machine's own tree: 10 on QEMU virt, 32 on the
        // JH7110. This step used to arm a QEMU constant, and on the board that enabled an
        // unrelated source, so a real keystroke could never reach the driver; boot 13 proved
        // it on silicon (notes/visionfive2.md, BUGS). The fallback when the tree does not say
        // is that same constant (user::UART_RX_INTID), and the line below names which source
        // won, so a bench transcript is diagnosable.
        let (uart_irq, uart_irq_source) = user::uart_irq_and_source();

        println!("  uart irq    : source {uart_irq} ({uart_irq_source})");

        let started = user::initial_ramdisk()
            .filter(|a| {
                nifefs::Fs::parse(a)
                    .map(|fs| fs.read("serial_driver").is_some())
                    .unwrap_or(false)
            })
            .and_then(|a| user::riscv_uart_driver_demo(a, uart_irq).ok());
        match started {
            Some(report) => {
                // A receiver for the driver's reports, so the boot tour does not block on input.
                sched::spawn(move || {
                    let byte = sched::ipc_receive(report)[0] as u8;
                    println!(
                        "  device IRQ  : an unprivileged userspace driver serviced the UART via its Irq cap and sent {byte:#04x} ({:?})",
                        byte as char,
                    );
                });
                println!(
                    "  device IRQ  : userspace driver started (holds an Irq cap + a UART mapping); pipe a byte to see it"
                );
                // Give a byte already piped a turn to flow through the driver before the banner.
                for _ in 0..8 {
                    sched::yield_now();
                }
            }
            None => println!(
                "  device IRQ  : skipped (no 'driver' in the initial ramdisk; run `cargo xtask initial-ramdisk-riscv`)"
            ),
        }
    }
}

/// riscv64 phase 5: virtio-mmio and PCIe block device discovery.
#[cfg(target_arch = "riscv64")]
fn riscv64_block_devices() {
    // virtio block device discovery (parity C). Probe the virtio-mmio slots the `virt` machine
    // lays out (0x1000_1000..); a block device shows up when a disk is attached (NIFE_DISK).
    // The kernel owns the transport and will hand a userspace driver the device's registers, an
    // Irq capability, and a DMA region; here we just prove the discovery works on RISC-V.
    match virtio::find_block_device() {
        Some(dev) => println!(
            "  virtio      : block device found at {:#x}, PLIC IRQ {} (kernel owns the transport)",
            dev.mmio_phys, dev.intid,
        ),
        None => {
            println!("  virtio      : no block device attached (pass NIFE_DISK to attach one)");
        }
    }

    // PCIe enumeration + virtio-pci bring-up (the PCIe transport, P1/P2). The disk QEMU
    // attaches as `virtio-blk-pci` arrives over the transport real hardware uses: found by
    // walking ECAM config space, its BARs placed by the kernel (OpenSBI does no PCI setup),
    // its register blocks resolved through the virtio vendor capabilities, its INTx line
    // swizzled to a PLIC input.
    match pci::find_block_device() {
        Some(d) => println!(
            "  pcie        : virtio-blk at {:02x}:{:02x}.{}, common {:#x}, notify {:#x} (mult {}), isr {:#x}, PLIC IRQ {}",
            d.bdf.bus,
            d.bdf.dev,
            d.bdf.func,
            d.common,
            d.notify_base,
            d.notify_mult,
            d.isr,
            d.intid,
        ),
        None => {
            println!("  pcie        : no virtio-blk on the bus (pass NIFE_DISK to attach one)");
        }
    }
}

// **The clock the `hw entropy` step is measured with**
// (design/roadmap/0306-time-the-hw-entropy-step.md, milestone 159's own follow-on).
//
// Read here, on the line after the `pcie` print, because the gap a bench session has been
// timing by eye is exactly `pcie` to `hw entropy`: those two lines are adjacent in the
// transcript, so the wall time between them is what a stopwatch at a serial console
// measures, and a person watching one resolves it to about a second. `design/fatal-risks/README.md`
// risk 6's third part is *at real speed*, and a bytes-per-second figure worth quoting needs
// the machine to time itself.
//
// **The span deliberately includes the `hw clock` step**, which prints between the two, so
// that the number printed below is the same quantity the stopwatch was measuring rather
// than a subset of it that happens to be tidier. The bring-up and draw costs are timed
// separately underneath it, and those are the two the rate question actually wants: a
// reseed plus a generation is a once-per-boot cost, and the round trips are the rate.
//
// `arch::timer::now()` is the mechanism the boot already uses (the preemption step above spins
// on it), so nothing new is introduced here; on riscv64 it is the `time` CSR, whose rate
// came out of this machine's device tree at `init_frequency`.
#[cfg(target_arch = "riscv64")]
fn riscv64_entropy() {
    let entropy_step_start = arch::timer::now();

    // **A real, non-virtio device, driven by a confined userspace process** (milestone 159,
    // design/roadmap/0159-jh7110-trng-driver.md; fatal risk 6 in design/fatal-risks/README.md). The
    // JH7110's TRNG is a register block on the `SoC`'s own fabric: no transport to negotiate,
    // no queue, no DMA. The kernel's whole part is the two lines below (ask the device tree
    // whether the device exists, then hand a userspace program one page of its registers and
    // two endpoints) and then playing a client over the same request endpoint any other client
    // would hold. The kernel never reads a `RAND` register.
    //
    // **The skip is the honest answer on every machine CI boots.** QEMU's riscv64 `virt` board
    // has no TRNG node under either spelling, so `jh7110_trng_device` returns `None` there and
    // this prints the skip rather than a claim. Only a `StarFive` VisionFive 2 (radon, the one
    // this project benches on) can print the other line, and the line only prints when bytes
    // actually arrived.
    //
    // **Both spellings are named in the skip because milestone 239 (radon's device tree does
    // not describe the TRNG, so a working driver never runs) was mis-diagnosed off the old
    // wording.** On 2026-09-03 this line said "describes no starfive,jh7110-trng" on a board
    // whose firmware tree does describe the device, under the vendor U-Boot's own
    // `starfive,trng`; the line was true and the conclusion drawn from it was not. A skip that
    // names everything it looked for cannot be read that way twice.
    // **The clock step announces itself even when there is nothing to do** (milestone 220).
    // On every machine this repository's CI boots there is no clock controller and no TRNG,
    // and a step that printed nothing there would leave a reader of the transcript to go and
    // check the source for whether it ran. The `Some` case is printed below, beside the
    // bring-up report, because the report is what makes the address worth reading.
    if user::entropy_service::jh7110_crg_window().is_none() {
        println!(
            "  hw clock    : skipped (this machine's tree names no JH7110 clock controller and no JH7110 TRNG, so no window was mapped and nothing was stored; QEMU virt has neither)"
        );
    }
    match user::entropy_service::jh7110_trng_device() {
        // **The skip, and a reference measurement beside it when this machine can give one.**
        //
        // The skip itself is the honest answer on every machine but radon and it has not
        // changed. What is new is the number after it, and the reason it is worth printing is
        // that the TRNG figure this step exists to produce is **not interpretable on its own**.
        // A draw through this service is one `entropy_contract` exchange per 8 bytes
        // (`MAX_BYTES`), so 32 bytes is four round trips through a userspace process, and a
        // measured microsecond count over 64 bytes mixes the device's cost with the IPC's
        // without saying in what proportion. The proposal names that as the thing to state
        // before the number leaves the machine: *does it count the IPC, the poll loop, the
        // process spawn?*
        //
        // The virtio-rng backend answers it, because it is **the same path with a different
        // device at the end**: the same `entropy_contract`, the same `Wiring::fill`, the same
        // eight round trips, the same confined userspace process holding the same two
        // rendezvous capabilities. Whatever it costs here is what the JH7110's number would
        // cost with a free device, so the difference between the two is the driver's.
        //
        // **It is a reference, not a measurement of hardware**, and the line says so in those
        // words: QEMU backs virtio-rng from the host's `/dev/urandom` and its cost is the
        // emulator's, not silicon's. `notes/benchmarks.md`'s standard is to say where a
        // comparison is not apples-to-apples, and this is where.
        //
        // Only when this machine actually has such a device, which on the default riscv64
        // boot it does not: `NIFE_RNG` is what attaches one (DECISIONS §120's QEMU-only
        // stopgap), and without it `ensure` returns `None` after the bus scan and the line
        // reads exactly as it did before plus its own microsecond count.
        None => riscv64_entropy_reference(entropy_step_start),
        Some(device) => riscv64_jh7110_entropy(device, entropy_step_start),
    }
}

/// riscv64 phase 6, on a machine with no TRNG: the virtio-rng reference draw, when there is one.
#[cfg(target_arch = "riscv64")]
fn riscv64_entropy_reference(entropy_step_start: u64) {
    let reference = user::program("entropy").and_then(|image| {
        let w = user::entropy_service::ensure(image, user::entropy_service::Bus::Mmio)?;
        let report = w.wait_for_ready().unwrap_or([0; 5]);
        let ready_at = arch::timer::now();
        let (mut a, mut b) = ([0u8; 32], [0u8; 32]);
        let (na, nb) = (w.fill(&mut a), w.fill(&mut b));
        let drawn_at = arch::timer::now();
        Some((report[0], na + nb, ready_at, drawn_at))
    });
    match reference {
        Some((verdict, bytes, ready_at, drawn_at))
            if verdict == entropy_contract::READY && bytes > 0 =>
        {
            println!(
                "  hw entropy  : skipped (this machine's tree names no TRNG: neither starfive,jh7110-trng nor the vendor U-Boot's starfive,trng; QEMU virt has neither); since the pcie line {} us. Reference, NOT a TRNG and NOT hardware: this emulator's virtio-rng over the same entropy_contract path, bring-up {} us, {} bytes in {} us ({} bytes/s over {} round trips)",
                micros_between(entropy_step_start, arch::timer::now()),
                micros_between(entropy_step_start, ready_at),
                bytes,
                micros_between(ready_at, drawn_at),
                bytes_per_second(bytes as u64, drawn_at.wrapping_sub(ready_at)),
                bytes as u64 / entropy_contract::MAX_BYTES,
            );
        }
        // A device was there and the service did not come up, or came up dry. Said
        // rather than swallowed, because a reference that silently degrades to the
        // plain skip would make a machine with a broken virtio-rng look like one with
        // no virtio-rng at all.
        Some((verdict, bytes, _, _)) => println!(
            "  hw entropy  : skipped (this machine's tree names no TRNG: neither starfive,jh7110-trng nor the vendor U-Boot's starfive,trng; QEMU virt has neither); since the pcie line {} us. The virtio-rng reference did not run: report {:#x}, {} bytes drawn",
            micros_between(entropy_step_start, arch::timer::now()),
            verdict,
            bytes,
        ),
        None => println!(
            "  hw entropy  : skipped (this machine's tree names no TRNG: neither starfive,jh7110-trng nor the vendor U-Boot's starfive,trng; QEMU virt has neither); since the pcie line {} us, which is the device-tree query and nothing else (no virtio-rng on this machine either, so there is no reference draw to time; pass NIFE_RNG to attach one)",
            micros_between(entropy_step_start, arch::timer::now()),
        ),
    }
}

/// riscv64 phase 6, on a JH7110: wire the TRNG driver, draw twice, and time it.
#[cfg(target_arch = "riscv64")]
fn riscv64_jh7110_entropy(device: jh7110_entropy::Discovered, entropy_step_start: u64) {
    match user::program("jh7110_entropy") {
        None => println!(
            "  hw entropy  : JH7110 TRNG at {:#x}, but no 'jh7110_entropy' in the initial ramdisk (run `cargo xtask initial-ramdisk-riscv`)",
            device.reg_base,
        ),
        Some(image) => {
            // **The wiring is timed, and the console output between the segments is not**
            // (the proposal above). `ensure` maps the register window and spawns the
            // driver; the readiness wait is the driver's own bring-up (a reseed, then a
            // first generation). The `hw clock` line prints between the two, and a
            // `println!` here is a polled UART: on radon at 115200 baud that line is
            // roughly 30 ms of the kernel doing nothing but shift bits out, which is the
            // same order as the bring-up it would otherwise be added to. So the bring-up
            // figure is the sum of two measured segments rather than one span across
            // them, and the whole-gap figure beside it (`since the pcie line`) is the one
            // that still includes the console, because that is the quantity a stopwatch
            // at the serial port was measuring.
            let wire_start = arch::timer::now();
            match user::entropy_service::ensure(image, user::entropy_service::Bus::Jh7110) {
                None => println!(
                    "  hw entropy  : JH7110 TRNG at {:#x}, but the service would not wire",
                    device.reg_base,
                ),
                Some(w) => {
                    let wired_at = arch::timer::now();
                    riscv64_clock_report(w.clock);
                    // The driver's bring-up report next:
                    // `[READY, first_refill_ok, bytes_in_hand]`, or a 0xDEAD_.. word whose
                    // low byte names the step. A device that never answered says so here,
                    // and since 2026-09-04 so does one that answered with zeros:
                    // `entropy_contract::readiness` decides that word from the bytes, which
                    // is what the boot below found it was not doing.
                    let wait_start = arch::timer::now();
                    let report = w.wait_for_ready().unwrap_or([0; 5]);
                    let ready_at = arch::timer::now();
                    // Then two draws through the request endpoint, as a client. Two,
                    // because one proves only that *something* was returned: a stuck
                    // register file, a driver serving its buffer twice, and a device that
                    // never started all present as a repeat.
                    //
                    // **Each draw is four round trips, not one**, and that is the fix for
                    // the failure radon printed on 2026-09-04. `Wiring::get` is a single
                    // `entropy_contract` exchange, and that protocol carries `MAX_BYTES = 8`,
                    // so `get(32, ..)` returns 8 and can never return 32: the success line
                    // below was unreachable on any device, working or not. It went
                    // unnoticed for three days because this branch runs on exactly one
                    // machine in the world and QEMU takes the `skipped` arm.
                    //
                    // Filling all 32 bytes is also the stronger test, which is why the
                    // count stays 32 rather than dropping to 8. The driver's pool holds
                    // one 32-byte generation, so draw `a` empties it and draw `b` forces a
                    // second trip to the hardware. Two 8-byte draws would both have come
                    // out of the same buffer, and `a != b` would then prove only that the
                    // cursor advanced. Now it compares two device generations, which is
                    // what the line below claims.
                    let (mut a, mut b) = ([0u8; 32], [0u8; 32]);
                    let (na, nb) = (w.fill(&mut a), w.fill(&mut b));
                    let drawn_at = arch::timer::now();
                    // The two segments the console does not sit inside: the wiring (window
                    // mapped, driver spawned) and the readiness wait (the driver's reseed
                    // and first generation).
                    let bringup_ticks =
                        wired_at.wrapping_sub(wire_start) + ready_at.wrapping_sub(wait_start);
                    let draw_ticks = drawn_at.wrapping_sub(ready_at);
                    // The service refuses to report ready on an all-zero first bufferful
                    // now, so this is a client checking a claim rather than the only thing
                    // standing between a boot and zeros served as randomness. It stays
                    // because a tour that only repeated the service's own verdict would
                    // have caught nothing on 2026-09-04.
                    let zeros = a.iter().all(|&x| x == 0);
                    if report[0] == entropy_contract::READY
                        && na == 32
                        && nb == 32
                        && !zeros
                        && a != b
                    {
                        println!(
                            "  hw entropy  : JH7110 TRNG at {:#x} served 32+32 bytes to a client through a capability that names no device; first draw {:02x}{:02x}{:02x}{:02x}.., second differs; STAT after init {:#010x} ({}); since the pcie line {} us, of which bring-up {} us (window mapped, driver spawned, reseed, first generation; the hw clock line's own console time excluded) and {} bytes in {} us ({} bytes/s over {} round trips)",
                            device.reg_base,
                            a[0],
                            a[1],
                            a[2],
                            a[3],
                            (report[1] >> 32) as u32,
                            mode_note((report[1] >> 32) as u32),
                            micros_between(entropy_step_start, arch::timer::now()),
                            micros(bringup_ticks),
                            na + nb,
                            micros(draw_ticks),
                            bytes_per_second((na + nb) as u64, draw_ticks),
                            (na + nb) as u64 / entropy_contract::MAX_BYTES,
                        );
                    } else {
                        // `report[2]` is the driver's bring-up diagnostic and it is the
                        // number a bench session reads first. It is **always** the raw
                        // `(STAT << 32) | ISTAT` now, on success as well as on failure:
                        // it used to be the byte count when the report word said READY,
                        // and on 2026-09-04 that cost an hour, because the `0x20` printed
                        // here was read as an undocumented `ISTAT` bit 5 when it was the
                        // number 32 wearing a register's clothes. All zeros means the
                        // register window read as nothing at all (a gated clock, an
                        // undeasserted reset, or a base that is not the TRNG) rather than
                        // a device that answered wrongly. See components/src/jh7110_entropy.rs.
                        // The tree's own two words about this node come with the failure,
                        // not in a separate line, because they are what a bench session
                        // reads next: an all-zero diagnostic on a node the firmware calls
                        // `disabled` is a clock or reset the firmware had no reason to
                        // ungate (milestone 220's territory), while the same zeros on a
                        // node it calls `okay` are a different problem entirely.
                        println!(
                            "  hw entropy  : FAILED: JH7110 TRNG at {:#x} (tree says {}, status {}): report {:#x}, bring-up diagnostic {:#018x}, STAT after init {:#010x} ({}), draws {na}/{nb} bytes, first-all-zero {zeros}, draws-differ {}; since the pcie line {} us, of which bring-up {} us and draws {} us",
                            device.reg_base,
                            core::str::from_utf8(device.compatible).unwrap_or("?"),
                            if device.status_okay {
                                "okay"
                            } else {
                                "disabled"
                            },
                            report[0],
                            report[2],
                            (report[1] >> 32) as u32,
                            mode_note((report[1] >> 32) as u32),
                            a != b,
                            micros_between(entropy_step_start, arch::timer::now()),
                            micros(bringup_ticks),
                            micros(draw_ticks),
                        );
                    }
                }
            }
        }
    }
}

// **The clock and reset controller's own answer, first** (milestone
// 220), because it decides how the line below should be read. On
// 2026-09-04 radon printed an all-zero TRNG register file, and the
// two candidate explanations (a block nobody had powered, or a
// driver talking to the wrong address) are told apart here and
// nowhere else. The `before` words are what separate them again:
// clocks that were already enabled mean this milestone's premise was
// wrong and the zeros have some other cause.
//
// Printed even when it says nothing happened, because "no controller
// in this tree" is itself the finding on any machine but radon, and a
// silent step is one a bench reader has to go and check the source
// for.
#[cfg(target_arch = "riscv64")]
fn riscv64_clock_report(clock: Option<jh7110_clock_and_reset::Report>) {
    match clock {
        Some(report) => {
            let found = user::entropy_service::jh7110_crg_window();
            println!(
                "  hw clock    : JH7110 STG CRG at {:#x} ({}): clocks {:#010x},{:#010x} -> {:#010x},{:#010x} ({}); reset {} assert {:#010x} -> {:#010x}, status {:#010x} ({}, {} polls){}",
                found.map_or(0, |f| f.base),
                match found {
                    Some(f) if f.from_tree => "named by this machine's device tree",
                    Some(_) =>
                        "NOT named by this machine's tree: the constant both published trees agree on",
                    None => "address unknown",
                },
                report.clock_before[0],
                report.clock_before[1],
                report.clock_after[0],
                report.clock_after[1],
                if report.has_clocks_running() {
                    "running"
                } else {
                    "NOT running: the enable bit did not read back, so nothing is behind this window"
                },
                jh7110_clock_and_reset::STGRST_SEC_AHB,
                report.reset_assert_before,
                report.reset_assert_after,
                report.reset_status_after,
                if report.released {
                    "released"
                } else {
                    "STILL HELD"
                },
                report.polls,
                if report.was_already_up() {
                    "; the firmware had already done all of this, so a gated clock is NOT why the TRNG reads zeros"
                } else {
                    ""
                },
            );
        }
        // Unreachable in practice, and printed rather than ignored for
        // that reason: this arm is inside `Some(device)`, so the tree
        // named a TRNG, and `memory::init` records a window for exactly
        // that case. If it ever fires, the guard and the caller have
        // drifted apart and a bench transcript should say so.
        None => println!(
            "  hw clock    : no JH7110 clock-and-reset window was mapped even though this tree names a TRNG, so nothing was ungated"
        ),
    }
}

/// **riscv64's milestone tour**: what is left of it until each of its device steps is a
/// self-test check (calef's ruling of 2026-10-19 (UTC) on #2434). Called where aarch64's is.
///
/// Its preemption, user-program and address-space steps are `self_test::run`'s `preemption` and
/// `userspace` checks, and its last line and boot stage are `kernel_main`'s, after the verdict.
/// What remains needs a device the machine may not have: the UART's interrupt, a virtio disk, and
/// the JH7110's TRNG and clock controller.
// The `shell`, `bench` and test builds never call it. Allowed rather than `cfg`-ed out, so the tour
// and the riscv64-only demonstrations beneath it still compile, and are linted, in every
// configuration: code that only type-checks in the configurations that use it rots in the others.
#[cfg_attr(
    any(test, feature = "system_tests", feature = "bench", feature = "shell"),
    allow(dead_code)
)]
pub(super) fn milestone_tour() {
    riscv64_device_interrupt();
    riscv64_block_devices();
    riscv64_entropy();
}

/// **Counter ticks as microseconds** (design/roadmap/0306-time-the-hw-entropy-step.md), for
/// the boot tour's `hw entropy` line.
///
/// Microseconds rather than milliseconds because the interesting half of the number is an IPC round
/// trip, and rather than raw ticks because a tick is a different amount of time on every machine
/// this runs on: QEMU's riscv64 `virt` counts at 10 MHz and the JH7110 at 4 MHz, so a transcript
/// quoting ticks could not be compared against another board's without the rate beside it. The rate
/// itself came out of this machine's own device tree (`arch::timer::init_frequency`), which is why
/// there is no constant here to go stale.
///
/// Truncating division, deliberately: this is an instrument, and a figure that rounded up would
/// report a nonzero duration for work that took no measurable time at all.
///
/// Provisional name (an architect's call): `micros`, with `micros_between` and `bytes_per_second`
/// beside it.
#[cfg(target_arch = "riscv64")]
#[allow(dead_code)]
fn micros(ticks: u64) -> u64 {
    // `frequency()` asserts the rate is nonzero rather than returning a poison value, and it has
    // been read from the tree since `bring_up_machine`, long before this tour, so there is no
    // pre-init case to guard here. `saturating_mul` bounds the scaling rather than the duration: a
    // gap long enough to overflow is one nobody is timing.
    ticks.saturating_mul(1_000_000) / arch::timer::frequency()
}

/// The two-timestamp form of [`micros`]. `wrapping_sub` because the `time` CSR is a free-running
/// 64-bit counter and this is the arithmetic the rest of this file already uses on it (see the
/// JH7110 entropy step above); at 10 MHz it wraps after about 58,000 years.
#[cfg(target_arch = "riscv64")]
#[allow(dead_code)]
fn micros_between(start: u64, end: u64) -> u64 {
    micros(end.wrapping_sub(start))
}

/// **Bytes per second over a measured span**, for the same line.
///
/// Computed from ticks rather than from the microsecond figure beside it, so the rate does not
/// inherit that figure's truncation. Zero for a span too short to measure, which is the honest
/// answer: a rate derived from a zero-length interval is not a large number, it is no number.
///
/// **What it counts, which has to be stated before it is quoted.** Everything between the readiness
/// report and the last byte landing: the `entropy_contract` round trips (one per
/// `entropy_contract::MAX_BYTES`, so four per 32-byte draw), the two context switches each one costs,
/// the driver's own poll loop, and the device. It does **not** count the process spawn or the
/// bring-up, which are the once-per-boot cost reported separately. It is therefore not comparable
/// to a Linux `hwrng` throughput figure, which is a read from an already-running kernel driver with
/// no IPC in it at all; `notes/benchmarks.md`'s rule is to say so where the number is printed.
#[cfg(target_arch = "riscv64")]
#[allow(dead_code)]
fn bytes_per_second(bytes: u64, ticks: u64) -> u64 {
    if ticks == 0 {
        return 0;
    }
    bytes.saturating_mul(arch::timer::frequency()) / ticks
}

/// **Say what the TRNG's `STAT` says about its output width** (milestone 159), for the boot tour's
/// `hw entropy` line.
///
/// This is the one bit that decides whether the 32 bytes that line reports are 32 bytes of device
/// output. In 128-bit mode only `RAND0..RAND3` carry a generation's answer, so a driver that
/// assembles all eight words is serving 16 real bytes and 16 of whatever the upper registers hold.
/// The driver writes `MODE.R256` during bring-up precisely because the width a given JH7110 resets
/// to is a build-time parameter of the silicon; `STAT.R256` is the read-back that says it took, and
/// nothing has ever read it on hardware.
///
/// An all-zero `STAT` is called out separately because it is not a mode report at all: it is the
/// signature of a register window that answered with nothing, which milestone 220's clock line
/// above is what explains.
// riscv64-only because the JH7110 is; `allow(dead_code)` because the boot tour that calls it is
// itself compiled out of the shell and bench builds, the same way `image_for_virtio` is.
#[cfg(target_arch = "riscv64")]
#[allow(dead_code)]
fn mode_note(stat: u32) -> &'static str {
    if stat == 0 {
        "the whole status register read zero, so this says nothing about the mode"
    } else if stat & jh7110_entropy::STAT_R256 != 0 {
        "256-bit: all eight RAND words are the answer"
    } else {
        "128-BIT: only RAND0..3 are the answer, so 16 of every 32 bytes are not device output"
    }
}
