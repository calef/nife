//! The boot sequence: `boot`, which builds the interactive system in numbered steps and then
//! becomes its spawn service, and the password helpers only it calls. Moved out of `lib.rs`
//! unchanged but for visibility by milestone 843 (the system-initializer crate root is split along
//! its seams), under §279 (the system-initializer crate root splits into private modules). `boot`
//! is still one body; decomposing it along its steps is that milestone's named follow-on.
//!
//! Name: provisional (milestone 843's lane, 2026-10-11 UTC).

use super::*;

/// Draw `out.len()` bytes from the entropy service through `request`
/// ([`entropy_protocol::MAX_BYTES`] at a time), the identical loop `components/src/credentialer.rs`'s own
/// `fill` performs as a *client* of that same service (this process is, briefly, one too: it draws
/// the generated password's own raw bytes before `credentialer` ever exists). `false` when the
/// service could not supply them, which the caller treats as fatal to generating a password at all
/// -- the same "never invent a weak one" posture `credentialer.rs`'s own `fill` documents for a
/// salt.
fn fill_entropy(request: u64, out: &mut [u8]) -> bool {
    let mut done = 0;
    while done < out.len() {
        let want = (out.len() - done).min(entropy_protocol::MAX_BYTES as usize);
        // SAFETY: `invoke` traps to the kernel, which validates the capability and the method
        // before acting.
        let (r0, r1) = call(
            request,
            entropy_protocol::req(entropy_protocol::GET, want as u64),
            0,
        );
        let Some(n) = entropy_protocol::delivered(r0) else {
            return false;
        };
        if n < want {
            return false;
        }
        done += entropy_protocol::take(n, r1, &mut out[done..]);
    }
    true
}

/// Hex-encode `bytes` into `out` (`out.len() == 2 * bytes.len()`), lowercase: the shape a cloud
/// image's generated first-boot password already takes, printable and typeable at a keyboard
/// without a decoder.
fn hex_password(bytes: &[u8], out: &mut [u8]) {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    for (i, &b) in bytes.iter().enumerate() {
        out[i * 2] = DIGITS[(b >> 4) as usize];
        out[i * 2 + 1] = DIGITS[(b & 0xf) as usize];
    }
}

/// **Build the interactive system and become its spawn service.** Never returns: the last thing it
/// does is park in `RECEIVE` on the shell's spawn channel for the life of the boot.
///
/// `initrd_len` is the archive length the kernel passed at entry; `fs_rights` is the `filesystem_protocol::dir`
/// rights the file-service endpoint carries, and 0 means this boot attached no disk. `second_dir`
/// is milestone 154's addition: `Some` hands the shell a second, disjoint directory capability
/// (see [`SecondDirGrant`] for what this does and does not decide); every real entry point passes
/// `None` today.
pub fn boot(
    g: &BootEndowment,
    initrd_len: u64,
    fs_rights: u64,
    second_dir: Option<SecondDirGrant>,
) -> ! {
    // The archive the kernel mapped read-only; its length arrived at entry.
    // SAFETY: the kernel mapped `initrd_len` bytes of reserved RAM, read-only, at INITRD_VA. It is
    // reserved memory that outlives every process, so the borrow is honest for the whole boot.
    let archive =
        unsafe { core::slice::from_raw_parts(INITRD_VA as *const u8, initrd_len as usize) };
    let Ok(fs) = nifefs::Fs::parse(archive) else {
        fail()
    };

    // **Give every scratch page back as soon as it is in the child** (milestone 95 (an unmap
    // primitive, and the mappings init never lets go), §249 (a running address space stays
    // nameable)). First, before anything is built, so no boot server's page is ever left mapped
    // here. A kernel that granted no slot 28 leaves the loader as it was: each page mapped until the
    // child's region is destroyed, which for a boot server is never.
    if is_granted(g.own_space) {
        supervision_protocol::give_up_scratch_through(g.own_space);
    }

    // **The table the progenitor measures what it loads against** (milestone 104). The kernel vouched for this
    // entry before it started us, exactly as it vouched for our own bytes
    // (`kernel::trust::require_program_measurements`), so the table is worth what this process is
    // worth and the chain extends by induction rather than by widening the kernel.
    //
    // Bytes that are not UTF-8 become the **empty** table rather than a fault, and an empty table
    // vouches for nothing: every lookup below answers `Unmeasured`, the console is refused, and the
    // boot stops. That is the same direction to be wrong in as the kernel's empty trust root, and it
    // is the failure mode a measured boot must never get backwards.
    let table = fs
        .read(measured_boot::PROGRAM_MEASUREMENTS)
        .and_then(|b| core::str::from_utf8(b).ok())
        .unwrap_or("");

    // **The image's package catalog** (milestone 198 (a package manager) rung 3a's installer):
    // what `package install` checks a package's digest against. Measured like a program, because
    // it is an archive entry above the table, and read as the **empty** catalog when the table
    // refuses it or the archive has none, which vouches for no package: the same direction to be
    // wrong in as the table's own rule above.
    let catalogue = fs
        .read(package_archive::CATALOG)
        .filter(|bytes| {
            measured_boot::verify_in_manifest(table, package_archive::CATALOG, bytes).is_ok()
        })
        .and_then(|b| core::str::from_utf8(b).ok())
        .unwrap_or("");

    let con_elf = measured(&fs, table, "console");
    // **The system log service** (milestone 342): optional, like the sink adapter below. A boot
    // without it, or whose kernel granted no ring, keeps the kernel printing for itself.
    let log_elf = measured(&fs, table, "system_log").elf;
    let in_elf = measured(&fs, table, "input");
    let td_elf = measured(&fs, table, "line_editor");
    // **The terminal's supervisor** (milestone 23 (a capability-routed component OS with live
    // replacement), calef's ruling of 2026-09-27): what builds `line_editor` now, so that something
    // on a running system holds what a replacement would need. It is handed `line_editor`'s bytes
    // rather than the archive, which this process cannot delegate, the same way `login` is handed
    // the caretaker's.
    let ts_elf = measured(&fs, table, "terminal_supervisor");
    let sh_elf = measured(&fs, table, "swish");
    // **The terminal's sink adapter** (milestone 50's last remainder). Optional on purpose: an
    // initrd built without it still boots, and a program that declares a second stream then finds
    // an empty slot and says what it has to say in-band. A missing component should cost a feature,
    // not a prompt. An adapter the table refuses costs exactly the same feature, which is the whole
    // policy in one line: the progenitor treats what it cannot vouch for as what is not there.
    let sink_elf = measured(&fs, table, "terminal_sink_caretaker").elf;
    // **The entropy service** (DECISIONS §120's 2026-08-26 amendment), optional in exactly the
    // adapter's own sense: a boot with no `entropy` program in its initrd, or a table that refuses
    // it, simply never builds the service, and [`g.virtio_rng`](BootEndowment::virtio_rng)'s own
    // probe finds nothing to build it *from* either way on most boots (real hardware, or a run
    // with `NIFE_RNG` unset). Neither case is a broken boot, so neither belongs with the required
    // three below.
    let ent_elf = measured(&fs, table, "entropy").elf;
    // **The network stack** (milestone 590 (provisional)), optional in the entropy service's sense
    // and for its reasons: a boot with no NIC, or an archive whose table refuses `net_stack`, comes
    // up without a network rather than without a prompt.
    let net_elf = measured(&fs, table, "net_stack").elf;
    // **The graphical terminal stack** (milestone 600 (provisional)), optional in the same sense:
    // an archive without them, or a table that refuses one, boots on the serial console. Measured
    // like every program this process builds, which the kernel never did for these three while it
    // built them (`kernel::user::program` reads the archive and checks nothing).
    //
    // **Not here, though: in [`graphical_verdict`] and the two builders**, and that is a stack
    // budget rather than tidiness. This frame is live for the whole boot, because the spawn service
    // runs inside it, and the progenitor's stack is eight pages (`kernel::user::INIT_STACK_PAGES`).
    // The first version kept three more `elf::Elf`s here and inlined the builders into it, and
    // overflowed that stack by 24 bytes under `package install` (`swish-check`, 2026-09-26). Each
    // helper measures for itself, in a frame that is gone before the spawn service starts; hashing
    // three small programs twice costs less than the stack does.

    // **Milestone 49's login stack** (`credentialer`, `identity_provisioner`, `login`,
    // `login_audit_receiver`): optional in exactly the entropy service's own sense, and gated on
    // it too -- there is no salt, no password and no credential store without real entropy, so a
    // boot with no entropy service has no login path either, the same way it has no login path
    // with no filesystem (`have_login_stack`, below, checks both). Read here with the rest of this
    // pass, not built yet: building happens later, after the shell's own construction has
    // returned this table to its resting count (see that block's own comment for why the timing
    // matters).
    let cred_elf = measured(&fs, table, "credentialer").elf;
    let idp_elf = measured(&fs, table, "identity_provisioner").elf;
    let login_elf = measured(&fs, table, "login").elf;
    let audit_elf = measured(&fs, table, "login_audit_receiver").elf;
    // The undertaker (milestone 22, the interactive increment). Read here with the rest, because
    // the archive is only readable while we hold it and every failure below is one `fail`. Required
    // rather than optional, unlike the adapter above: without it a bounded job pool fills and the
    // prompt stops spawning, which is a broken system and not a missing feature.
    let reaper_elf = measured(&fs, table, "job_undertaker");
    // **The subtree caretaker** (milestone 31 phase 3), which is not a boot component and not a
    // spawnable program: it is what a *directory grant* is made of, one per invocation, so it is
    // read here with the rest and built by the spawn service far below.
    //
    // Optional in exactly `terminal_sink_caretaker`'s sense, and the missing-component rule decides
    // it rather than a second policy: without one, a directory grant cannot be delivered and the
    // prompt says so, which costs `rm` and nothing else. A refusal by the measurement table costs
    // the same, because the progenitor treats what it cannot vouch for as what is not there.
    let care_elf = measured(&fs, table, "fs_subtree_caretaker").elf;
    // **The nameset caretaker** (milestone 205 (how a foreign program is told what to do)'s
    // designation half): what a program that hears words is granted when its words named some
    // entries of the shell's directory and not the directory itself. A boot without it refuses such
    // a grant rather than widening it to the subtree one.
    let set_elf = measured(&fs, table, "fs_nameset_caretaker").elf;
    // **Whether the file server enforces subtree grants itself** (milestone 606 (a directory walk
    // costs what it does on Linux), rulings D and T1
    // of 2026-09-27): read from the note its own measured bytes carry, never assumed. `false` for
    // bytes the table would not vouch for, for a server with no such note, and for a note that is
    // there and unreadable, so every doubt ends in a caretaker, which is the default.
    let fs_scoped = subtree_grants(measured(&fs, table, "redoxfs_server").elf.as_ref());
    // **And the same bytes again, unparsed, because `login` needs them too** (milestone 233).
    //
    // That program builds one caretaker per authenticated session, so it needs an image to build
    // from. It used to read the archive itself, which worked only under the kernel's own test
    // harness: `build_child` can map a page only if this process holds a `PageFrame` capability for
    // it, and the archive is reserved RAM the frame allocator does not own and no capability names.
    // So the real boot started `login` with no archive at all and it died at `_start` on every
    // boot. It is handed a copy of exactly the one program it needs instead, as a blob (see the
    // login block far below, and `login_protocol::CARETAKER_ELF_VA` for the whole account).
    //
    // Empty when this process would not vouch for the bytes, which is the same answer `care_elf`
    // above already gives the spawn service: `login` then comes up and denies every login rather
    // than refusing to start. Held across the shadowing of `fs` further down, which is why it is
    // read here with the rest rather than at the point of use.
    let care_blob: &[u8] = if care_elf.is_some() {
        fs.read("fs_subtree_caretaker").unwrap_or(&[])
    } else {
        &[]
    };
    // **And what `login` builds a durable session from** (milestone 152 (durable delegation)):
    // `user_timetable_keeper` and `timetable`, as blobs for the caretaker's reason, and empty unless this process
    // vouches for both, so `login` then answers `SCHEDULE` as a plain login. No program a job runs
    // travels with them: a scheduled job runs what the live activation generation names (Fork 8
    // ruled D by calef on 2026-09-27, on #1377).
    let (keeper_blob, timetable_blob): (&[u8], &[u8]) = match (
        measured(&fs, table, "user_timetable_keeper").elf.is_some(),
        measured(&fs, table, "timetable").elf.is_some(),
        fs.read("user_timetable_keeper"),
        fs.read("timetable"),
    ) {
        (true, true, Some(session), Some(timetable)) => (session, timetable),
        _ => (&[], &[]),
    };

    // **The programs the shell can spawn** (milestone 31), measured and parsed here rather than
    // after the giveaway: the announcement further down is the only thing the progenitor ever says, so the
    // verdicts have to exist before it. One `Option<elf::Elf>` is five words, so moving the whole
    // table up the frame costs a few hundred bytes and buys a person being told at boot instead of
    // at the prompt. (It said "seven of them" while `PROG_COUNT` was nine and then ten; a count
    // written beside a constant that moves is a count that goes stale, so this one is not written.)
    //
    // The refusals are collected in the same pass, because a second pass would hash every program a
    // second time. Only entries the archive **has** count as refusals: `rm` is deliberately not
    // loadable here, and a program this initrd never packed was never spawnable, so neither is news.
    let mut progs: [Option<elf::Elf>; grant_plan::PROG_COUNT] = core::array::from_fn(|_| None);
    let mut refused = [""; grant_plan::PROG_COUNT];
    let mut refused_n = 0usize;
    for (id, slot) in progs.iter_mut().enumerate() {
        let Some(name) = Prog::from_id(id as u64).and_then(archive_name) else {
            continue;
        };
        let found = measured(&fs, table, name);
        if found.unvouched {
            refused[refused_n] = name;
            refused_n += 1;
        }
        *slot = found.elf;
    }

    let ut = g.untyped;

    // **Freed here rather than after the console, line discipline and input are all built**
    // (milestone 47, DECISIONS §111's config-page lane): `for_test_roles` is dead weight on this
    // boot from the instant `g` exists (it names capabilities milestone 19d's test roles use and
    // the interactive system never touches), and this capability table has only sixteen slots, a
    // margin this file's own comment two screens down already documents as having broken once
    // (milestone 50's two extra kernel grants left "no slot left" for the shell's `build_child`).
    // Adding a fourth permanently-held kernel grant (the config page, alongside the clock page and
    // the filesystem pair) reproduced exactly that failure during the console/line-discipline/input
    // build, silently, because nothing between here and the old deletion point ever reads a
    // `for_test_roles` capability's contents: only its *slot* matters, and freeing the slot two
    // component-builds earlier costs nothing this boot was using.
    for &c in g.for_test_roles {
        cap_delete(c);
    }

    // **The display devices, when the kernel granted a virtio-gpu and the archive carries the
    // programs a `graphical_terminal` session is built from** (milestone 632 (provisional), calef's 2026-09-30
    // ruling: *"I don't want graphics at boot and won't for a long time"*. The boot below this
    // comment is the same minimal UART system on every boot; a gpu on the bus changes only *who
    // holds the grants*, and the answer is the shell, which holds them until a person at the
    // prompt delegates them into a `graphical_terminal` session). The gpu's four grants are probed the way the
    // virtio-rng trio is (`is_granted`, since there is no fourth `START` word to be told with) and
    // come together or not at all; a grant this process can neither build a session from nor hand
    // to the shell is released at once rather than carried, the posture every kernel grant here
    // already takes.
    //
    // **The keyboard is optional inside a session-bearing boot**, and its absence is milestone
    // 192's option A rather than a failure: a `graphical_terminal` session then takes its keystrokes from the
    // boot's own line discipline over the UART, at launch rather than at boot.
    //
    // **The UART is never freed early.** There is no graphical boot to leave the console with
    // nothing to drive, so `uart_dev`/`uart_irq` go back exactly where they always did, after the
    // input driver is built. Seven grants is the most any boot hands the shell here, and they are
    // placed below (at the shell's build, before the login block's peak) rather than carried past
    // it.
    let graphical_terminal_caps = graphical_terminal_grants(&fs, table, g);
    // **A terminal on the firmware's screen beside the serial console** (the shell on the firmware
    // screen, milestone 198's rung 1b): the kernel grants `disp_term_ep`/`disp_term_page` alone
    // when it put a terminal on the firmware's framebuffer (`kernel::user::boot_screen_terminal`),
    // and the UART stays the console and the keystroke source exactly as on every boot. The gpu's
    // grants and this pair never coexist (the kernel wires one or the other), so nothing needs to
    // say which won.
    let has_screen = is_granted(g.disp_term_ep);
    // **Read before anything below retypes**, and the position is the mechanism: every object this
    // function makes lands in the first free slot, so once one has, "slot 16 holds something" no
    // longer means the kernel put it there. See [`BootEndowment::entropy_ep`].
    let kernel_built_entropy = is_granted(g.entropy_ep);

    // **The entropy service** (DECISIONS §120's 2026-08-26 amendment), when the kernel granted a
    // virtio-rng device and the archive carries a program for it. Built **here, before anything
    // else**, and that positioning is load-bearing rather than a style choice.
    //
    // An earlier version of this built entropy after the console and line discipline, in the gap
    // their own three capabilities free ("The console's three capabilities go back now"). That
    // reads as the more generous gap, but it counts the wrong thing: the virtio-rng trio is
    // granted by the *kernel*, at spawn, so it inflates the resting baseline for the **whole**
    // function, not just the block that builds it. The earliest peak this function ever reaches
    // (retyping `request`/`reply`/`term_ep`/`con_shared`/`term_out`/`term_in`, all six before
    // console is even built) is already tight against the sixteen-slot wall on its own account
    // (`crates/system_initializer`'s own BUGS: "one slot from the wall" describes the *shell's*
    // peak, and the terminal-plumbing peak just named is close behind it); adding three more
    // permanent slots to a baseline that peak already stresses pushed it over, and the boot
    // faulted building `term_in`, in total silence, with no route to a person (this file's own
    // BUGS on why a refusal this early has none). Found by bisection, not reasoning: an isolated
    // test granted the progenitor one single harmless extra capability, at an arbitrary slot, with no code
    // anywhere naming or using it, and the identical silent fault reproduced.
    //
    // Building here instead avoids the collision rather than widening anything: this is the
    // *only* point in the function where the virtio-rng trio's three slots and the terminal
    // plumbing's six are never both live at once. Entropy is built and its three slots are
    // released before `request` is ever retyped, so every peak downstream of this block is
    // exactly what it was before this amendment existed.
    //
    // **Absence is not a failure, twice over**: no device (real hardware, or `NIFE_RNG` unset) and
    // no `entropy` program in the archive both leave the system exactly as it was before this
    // amendment, the same "a missing component costs a feature, not a boot" posture the sink
    // adapter and the subtree caretaker already take.
    //
    // **`entropy_ready`, not an inline `announce`.** This process has no terminal yet at this
    // point in the boot (that is the whole reason the peak accounting above works out), so the
    // outcome is carried forward as data and said later, at the existing, already-mapped print
    // site right before `term_ep`/`term_out` are dropped (see that site's own comment).
    //
    // # BUGS (historical: resolved below, this update)
    //
    // **Used to prove only the device chain.** This block builds the service and confirms it drew
    // real bytes from the real device; `entropy_client`, below, is what changed: `request`'s
    // the progenitor-side copy is now kept (not `cap_delete`d) exactly when a client will need it, so
    // `credentialer` can be handed a working view of it further down.
    let mut entropy_ready = false;
    // **`request`'s the progenitor-side copy, kept for the life of the boot** (milestone 49's boot-wiring
    // update, widened by milestone 111). `None` on every path that `cap_delete`s it (device
    // absent, mapping failed, the handshake did not answer `READY`); `Some(request)` exactly once
    // entropy is proven up.
    //
    // **It used to be dropped once `credentialer` held its own copy**, with the login stack as its
    // only consumer. Milestone 111 gave it a second one that outlives the boot: a child whose
    // manifest declares [`grant_plan::Manifest::entropy`] is endowed a `WRITE` view of this same
    // endpoint at spawn, so the progenitor is the only process that can hand a program at the prompt a
    // random source, exactly as it is the only one that can hand it a clock. That costs one
    // permanent capability slot in a table milestone 230 measured at 21 of 24 at peak (the table is 32 now); milestone
    // 231's `capability slots: N of M at peak` line is what says whether that is still true, and it
    // is printed by every boot.
    let mut entropy_client: Option<u64> = None;
    if let Some(ent_program) = ent_elf.as_ref() {
        // `invoke`'s own `NoSuchSlot` (-1) on an ungranted slot is the probe: there is no fourth
        // `START` argument word left to be told with instead (see
        // [`BootEndowment::virtio_rng`]'s own doc). `virtio_rng`/`virtio_rng_irq`/`virtio_rng_dma`
        // are granted as one triple or not at all (`kernel::user::boot_virtio_rng_device`), so a
        // negative answer here means all three slots are empty, not just this one.
        // SAFETY: `invoke` traps to the kernel, which validates the capability and the method
        // before acting; a probe against an empty slot is exactly what that contract is built to
        // answer safely.
        let magic = unsafe { invoke(g.virtio_rng, abi::virtio::READ_REG, 0, 0, 0) };
        if magic >= 0 {
            // SAFETY: as above.
            if unsafe {
                invoke(
                    g.virtio_rng_dma,
                    abi::page_frame::MAP,
                    RNG_DMA_PEEK_VA,
                    1,
                    ut,
                )
            } == 0
            {
                // SAFETY: just mapped read/write, one page, ours alone until entropy is built and
                // holds its own copy of the same frame; `VIRTIO_DMA_PHYS_OFFSET` is inside it and
                // outside entropy's own ring-and-buffer layout (that constant's own doc).
                let direct_memory_access_phys = unsafe {
                    core::ptr::read_unaligned(
                        (RNG_DMA_PEEK_VA as *const u8)
                            .add(VIRTIO_DMA_PHYS_OFFSET as usize)
                            .cast::<u64>(),
                    )
                };
                // Read; the page is entropy's from here, so our view of it goes (milestone 95).
                supervision_protocol::give_up_own_page(RNG_DMA_PEEK_VA);
                let request = must(retype_obj(ut, abi::objtype::RENDEZVOUS));
                let ready = must(retype_obj(ut, abi::objtype::RENDEZVOUS));
                let entropy = must(build_child(
                    ut,
                    ut,
                    ent_program,
                    &ChildEndowment {
                        caps: &[
                            (request, abi::rights::READ),
                            (g.virtio_rng_irq, abi::rights::READ),
                            (g.virtio_rng, abi::rights::WRITE),
                            (ready, abi::rights::WRITE),
                        ],
                        maps: &[(RNG_DMA_VA, g.virtio_rng_dma, abi::address_space::MAP_RW)],
                        stack_pages: CHILD_STACK_PAGES,
                        ..ChildEndowment::new(Retention::Nothing)
                    },
                ));
                must_ok(start_child(
                    entropy,
                    RNG_MODE_VIRTIO,
                    direct_memory_access_phys,
                    0,
                ));
                cap_delete(g.virtio_rng_irq);
                cap_delete(g.virtio_rng);
                cap_delete(g.virtio_rng_dma);
                // Block for the service's own proof of life: it fetches a first bufferful from the
                // real device before answering, so this means "a client that asks will be
                // answered", not merely "the handshake completed" (`components/src/entropy.rs`'s own
                // doc).
                let (verdict, _, _) = receive(ready);
                cap_delete(ready);
                entropy_ready = verdict == entropy_protocol::READY;
                if entropy_ready {
                    // Kept for the login stack below (`credentialer`'s own client view) and, since
                    // milestone 111, for the spawn service after it; see `entropy_client`'s own
                    // doc.
                    entropy_client = Some(request);
                } else {
                    // No login stack will be built without a working entropy service (this
                    // block's own comment on `have_login_stack`), so `request` has no further use
                    // on this path.
                    cap_delete(request);
                }
            } else {
                cap_delete(g.virtio_rng_irq);
                cap_delete(g.virtio_rng);
                cap_delete(g.virtio_rng_dma);
            }
        }
    }
    // **Or the service the kernel built on the CPU's seed instruction** (milestone 595
    // (provisional)), which arrives already proven: the kernel waited for its readiness report and
    // grants nothing on a refusal ([`BootEndowment::entropy_ep`]). One slot rather than the trio's
    // three, granted only when the trio was not, so this block's peak argument above still holds.
    // Nothing is built here, so nothing is spent here either.
    let mut entropy_from_instruction = false;
    if entropy_client.is_none() && kernel_built_entropy {
        entropy_ready = true;
        entropy_from_instruction = true;
        entropy_client = Some(g.entropy_ep);
    }

    // **The network stack** (milestone 590 (provisional), the booted system starts its network
    // stack), when the kernel granted a virtio-net device (slots 13-15) and the archive carries
    // `net_stack`. Built here, straight after entropy and before any of the terminal plumbing, for
    // the entropy block's own reason: the NIC trio is a kernel grant, alive from spawn, so it
    // inflates this function's baseline until the moment it is deleted, and the terminal plumbing
    // is where the early peak is. After this block the trio is gone on every path, built or not,
    // and what is left is one endpoint.
    //
    // **What it is given is what the kernel harness gives it, and no more** (`kernel::user::
    // virtio_service::wire_net_server`, which every network test uses): the confined transport,
    // the interrupt, the DMA page, a heap budget, a report endpoint for its DHCP lease, and `READ`
    // on the endpoint its clients `CALL`. Its inbound authority is `NO_LISTEN_GRANT`: nothing at
    // this prompt may listen, because nothing here has asked to and a listen grant is a policy
    // somebody has to choose (milestone 107 (the socket contract learns to accept)). The proposal this block answers named that as an
    // open question; "none" is the answer for every program the prompt can spawn today.
    //
    // **It blocks the boot until DHCP answers**, which is this block's one real cost, recorded in
    // milestone 590's BUGS: on QEMU's user-mode network the answer is immediate, and on a network
    // with a NIC and no DHCP server the prompt would never appear. The lease is what
    // `net_stack`'s first message says, and it cannot serve a client before it has sent it.
    let mut network: Option<(u64, u64)> = None;
    if is_granted(g.virtio_net) {
        if let Some(net_program) = net_elf.as_ref() {
            network = Some(build_net_stack(ut, net_program, g));
        }
        cap_delete(g.virtio_net_irq);
        cap_delete(g.virtio_net);
        cap_delete(g.virtio_net_dma);
    } else if is_granted(g.net_stack_ep) {
        // **Or the stack the kernel built on an `e1000e`** (milestone 198 (a package manager)),
        // [`BootEndowment::net_stack_ep`]. Nothing to build, so nothing spent; the lease is taken
        // here for the same reason [`build_net_stack`] takes it, and blocks the boot the same way.
        let (lease, _, _) = receive(g.net_stack_report);
        cap_delete(g.net_stack_report);
        network = Some((g.net_stack_ep, lease));
    }

    // **The two components that have to exist before the progenitor can say anything.** The console writes
    // the UART and the line discipline is its only client, so a refusal of either has no route to a
    // person and this is the one case that stops in silence (see this module's BUGS). Everything
    // else is checked below, after they are running.
    let (Some(con_elf), Some(_), Some(ts_elf)) = (con_elf.elf, td_elf.elf, ts_elf.elf) else {
        fail()
    };
    // Vouched above, so these are the measured bytes.
    let td_bytes = fs.read("line_editor").unwrap_or(&[]);

    // `graphical_terminal_grants` ran at the top of this function, before the entropy block: the devices'
    // caps sit in their kernel-granted slots until the shell's build places them, and nothing
    // graphical is built here on any boot (milestone 632 (provisional)).

    // The endpoints and shared pages we own and hand out, each retyped with full rights so we can
    // delegate narrowed views. `term_ep` is the terminal contract's one endpoint: the discipline
    // serves it; the input driver (or `keyboard_driver`) and the shell only hold WRITE on it, and neither can
    // tell what is on the other side (notes/terminal-contract.md). Ours to create on every boot
    // since milestone 600 (provisional): the kernel used to create it for a graphical boot, because
    // it spawned `keyboard_driver` before this process existed and had to wire it to something.
    let term_out = must(retype_page_frame(ut)); // shell -> line_editor text and prompts
    let term_in = must(retype_page_frame(ut)); // line_editor -> shell completed lines

    let term_ep = must(retype_obj(ut, abi::objtype::RENDEZVOUS));

    // Every boot builds the console pair, including one with a gpu attached: the display devices
    // change nothing here (milestone 632 (provisional)), so what follows is the pre-177 shape,
    // unconditional again. `line_editor` runs `MODE_CONSOLE`, the UART is the terminal, and the
    // graphical stack, when a person asks for one, is built by the spawn service inside a `graphical_terminal`
    // session from caps the shell holds.
    {
        let request = must(retype_obj(ut, abi::objtype::RENDEZVOUS));
        let reply = must(retype_obj(ut, abi::objtype::RENDEZVOUS));
        let con_shared = must(retype_page_frame(ut)); // line_editor -> console text

        // 1. Console server: reads text from the shared page, writes it to the UART.
        //
        // **On aarch64/riscv64 the UART is a page**, so the console holds it as a mapped
        // `DeviceFrame` (`CON_UART_VA`). **On x86 COM1 is a port range**, which has nothing to map
        // (DECISIONS §121, reversed 2026-09-15): the console holds it as a `PortRange` capability
        // delegated into its table instead, and the kernel's TSS I/O bitmap is what lets its `out`
        // reach the port. So `g.uart_dev` moves from a `maps` entry to a `caps` entry on x86, and
        // the UART mapping is dropped. It lands in the child's next free slot (2, or 3 behind a
        // screen); the console never invokes it by slot (it executes `out` directly), it only has
        // to hold it.
        //
        // **And a screen beside the UART, when the kernel wired one** (`has_screen`, above): the
        // terminal's endpoint lands in the console's slot 2, ahead of the batching pair (slots 3
        // and 4) and x86's port range (which lands after them; the console holds it but never
        // names it by slot), and the page that terminal reads is mapped at `CON_SCREEN_OUT_VA`.
        // Arrays with a count rather than one slice literal per case, because the screen is a
        // runtime fact and the port range a compile-time one, and five literals would be five
        // places to get the slot order wrong.
        //
        // **The batching pair** (the paint lane, 2026-09-30): a notification and a timer, retyped
        // from this boot's own untyped, the notification bound to the console's thread before it
        // starts (milestone 151 (notification objects: async multiplexing without wait-any)'s
        // `BIND`: a spawner holds the child's `ThreadControlBlock` in
        // exactly this window, between `build_child` and `start_child`, and a running thread cannot
        // bind itself). This is what lets the console paint the screen once per window instead of
        // once per write (`components/src/console.rs`'s own module doc holds the why); the console
        // degrades to one paint per write if this pair is ever missing, so the wiring is the
        // accelerator rather than a precondition.
        let mut con_batch = (0, 0);
        if has_screen {
            let notified = must(retype_obj(ut, abi::objtype::NOTIFICATION));
            let timer = must(retype_obj(ut, abi::objtype::TIMER));
            con_batch = (notified, timer);
        }
        let mut con_caps = [(0, 0); 6];
        let mut con_maps = [(0, 0, 0); 3];
        let (mut ncaps, mut nmaps) = (0, 0);
        for cap in [(request, abi::rights::READ), (reply, abi::rights::WRITE)] {
            con_caps[ncaps] = cap;
            ncaps += 1;
        }
        con_maps[nmaps] = (CON_SHARED_VA, con_shared, abi::address_space::MAP_RO);
        nmaps += 1;
        if has_screen {
            con_caps[ncaps] = (g.disp_term_ep, abi::rights::WRITE);
            ncaps += 1;
            // `READ | WRITE` on the notification: WAIT/POLL and the SIGNAL an armed timer makes of
            // it. `WRITE` on the timer: ARM and CANCEL.
            con_caps[ncaps] = (con_batch.0, abi::rights::READ | abi::rights::WRITE);
            ncaps += 1;
            con_caps[ncaps] = (con_batch.1, abi::rights::WRITE);
            ncaps += 1;
            con_maps[nmaps] = (
                CON_SCREEN_OUT_VA,
                g.disp_term_page,
                abi::address_space::MAP_RW,
            );
            nmaps += 1;
        }
        #[cfg(not(target_arch = "x86_64"))]
        {
            // mode ignored for a DeviceFrame
            con_maps[nmaps] = (CON_UART_VA, g.uart_dev, abi::address_space::MAP_RO);
            nmaps += 1;
        }
        #[cfg(target_arch = "x86_64")]
        {
            con_caps[ncaps] = (g.uart_dev, abi::rights::WRITE); // COM1's port range, held not mapped
            ncaps += 1;
        }
        let con = must(build_child(
            ut,
            ut,
            &con_elf,
            &ChildEndowment {
                caps: &con_caps[..ncaps],
                maps: &con_maps[..nmaps],
                stack_pages: CHILD_STACK_PAGES,
                ..ChildEndowment::new(Retention::Nothing)
            },
        ));
        // The bind belongs in the window between build and start: after `start_child` the retention
        // below has deleted our `ThreadControlBlock`, and before `build_child` there is no thread.
        if has_screen {
            must_ok(user_mode_runtime::notification_bind(con_batch.0, con.tcb) >= 0);
        }
        must_ok(start_child(
            con,
            if has_screen { CONSOLE_MODE_SCREEN } else { 0 },
            0,
            0,
        ));
        // The console holds its own narrowed copies of the terminal's endpoint and page now, and
        // nothing else in this boot prints to that terminal directly: everyone prints through the
        // console. Freed here rather than with `request`/`reply` below for the sixteen-slot reason
        // this function's every early free gives, and the batch pair goes with them: the binding
        // lives in the objects, not in our copies of them.
        if has_screen {
            cap_delete(g.disp_term_ep);
            cap_delete(g.disp_term_page);
            cap_delete(con_batch.0);
            cap_delete(con_batch.1);
        }

        // 1b. **The system log service, draining the kernel's ring into the console** (milestone
        // 342 (the kernel and the `console` server drive one UART from two address spaces), §175
        // (where the kernel's own output goes)'s ruling B). Built right after the console, because
        // until it attaches the kernel still prints for itself and can splice with the console's
        // bytes; this keeps that window to the build of one program. It holds the ring read-only,
        // the cursor page, the ring's notification bound to its thread, a timer, and `WRITE` on the
        // console's request endpoint, through which it sends kernel lines the console writes only
        // at a line start.
        if let Some(log_program) = log_elf.as_ref()
            && is_granted(g.kernel_ring)
            && is_granted(g.kernel_ring_cursor)
            && is_granted(g.kernel_ring_notification)
        {
            let intake = must(retype_obj(ut, abi::objtype::RENDEZVOUS));
            let timer = must(retype_obj(ut, abi::objtype::TIMER));
            let log = must(build_child(
                ut,
                ut,
                log_program,
                &ChildEndowment {
                    caps: &[
                        (intake, abi::rights::READ),
                        (
                            g.kernel_ring_notification,
                            abi::rights::READ | abi::rights::WRITE,
                        ),
                        (timer, abi::rights::WRITE),
                        (request, abi::rights::WRITE),
                    ],
                    maps: &[
                        (LOG_RING_VA, g.kernel_ring, abi::address_space::MAP_RO),
                        (
                            LOG_CURSOR_VA,
                            g.kernel_ring_cursor,
                            abi::address_space::MAP_RW,
                        ),
                    ],
                    stack_pages: CHILD_STACK_PAGES,
                    ..ChildEndowment::new(Retention::Nothing)
                },
            ));
            // In the window between build and start, as the console's batching notification is.
            must_ok(user_mode_runtime::notification_bind(g.kernel_ring_notification, log.tcb) >= 0);
            must_ok(start_child(log, LOG_MODE_KERNEL, 0, 0));
            // Nothing at this boot registers a log writer yet, so the unbadged intake is not kept
            // (the service's own BUGS records it), and the timer is the service's alone.
            cap_delete(intake);
            cap_delete(timer);
        }
        // Ours no further either way: the service holds its own, and a boot that did not start one
        // must not carry them across the login block's peak.
        cap_delete(g.kernel_ring);
        cap_delete(g.kernel_ring_cursor);
        cap_delete(g.kernel_ring_notification);

        // 2. The line discipline: serves the terminal endpoint, prints through the console. It is
        // the console's only client; everyone else prints through it. `LINE_EDITOR_MODE_CONSOLE`
        // (0) is `start_child`'s own default, so this is unchanged from before
        // milestone 177 gave `line_editor` a second mode.
        build_terminal(
            ut,
            ut,
            &ts_elf,
            td_bytes,
            LINE_EDITOR_MODE_CONSOLE,
            &[
                (
                    term_ep,
                    abi::rights::READ | abi::rights::WRITE | abi::rights::GRANT,
                ),
                (request, abi::rights::WRITE | abi::rights::GRANT),
                (con_shared, abi::rights::READ | abi::rights::WRITE), // it fills what the console reads
                (term_out, abi::rights::READ),
                (term_in, abi::rights::READ | abi::rights::WRITE),
                (reply, abi::rights::READ | abi::rights::GRANT),
            ],
        );

        // `request`/`reply`/`con_shared` are ours no further: the console and `line_editor` both
        // hold their own narrowed copies (the console's own doc, one screen up, gives the
        // capability-table reason this cannot wait).
        for c in [request, reply, con_shared] {
            cap_delete(c);
        }
    }

    // **Refuse the system if a required component is not the one that was measured** (milestone
    // 104), here, because this is the earliest point at which the progenitor can be read by a person and the
    // latest at which nothing unmeasured has been built. The console and the line discipline above
    // are running; nothing else is.
    //
    // Halting is not a second policy. The policy is that the progenitor grants nothing of its own to
    // what it cannot vouch for (§219), which for an archive program means it does not run it, and
    // for a component the whole system is made of, not running it and not having a system are the
    // same outcome. What it costs is decided by what the program was for, which is a question the progenitor
    // already had to answer for an archive entry that is simply missing.
    let unvouched: [&str; 3] = [
        if in_elf.unvouched { "input" } else { "" },
        if sh_elf.unvouched { "swish" } else { "" },
        if reaper_elf.unvouched {
            "job_undertaker"
        } else {
            ""
        },
    ];
    if unvouched.iter().any(|n| !n.is_empty()) {
        // The shell's output page, in our own space, so the refusal can be read. The giveaway below
        // maps it for the same reason and this path never reaches the giveaway; a refusal nobody
        // can see is most of what this milestone was written to fix.
        // SAFETY: `invoke` traps to the kernel, which validates the capability and the method before
        // acting (user_mode_runtime's contract).
        if unsafe { invoke(term_out, abi::page_frame::MAP, INIT_OUT_VA, 1, ut) } == 0 {
            let mut buf = [0u8; SENTENCE];
            announce(
                term_ep,
                sentence(
                    &mut buf,
                    b"progenitor: cannot vouch for",
                    &unvouched,
                    b"; halting rather than building an unmeasured system\n",
                ),
            );
        }
        fail()
    }
    // Past the refusal, so an absent component still traps the way it always has: a build that did
    // not pack the console's client is a broken build, not an attack, and it has never had a
    // message.
    let (Some(in_elf), Some(sh_elf), Some(reaper_elf)) = (in_elf.elf, sh_elf.elf, reaper_elf.elf)
    else {
        fail()
    };

    // 3. The keystroke source: the input driver, waiting on the UART receive interrupt and
    // forwarding raw bytes to the terminal, in the same `OPERATION_BYTES` framing a `graphical_terminal` session's
    // `keyboard_driver` uses (milestone 192's "one place decides where a keystroke comes from").
    // The one keystroke source any boot builds: a virtio keyboard's driver is built inside a
    // session the user launched, never here (milestone 632 (provisional)), and nothing downstream
    // of `term_ep` can tell the two apart.
    //
    // **Input is interrupt-driven on all three architectures**: it holds the receive interrupt
    // (`g.uart_irq`, slot 1) on each. The device differs: the UART page (`IN_UART_VA`) on
    // aarch64/riscv64, and on x86 COM1's port range (slot 2), the same capability the console holds
    // (milestone 299 (the x86 port-range capability: the serial console becomes a userspace
    // driver); see `components/src/input.rs`). It never invokes the port cap by slot;
    // holding it is what the TSS bitmap enforcement reads. x86 polled the port with no interrupt
    // until milestone 505 (an x86_64 input driver that never lets the core idle).
    #[cfg(not(target_arch = "x86_64"))]
    let (in_caps, in_maps): EndowmentSlices = (
        &[
            (term_ep, abi::rights::WRITE),
            (g.uart_irq, abi::rights::READ),
        ],
        &[(IN_UART_VA, g.uart_dev, abi::address_space::MAP_RO)],
    );
    #[cfg(target_arch = "x86_64")]
    let (in_caps, in_maps): EndowmentSlices = (
        &[
            (term_ep, abi::rights::WRITE),
            (g.uart_irq, abi::rights::READ),
            (g.uart_dev, abi::rights::WRITE), // COM1's port range, read by the TSS I/O bitmap
        ],
        &[],
    );
    let input = must(build_child(
        ut,
        ut,
        &in_elf,
        &ChildEndowment {
            caps: in_caps,
            maps: in_maps,
            stack_pages: CHILD_STACK_PAGES,
            ..ChildEndowment::new(Retention::Nothing)
        },
    ));
    must_ok(start_child(input, 0, 0, 0));

    // 3b. **A second keystroke source, a USB keyboard** (milestone 242 (USB host and HID)), when
    // the kernel started one: `WRITE` on the same terminal endpoint `input` holds, delegated
    // through the driver's attach endpoint. The driver is already waiting there (it receives
    // before it does anything else after its bring-up report), so this send does not wait on
    // anything that could fail to come. Both sources then feed one line discipline, and a person
    // can type on the keyboard or the serial line alike.
    if is_granted(g.usb_keyboard_attach) {
        must_ok(
            user_mode_runtime::send_cap(g.usb_keyboard_attach, term_ep, abi::rights::WRITE, 0) >= 0,
        );
        cap_delete(g.usb_keyboard_attach);
    }

    // **The console's capabilities go back now, before the shell is built**, and that is not
    // tidiness: this capability table has sixteen slots, and milestone 50 added two more kernel grants (the
    // file service and its page). With them held, the shell's `build_child` had no slot left to
    // retype an address space into and failed silently, which presented as a boot that brought up
    // the console and then printed nothing. Nothing below needs these: line_editor is the console's
    // only client and it already holds its narrowed copies.
    //
    // **The device and the interrupt go with them** (milestone 22, the interactive increment). Both
    // drivers that need them exist and hold their own narrowed copies, and nothing below builds
    // another driver, so a progenitor that kept them would be keeping the authority to hand the UART to
    // anything it later builds. Dropping them here is the same act as dropping the construction
    // budget further down, one boot stage earlier.
    cap_delete(g.uart_dev);
    cap_delete(g.uart_irq);
    // `for_test_roles` no longer needs freeing here: it is freed at the top of this function now,
    // two component-builds earlier, for the reason recorded there.

    // **The spawn channel is retyped here, not with the rest**, and the reason is the same sixteen
    // slots: holding two more endpoints through the three builds above is what pushed this capability table
    // over. They are the shell's and the service's, so this is also where they belong.
    let spawn_ep = must(retype_obj(ut, abi::objtype::RENDEZVOUS));
    let result_ep = must(retype_obj(ut, abi::objtype::RENDEZVOUS));
    // **The supervision endpoint every job is born holding** (milestone 22, the interactive
    // increment; DECISIONS §26's spawn-slot convention). We keep it for its `GRANT`, which is all we
    // need it for: to place a `READ` view of it in each job's reserved fault slot. We never receive
    // on it. `job_undertaker` does, and collecting is the only thing that endpoint authorizes.
    let deaths = must(retype_obj(ut, abi::objtype::RENDEZVOUS));

    // 4. The shell: prints and reads lines through the terminal, holds the spawn channel, and holds
    // its own untyped budget (slot 3) so `run --mem N` grants from memory that is genuinely the
    // shell's. WRITE lets it SPLIT the budget; GRANT lets it delegate the split to the progenitor. We carve
    // that budget from our own untyped and hand it over the same way we hand any capability.
    //
    // Slot 4 is the filesystem when this boot has one, which is the whole of what `>` and `<` need
    // (milestone 50, notes/pipes.md): the shell resolves a redirection against it and writes the
    // file itself. Narrowed to WRITE, which on an endpoint is the right to CALL, and without GRANT,
    // so the shell can hand it to nobody.
    //
    // **And a read-only clock last** (milestone 86), which is what `time <command>` measures with.
    // It is [`BootEndowment::clock_page`], the same frame this progenitor was granted and hands to a child
    // whose manifest declares a clock; the shell is simply another holder of a narrowed view. `READ`
    // and no `GRANT`, deliberately: the shell can read the wall clock and can hand one to nothing it
    // spawns, so which processes can read the time is still decided by the manifests this crate
    // reads (DECISIONS §43) rather than by anything typed at a prompt.
    //
    // Last in the list, so a boot with no filesystem takes exactly the path it took before this
    // existed. Its slot therefore moves (4 without a disk, 5 with one), which is why the shell is
    // *told* the number in `x2`/`a2` instead of assuming one; see swish.rs's `CLOCK_SLOT`.
    let sh_budget = must(memory_region_split(ut, SH_BUDGET_PAGES));
    let with_fs = fs_rights != 0;

    // **The second grant, when this boot was configured with one** (milestone 154's "wiring a
    // second grant into the real boot"). Built here, before the shell, for the same reason
    // `build_caretaker` is always called before the process that will hold its endpoint: the
    // caretaker must exist and answer ready before anything could `CALL` it.
    //
    // Meaningless without a first directory ([`SecondDirGrant`]'s own doc), so this is `None`
    // whenever the boot has no filesystem at all, and it degrades to "no second grant" rather
    // than a boot failure whenever the caretaker cannot be built, the same posture this file
    // already takes toward a missing `sink_elf` or `care_elf`: a component the archive did not
    // pack or the table would not vouch for costs a feature, not a boot.
    //
    // # BUGS
    //
    // **Unverified against a real boot.** `second_dir` is `None` at every shipped entry point
    // (DECISIONS §126 (a real, single, moving `cwd`): what the subtree should be is an architect's
    // call), so this branch has never
    // run
    // under `script/swish-check`, which is the only thing in the tree that runs a real progenitor.
    // `build_caretaker` retypes two more objects into *this process's* capability table right
    // where the comment two screens up already documents this table as tight ("the shell's
    // `build_child` had no slot left ... and failed silently"). The failure mode if this pushes
    // the progenitor over sixteen slots is exactly that one: a boot that reaches userspace and prints
    // nothing. Whoever first passes `Some` here should watch for it and run `script/swish-check`
    // before trusting this path.
    let second_dir_ep: Option<u64> = second_dir.filter(|_| with_fs).and_then(|sd| {
        assert!(
            filesystem_protocol::grant::fits(sd.name.as_bytes()),
            "a granted directory's name rides in two argument words; this one does not fit",
        );
        let region = memory_region_split(ut, SECOND_DIR_CARETAKER_PAGES).ok()?;
        let care = care_elf.as_ref()?;
        let (lo, hi) = filesystem_protocol::grant::pack_name(sd.name.as_bytes());
        let spec = filesystem_protocol::grant::spec(sd.name.len(), sd.rights);
        // Window 0's page, sliced out of the pool for the build and dropped after (milestone 599).
        let zero = window_zero(Fs {
            ep: g.fs_ep,
            page: g.fs_page,
        })?;
        let built = build_caretaker(ut, region, care, zero, (lo, hi, spec), None);
        cap_delete(zero.page);
        built
    });

    // Slot 4 is the filesystem when this boot has one, which is the whole of what `>` and `<` need
    // (milestone 50, notes/pipes.md): the shell resolves a redirection against it and writes the
    // file itself. Narrowed to WRITE, which on an endpoint is the right to CALL, and without GRANT,
    // so the shell can hand it to nobody.
    //
    // **Slot 5, when this boot also has a second grant**, is [`second_dir_ep`]: the narrowed
    // caretaker endpoint built above. It shares [`SH_FS_VA`] rather than needing a map of its own,
    // for `narrow_dir`'s own reason one level narrower: caretaker and client both map the *same*
    // physical frame the FS server shares with everything downstream of it, and the shell is one
    // thread of control with at most one `CALL` in flight, so it is never mid-request on both
    // endpoints at once.
    //
    // **And a read-only clock last, always** (milestone 86), which is what `time <command>`
    // measures with. It is [`BootEndowment::clock_page`], the same frame this progenitor was granted and
    // hands to a child whose manifest declares a clock; the shell is simply another holder of a
    // narrowed view. `READ` and no `GRANT`, deliberately: the shell can read the wall clock and
    // can hand one to nothing it spawns, so which processes can read the time is still decided by
    // the manifests this crate reads (DECISIONS §43) rather than by anything typed at a prompt.
    //
    // Clock last in the list whatever else is granted, which is why the shell is *told* its slot
    // in `x2`/`a2` instead of assuming one; see swish.rs's `CLOCK_SLOT`. That was already true
    // before this milestone (4 without a disk, 5 with one); it now also moves to 6 when a second
    // grant lands, and the mechanism that keeps the shell honest about the number is unchanged.
    let mut sh_caps = [(0u64, 0u64); 7];
    let mut sh_maps = [(0u64, 0u64, 0u64); 5];
    let mut n_caps = 0usize;
    let mut n_maps = 0usize;
    sh_caps[n_caps] = (term_ep, abi::rights::WRITE);
    n_caps += 1;
    sh_caps[n_caps] = (spawn_ep, abi::rights::WRITE);
    n_caps += 1;
    sh_caps[n_caps] = (result_ep, abi::rights::READ);
    n_caps += 1;
    sh_caps[n_caps] = (sh_budget, abi::rights::WRITE | abi::rights::GRANT);
    n_caps += 1;
    sh_maps[n_maps] = (SH_OUT_VA, term_out, abi::address_space::MAP_RW); // text and prompts
    n_maps += 1;
    sh_maps[n_maps] = (LINE_VA, term_in, abi::address_space::MAP_RO); // completed lines
    n_maps += 1;
    // **Window 0, not the pool** (milestone 599): the kernel's slot 6 names every client window
    // as one run, so the shell maps a one-page slice of window 0, deleted once it is built.
    let sh_window = if with_fs {
        window_zero(Fs {
            ep: g.fs_ep,
            page: g.fs_page,
        })
    } else {
        None
    };
    if let Some(zero) = sh_window {
        sh_caps[n_caps] = (zero.ep, abi::rights::WRITE);
        n_caps += 1;
        sh_maps[n_maps] = (SH_FS_VA, zero.page, abi::address_space::MAP_RW);
        n_maps += 1;
    }
    if let Some(ep) = second_dir_ep {
        sh_caps[n_caps] = (ep, abi::rights::WRITE);
        n_caps += 1;
    }
    sh_caps[n_caps] = (g.clock_page, abi::rights::READ);
    n_caps += 1;
    sh_maps[n_maps] = (SH_CLOCK_VA, g.clock_page, abi::address_space::MAP_RO);
    n_maps += 1;
    // **The inert-configuration page, mapped for `caps` to read** (milestone 47 (navigation and
    // naming), DECISIONS §111 (inert configuration is a validated page)'s "extend that preview to
    // print the actual values"). The frame every child declaring `Manifest::config` is endowed
    // with, so what the shell prints is what the child will read. Its capability is placed at a
    // named slot after the build (below), not in this list, because the list's slots move with what
    // the boot granted and the shell probes rather than being told.
    sh_maps[n_maps] = (
        grant_plan::SHELL_CONFIG_VA,
        g.config_page,
        abi::address_space::MAP_RO,
    );
    n_maps += 1;
    // Which slot the clock landed in, for the shell's `x2`: the count of what went before it, the
    // same arithmetic `build_child` does when it fills the capability table from zero.
    let sh_clock_slot: u64 = n_caps as u64 - 1;
    // **Built but not started**, because the drop below has to happen while the shell's output page
    // is still ours alone to write: the negative control is printed through it, and a running shell
    // would be printing its banner into the same page.
    let shell = must(build_child(
        ut,
        ut,
        &sh_elf,
        &ChildEndowment {
            caps: &sh_caps[..n_caps],
            maps: &sh_maps[..n_maps],
            stack_pages: CHILD_STACK_PAGES,
            ..ChildEndowment::new(Retention::Nothing)
        },
    ));
    cap_delete(sh_budget); // our copy; the shell holds its own now
    if let Some(zero) = sh_window {
        cap_delete(zero.page); // the slice was only the means; the shell holds the mapping
    }
    // `READ` and no `GRANT`, the clock's rights for the clock's reason: the shell can read the
    // configuration and can hand it to nothing it spawns, so which children see it is still decided
    // by their manifests. Placed before the shell starts, which is what makes its `_start` probe of
    // this slot sound.
    must_ok(place_at(
        shell.tcb,
        g.config_page,
        abi::rights::READ,
        grant_plan::SHELL_CONFIG_SLOT,
    ));
    // **The machine statistics page goes to the session, and this process keeps no copy**
    // (milestone 126, DECISIONS §225). Placed now, before the login block, because that block is
    // this table's peak and a page held across it would spend the table's last slot
    // (`kernel::cap::CAPABILITY_TABLE_PEAK_MEASURED`). The shell hands it back with a spawn request
    // only for a program that declares `machine` (`spawnproto::MACHINE_PAGE_SLOT`, `MACHINE_BIT`),
    // so which programs see the machine is decided by what the session holds. `READ | GRANT`: it
    // can delegate the page and cannot write it. [`GRANT_MACHINE_PAGE`] is the owner's switch.
    if GRANT_MACHINE_PAGE {
        must_ok(place_at(
            shell.tcb,
            g.machine_page,
            abi::rights::READ | abi::rights::GRANT,
            spawnproto::MACHINE_PAGE_SLOT,
        ));
    }
    cap_delete(g.machine_page);
    // **The display devices stay here, and the shell gets none** (milestone 715 (provisional), the
    // 2026-10-03 security audit's follow-up). Until 715 the gpu's four and the keyboard's three
    // were placed in the shell at `spawnproto::SHELL_GPU_SLOT` onward, with `GRANT`, for the life
    // of the boot, and came back with each `graphical_terminal` request. The spawn service builds
    // the session's drivers from its own copies instead, which is `term_ep`'s posture for the same
    // session: the prompt asks for a launch and cannot hand a device to anything. They stay in the
    // boot endowment's own slots, read from [`GraphicalTerminalCaps`]; [`graphical_terminal_grants`]
    // already deleted whatever no session can be built from.
    // The caretaker's endpoint was only ever the means of wiring: the shell holds its own copy and
    // the caretaker holds the other end, the same disposal `spawn_service`'s dynamic directory
    // grants already give their own narrowed endpoint below.
    if let Some(ep) = second_dir_ep {
        cap_delete(ep);
    }

    // Free every boot cap the spawn service does not need, so the progenitor's 16-slot capability table has room to
    // build a supervised child (which holds a job untyped and a job frame while the loader retypes
    // an address space, frames, and a TCB). The drivers and the shell hold the narrowed copies that matter.
    //
    // **Only the input frame, and the two that stay have a reason each.** `term_ep` is still ours to
    // delegate: the sink adapter below is handed `WRITE` on it, and the drop announcement is a
    // `CALL` on it. `term_out` is where that announcement stages its bytes. Both go back the moment
    // their last use is done, and after that this process holds no way to reach the terminal at all.
    cap_delete(term_in);
    // **The filesystem stays** (milestone 31 phase 3, 2026-08-17). It used to go here, with a
    // comment saying "the day `rm` is reachable from the prompt, the progenitor keeps the endpoint instead,
    // because building a `fs_subtree_caretaker` is its job and not the shell's". This is that day.
    //
    // Two slots held for the life of the boot, and it is worth being precise about what they buy and
    // what they cost. They buy the only delivery mechanism a directory grant has: the caretaker must
    // hold the file service to attenuate it, the shell's copy carries no `GRANT`, and a program
    // spawned without the capability its command line named would be the worst failure this model
    // has. They cost two of the progenitor's sixteen capability table slots, permanently, which takes the spawn service's
    // resting endowment from seven capabilities to nine and its peak from thirteen to fifteen. That
    // peak is the number to watch: it is a directory-granted spawn, and it is one slot from the wall.
    // See `spawn_dir_grant`, which counts it.
    let fs = with_fs.then_some(Fs {
        ep: g.fs_ep,
        page: g.fs_page,
    });

    // 5. **The terminal's sink adapter** (milestone 50's last remainder, notes/sink-protocol.md,
    // DECISIONS §67). It holds the terminal `WRITE` and serves the sink contract on an endpoint of
    // its own, so a child can be handed "the terminal" as a place to put bytes **without** being
    // handed the terminal endpoint, which also carries `OPERATION_READLINE` and would be the keyboard.
    //
    // **After the shell and before the giveaway, and both halves of that are load-bearing.**
    //
    // After the shell, because of this capability table's sixteen slots: building the adapter earlier put the progenitor
    // one slot over while the loader was retyping the shell's address space, and the symptom was
    // the one this system has already seen, a boot that reaches userspace and then prints nothing at
    // all. That constraint is about the shell's build, not about being the last thing built, and
    // milestone 22 is what made the difference visible: the adapter is now the fifth of six boot
    // components rather than the last of five, and the capability table has room either way.
    //
    // Before the giveaway, because this is a **system** component and the root untyped is what the
    // system is built from. Everything below hands that budget away and proves it is gone, so an
    // adapter built after it would have to come out of [`INIT_OWN_PAGES`], the scratch budget sized
    // for page tables and nothing else. Spending a whole program out of that pool would be invisible
    // here and would surface as some later child failing to map a scratch page.
    let term_sink = must(retype_obj(ut, abi::objtype::RENDEZVOUS));
    if let Some(elf) = sink_elf.as_ref() {
        let adapter = must(build_child(
            ut,
            ut,
            elf,
            &ChildEndowment {
                caps: &[
                    (term_sink, abi::rights::READ),
                    (term_ep, abi::rights::WRITE),
                ],
                stack_pages: CHILD_STACK_PAGES,
                ..ChildEndowment::new(Retention::Nothing)
            },
        ));
        // Started here even though the shell deliberately is not: the adapter owns no page and
        // prints only what a client sends it, and it has no clients until the spawn service below
        // hands one its endpoint. It cannot write into the page the announcement below stages in.
        must_ok(start_child(adapter, 0, 0, 0));
    }

    // **Milestone 49's login stack**: `credentialer`, `identity_provisioner`, `login`,
    // `login_audit_receiver`. Built here, after the shell (this table's own tightest peak, "one
    // slot from the wall") has already returned its own transient capabilities, and before the
    // giveaway below spends `ut` down to nothing: everything this block needs (`ut` itself,
    // `term_ep`, the file service pair, a live client view of entropy) is still ours to spend here
    // and nowhere later, the identical reasoning the entropy block's own comment gives for building
    // *that* where it does.
    //
    // Optional, in the sink adapter's own sense: a boot with no filesystem, no working entropy
    // service, or any of these four programs missing or unvouched for simply has no login path,
    // and this whole block is skipped rather than faulted on. `login_ready`/`login_password` are
    // carried forward as data (the same shape `entropy_ready` already uses) because this process
    // has no terminal mapped yet at this point in the boot; the actual announcement happens later,
    // at the existing print site, right before `term_ep` is dropped.
    let have_login_stack = with_fs
        && entropy_client.is_some()
        && cred_elf.is_some()
        && idp_elf.is_some()
        && login_elf.is_some()
        && audit_elf.is_some();
    let mut login_ready = false;
    let mut login_password = [0u8; PASSWORD_HEX_LEN];
    // **The run-unvouched endpoint** (DECISIONS §219 gate D2), retyped the first time something is
    // placed on it: inside the block below when `login` is built, and after it otherwise. Never
    // before `login`'s build, because this process's table peaks inside this block and this
    // endpoint is held for the life of the boot; see [`Channels::run_unvouched`].
    let mut run_unvouched: Option<u64> = None;
    if !have_login_stack {
        // **Not given back any more** (milestone 111). A boot with a working entropy service and
        // no login stack (no filesystem, or one of the four programs missing) still reaches a
        // prompt, and `uuid` at that prompt needs this endpoint; before, the slot was released
        // here because nothing downstream had a use for it.
    } else {
        let request = entropy_client.expect("have_login_stack checked entropy_client.is_some()");

        // **The generated password, drawn before anything else in this block**: `credentialer`'s
        // own posture ("it never invents a salt") one level up. No unpredictable bits, no
        // password, and this whole block is abandoned rather than provisioning one with something
        // weaker (DECISIONS §42).
        let mut password_bytes = [0u8; PASSWORD_BYTES];
        // `request` is not released on either arm any more (milestone 111): the spawn service holds
        // it for the life of the boot, so a service that could not fill a password is still a
        // service a program at the prompt may `CALL` and be refused by honestly.
        if !fill_entropy(request, &mut password_bytes) {
        } else {
            hex_password(&password_bytes, &mut login_password);

            // 1. **`credentialer`** (milestone 56/65, the secrets service): built the same way
            // entropy was, permanent for the life of the boot, holding the client view of entropy
            // this block just drew a password from.
            let prov = must(retype_obj(ut, abi::objtype::RENDEZVOUS));
            let verify = must(retype_obj(ut, abi::objtype::RENDEZVOUS));
            let cred_ready = must(retype_obj(ut, abi::objtype::RENDEZVOUS));
            let prov_page = must(retype_page_frame(ut));
            let verify_page = must(retype_page_frame(ut));
            let cred_budget = must(memory_region_split(ut, CRED_BUDGET_PAGES));
            let cred_program = cred_elf.as_ref().expect("have_login_stack checked this");
            let cred_child = must(build_child(
                ut,
                ut,
                cred_program,
                &ChildEndowment {
                    caps: &[
                        (prov, abi::rights::READ),
                        (verify, abi::rights::READ),
                        (request, abi::rights::WRITE),
                        (cred_budget, abi::rights::WRITE),
                        (cred_ready, abi::rights::WRITE),
                    ],
                    maps: &[
                        (CRED_SVC_PROV_VA, prov_page, abi::address_space::MAP_RW),
                        (CRED_SVC_VERIFY_VA, verify_page, abi::address_space::MAP_RW),
                    ],
                    stack_pages: CRED_STACK_PAGES,
                    ..ChildEndowment::new(Retention::Nothing)
                },
            ));
            must_ok(start_child(cred_child, 0, 0, 0));
            // `request` is deliberately **not** deleted here, unlike `cred_budget` below.
            // `credentialer` holds its own copy, and so does this process, because the spawn
            // service endows the same endpoint to a child declaring `Manifest::entropy`
            // (milestone 111). See `entropy_client`'s own doc for the slot it costs.
            cap_delete(cred_budget);

            // **`cred_ready` is not read yet, and that ordering is load-bearing rather than an
            // oversight.** `credentialer.rs`'s own `_start` sends its one readiness message
            // (`RPT_READY`) only *after* `provision()` returns, which is only after this process's
            // own `SEAL` arrives (`credentialer.rs`'s own "Two phases" doc: phase one is `RECEIVE` on
            // the provision endpoint, forever, until sealed). A `receive(cred_ready)` here, before
            // provisioning has even been attempted, is not "wait for the service to come up" the
            // way the entropy block's own `receive(ready)` is -- it is "wait for a message that
            // cannot exist until this same function seals the store a few lines further down", a
            // deadlock this process would never wake from. Found by running `script/swish-check`
            // and watching it hang rather than fault: no `[PANIC]`, nothing kept building, because
            // this process was genuinely blocked rather than trapped.
            //
            // 2. **Provision the generated credential and the demo identity's home subtree, as
            // one act, through `identity_provisioner`** (milestone 155): stage the plaintext
            // into a page this process maps briefly, hand it to the tool along with `prov`
            // (before its seal) and this boot's own `g.fs_ep`.
            let req_frame = must(retype_page_frame(ut));
            // SAFETY: `invoke` traps to the kernel, which validates the capability and the
            // method before acting.
            if unsafe { invoke(req_frame, abi::page_frame::MAP, PROVISION_SCRATCH_VA, 1, ut) } != 0
            {
                cap_delete(req_frame);
            } else {
                // SAFETY: just mapped read/write, one page, ours alone until it is delegated
                // on below.
                let req_slice = unsafe {
                    core::slice::from_raw_parts_mut(
                        PROVISION_SCRATCH_VA as *mut u8,
                        credential_protocol::PAGE,
                    )
                };
                req_slice.fill(0);
                let placed = credential_protocol::place(
                    req_slice,
                    DEMO_IDENTITY,
                    &login_password,
                    credential_protocol::provision::PUT,
                );
                placed.expect(
                    "DEMO_IDENTITY and login_password are both well within credential_protocol's \
                     bounds by construction; a None here is a logic bug in this file, not a \
                     runtime condition",
                );

                let idp_report = must(retype_obj(ut, abi::objtype::RENDEZVOUS));
                let idp_program = idp_elf.as_ref().expect("have_login_stack checked this");
                // **Window 0's page, mapped after the build rather than during it** (milestone
                // 599). This is the table's peak (the login block), and a slice held across the
                // build would be a twenty-fifth slot. So the space is built with the two pages the
                // provisioner shares with this process, `req_frame` goes (its mapping stays, here
                // and in the child), and only then is window 0 sliced, mapped and dropped: the
                // slice spends the slot `req_frame` gave back.
                let (idp_child, idp_aspace) = must(supervision_protocol::build_child_space(
                    ut,
                    ut,
                    idp_program,
                    &ChildEndowment {
                        caps: &[
                            (prov, abi::rights::WRITE),
                            (g.fs_ep, abi::rights::WRITE),
                            (idp_report, abi::rights::WRITE),
                        ],
                        maps: &[
                            (IDP_REQ_VA, req_frame, abi::address_space::MAP_RW),
                            (IDP_PROV_VA, prov_page, abi::address_space::MAP_RW),
                        ],
                        stack_pages: CHILD_STACK_PAGES,
                        ..ChildEndowment::new(Retention::Nothing)
                    },
                ));
                cap_delete(req_frame);
                let idp_page = must(window_page(g.fs_page, 0).ok_or(()));
                // SAFETY: the syscall; the kernel validates both capabilities and the address.
                let mapped = unsafe {
                    invoke(
                        idp_aspace,
                        abi::address_space::MAP_INTO,
                        IDP_FS_VA,
                        idp_page,
                        abi::address_space::MAP_RW,
                    )
                } == 0;
                cap_delete(idp_page);
                must(mapped.then_some(()).ok_or(()));
                must(supervision_protocol::configure_child(
                    idp_child.tcb,
                    idp_aspace,
                    idp_program.entry(),
                ));
                must_ok(start_child(
                    idp_child,
                    DEMO_IDENTITY.len() as u64,
                    login_password.len() as u64,
                    0,
                ));

                let (idp_code, _, _) = receive(idp_report);
                cap_delete(idp_report);
                login_ready = idp_code == IDP_RPT_OK;

                // Wipe the staged plaintext regardless of outcome: this process's own copy of
                // the generated password should not sit in memory longer than provisioning
                // needs it.
                // SAFETY: still the same page this process mapped above, and the only writer
                // between it being staged and this wipe.
                let req_slice = unsafe {
                    core::slice::from_raw_parts_mut(
                        PROVISION_SCRATCH_VA as *mut u8,
                        credential_protocol::PAGE,
                    )
                };
                credential_protocol::wipe(req_slice);
            }
            // Ours no further either way: `identity_provisioner` holds its own copy when it was
            // built at all, and this process never maps this page again.
            cap_delete(prov_page);

            // **Seal the store regardless of whether provisioning succeeded.** `credentialer`
            // must leave phase one before its `VERIFY` endpoint will ever answer anything
            // (`credentialer.rs`'s own "two phases" doc), and a login against an empty or
            // partially-provisioned store should deny cleanly rather than the whole boot
            // hanging with a service that can never be asked a real question.
            call(
                prov,
                credential_protocol::req(credential_protocol::provision::SEAL, 0, 0),
                0,
            );
            cap_delete(prov);

            // **Now** `cred_ready` answers: the seal just sent is exactly what makes
            // `provision()` return and `credentialer.rs`'s own `_start` reach its one
            // `send(READY, RPT_READY, ...)`. A dead-on-arrival service (its own
            // `E_ENTROPY`/`E_SCRATCH` startup failure, which happens before phase one even
            // begins) still answers here too, with its own `0xDEAD_...` word instead, so this
            // `receive` is never left permanently unanswered by that path either.
            let (cred_rv, _, _) = receive(cred_ready);
            cap_delete(cred_ready);
            if cred_rv != CRED_RPT_READY {
                cap_delete(verify);
                cap_delete(verify_page);
                login_ready = false;
            } else {
                // 3. **`login`** (milestone 49): built the same way, holding narrowed views of
                // what this process already has (the file service pair, a fresh construction
                // budget) plus a client view of `credentialer`'s own verify endpoint just built,
                // and (this update) a `WRITE | GRANT` view of the terminal so it can hand the
                // single-session terminal to its first successful caller
                // (`components/src/login.rs`'s "The terminal: single-session, deny cleanly").
                //
                // `login_audit_receiver` is built *first* and started before `login` ever runs,
                // so the receiver for `login`'s blocking `AUDIT` send exists before there is any
                // way to reach it (`components/src/login_audit_receiver.rs`'s own doc on why this
                // ordering matters).
                let audit = must(retype_obj(ut, abi::objtype::RENDEZVOUS));
                let audit_program = audit_elf.as_ref().expect("have_login_stack checked this");
                let audit_child = must(build_child(
                    ut,
                    ut,
                    audit_program,
                    &ChildEndowment {
                        caps: &[(audit, abi::rights::READ)],
                        stack_pages: CHILD_STACK_PAGES,
                        ..ChildEndowment::new(Retention::Nothing)
                    },
                ));
                must_ok(start_child(audit_child, 0, 0, 0));

                let login_request = must(retype_obj(ut, abi::objtype::RENDEZVOUS));
                let login_result = must(retype_obj(ut, abi::objtype::RENDEZVOUS));
                let login_ut = must(memory_region_split(ut, LOGIN_CONSTRUCTION_PAGES));
                let login_program = login_elf.as_ref().expect("have_login_stack checked this");
                // **Window 0's page, never the pool** (milestone 599). Login maps what it is handed
                // into every session caretaker whole, so the pool here would be every client's
                // window in every session. A transient slot during the login block, which is where
                // this table's peak is: `script/swish-check` is what confirms it still fits.
                let login_page = must(window_page(g.fs_page, 0).ok_or(()));
                let login_child = must(build_child(
                    ut,
                    ut,
                    login_program,
                    &ChildEndowment {
                        caps: &[
                            (login_request, abi::rights::READ),
                            (login_result, abi::rights::WRITE | abi::rights::GRANT),
                            (verify, abi::rights::WRITE),
                            (g.fs_ep, abi::rights::WRITE | abi::rights::GRANT),
                            // **`WRITE` only, not `READ | WRITE`.** This process itself holds
                            // only `WRITE | GRANT` on the real file service's shared page (the
                            // kernel's own grant to the progenitor), and `WRITE` is all a writable mapping
                            // ever checks (`kernel::syscall::page_frame_map`'s own comment: "a
                            // read/write mapping needs WRITE on the frame"). See
                            // `components/src/login.rs`'s own comment on its matching delegation to its
                            // clients for the fuller account. Found here first: `must(build_child(...))`
                            // for `login` refused this exact `SEND_CAP` when it asked for `READ`
                            // too, in total silence, the same way every other capacity mismatch in
                            // this function has.
                            (login_page, abi::rights::WRITE | abi::rights::GRANT),
                            (login_ut, abi::rights::WRITE | abi::rights::GRANT),
                            (audit, abi::rights::WRITE),
                            (term_ep, abi::rights::WRITE | abi::rights::GRANT),
                        ],
                        maps: &[(LOGIN_CRED_VA, verify_page, abi::address_space::MAP_RW)],
                        // **What milestone 233 added, and it costs this process no slots at all.**
                        // `login` needs one program's bytes and a table to check them against; it
                        // used to read the whole archive, which this process cannot delegate
                        // (reserved RAM, no capability names it) and so never did. `blobs` copies
                        // through `supervision_protocol`'s `fill_and_map`, which holds one frame
                        // capability at a time and deletes it, so this adds nothing to the
                        // 21-of-24 peak `kernel::cap::CAPABILITY_TABLE_PEAK_MEASURED` records:
                        // `build_child` already reaches that same transient peak copying `login`'s
                        // own segments.
                        blobs: &[
                            (login_protocol::CARETAKER_ELF_VA, care_blob),
                            (login_protocol::PROGRAM_MEASUREMENTS_VA, table.as_bytes()),
                            (login_protocol::USER_TIMETABLE_KEEPER_ELF_VA, keeper_blob),
                            (login_protocol::TIMETABLE_ELF_VA, timetable_blob),
                        ],
                        stack_pages: LOGIN_STACK_PAGES,
                        ..ChildEndowment::new(Retention::Nothing)
                    },
                ));
                // **`login` hands the run-unvouched capability to each session it builds**
                // (DECISIONS §219 gate D2; calef's consequence: a user whose session was not given
                // it cannot run unvouched bytes). Placed after the build rather than in `caps`
                // above, because the build is this table's peak (`kernel::cap::
                // CAPABILITY_TABLE_PEAK_MEASURED`) and the endpoint is held from here on: retyped
                // now, it costs a slot the loader's transient address space and page have just
                // given back. `WRITE | GRANT`, because `login` must delegate it; each session gets
                // `WRITE` alone (`components/src/login.rs`).
                let d2 = *run_unvouched
                    .get_or_insert_with(|| must(retype_obj(ut, abi::objtype::RENDEZVOUS)));
                must_ok(place_at(
                    login_child.tcb,
                    d2,
                    abi::rights::WRITE | abi::rights::GRANT,
                    spawnproto::RUN_UNVOUCHED_SLOT,
                ));
                // The two blob lengths, in `x0` and `x1`, which is the rest of that contract: a
                // mapping with no length is a slice this process cannot bound.
                // **The durable window's page** (milestone 152, Fork 8 D): the channel a durable
                // session's timetable reads the store through, apart from window 0, which `login`
                // and every signed-in user's caretaker share. Placed after the build, like the
                // run-unvouched endpoint and for its reason: the build is this table's peak.
                // [`Windows`] never hands this window to a job.
                let durable_page =
                    must(window_page(g.fs_page, login_protocol::DURABLE_WINDOW).ok_or(()));
                must_ok(place_at(
                    login_child.tcb,
                    durable_page,
                    abi::rights::WRITE | abi::rights::GRANT,
                    login_protocol::DURABLE_WINDOW_SLOT,
                ));
                cap_delete(durable_page);
                must_ok(start_child(
                    login_child,
                    care_blob.len() as u64,
                    table.len() as u64,
                    login_protocol::schedule_lengths(
                        keeper_blob.len() as u64,
                        timetable_blob.len() as u64,
                    ),
                ));
                // login holds its own copies now; ours were only ever the means of wiring.
                cap_delete(login_page);
                cap_delete(login_request);
                cap_delete(login_result);
                cap_delete(verify);
                cap_delete(verify_page);
                cap_delete(login_ut);
                cap_delete(audit);
            }
        }
    }

    // **Give the construction budget away** (milestone 22, the interactive increment). Two bounded
    // carves and then the root itself: after this line the progenitor can spend at most `INIT_OWN_PAGES` on
    // itself and `JOBS_BUDGET_PAGES` on the prompt's jobs, and it can no longer reach the rest of the
    // memory the kernel handed it or delegate the root to anything it builds.
    //
    // Two budgets rather than one, and the split is load-bearing rather than tidy: the job pool's
    // watermark must move for **jobs only**, or the LIFO return-of-pages (§16) never fires. A scratch
    // page table carved out of the same region between a job's split and its reap would sit above
    // that job's run, so the reclaim would find it is not the top and give back nothing.
    // **The boot prompt holds the run-unvouched capability** (DECISIONS §219 gate D2), and this is
    // the one line in this file that decides it. calef ruled it 2026-09-26 (DECISIONS §221 (the
    // boot prompt is the owner's console), ruling 1): the boot prompt is the owner's root console,
    // and whoever holds it is the owner, as with single-user or recovery mode elsewhere. Deleting
    // this call makes the prompt refuse every unvouched image, as it did before D2 existed.
    // `WRITE` alone, so the shell can present it and can hand it to nothing it spawns.
    let run_unvouched =
        run_unvouched.unwrap_or_else(|| must(retype_obj(ut, abi::objtype::RENDEZVOUS)));
    must_ok(place_at(
        shell.tcb,
        run_unvouched,
        abi::rights::WRITE,
        spawnproto::RUN_UNVOUCHED_SLOT,
    ));

    let own_ut = must(memory_region_split(ut, INIT_OWN_PAGES));
    let jobs_ut = must(memory_region_split(ut, JOBS_BUDGET_PAGES));
    let images_ut = must(memory_region_split(ut, IMAGE_POOL_PAGES));
    // The shell's output page, in our own space, so we can say what just happened. Given up with
    // `UNMAP` after the last line, before the shell starts (`PageFrame::REVOKE` would have taken the
    // page from the shell too); see this module's BUGS.
    // SAFETY: `invoke` traps to the kernel, which validates the capability and the method before
    // acting (user_mode_runtime's contract).
    if unsafe { invoke(term_out, abi::page_frame::MAP, INIT_OUT_VA, 1, ut) } != 0 {
        fail()
    }
    cap_delete(ut);

    // And prove it from the inside, on the two primitives that build things, before anything else
    // runs. `NoSuchSlot` (-1) rather than `NotPermitted` (-3) is the whole claim: the capability is
    // *gone*, not narrowed, so there is nothing there to name. This is `root_supervisor`'s proof at
    // the interactive prompt, and `script/swish-check` reads the sentence.
    // SAFETY: as above: the kernel validates the capability and the method.
    let frame = unsafe { invoke(ut, abi::memory_region::RETYPE, 0, 0, 0) };
    // SAFETY: as above: the kernel validates the capability and the method.
    let object = unsafe {
        invoke(
            ut,
            abi::memory_region::RETYPE_OBJ,
            abi::objtype::THREAD_CONTROL_BLOCK,
            0,
            0,
        )
    };
    announce(
        term_ep,
        if frame == -1 && object == -1 {
            b"progenitor: construction budget dropped; retype answers NoSuchSlot\n"
        } else {
            b"progenitor: construction budget NOT dropped; it can still build\n"
        },
    );
    // **And what the measurement decided** (milestone 104), on the same terms as the line above: a
    // claim about what the progenitor refuses is worth what the check behind it is worth, and only the progenitor can
    // run that check. The affirmative line is the load-bearing one. A measured boot's natural bug is
    // for the check to evaporate when the build step does not run, and a boot that says nothing
    // looks exactly like a boot that measured everything, so `script/swish-check` reads this
    // sentence and a boot that stopped measuring fails the gate instead of passing quietly.
    let mut buf = [0u8; SENTENCE];
    announce(
        term_ep,
        if refused_n == 0 {
            b"progenitor: every program measured against the archive table\n"
        } else {
            sentence(
                &mut buf,
                b"progenitor: measurement refused",
                &refused[..refused_n],
                b"; they cannot be spawned\n",
            )
        },
    );
    // **How directory grants will be delivered on this boot** (milestone 606, rulings D and T1),
    // said once, so `script/swish-check` can hold the boot to it: a server whose own measured bytes
    // carry the subtree-grants note gets bound badges, and every other boot keeps caretakers.
    if with_fs {
        announce(
            term_ep,
            if fs_scoped {
                b"progenitor: the file server enforces subtree grants itself; directory grants get a bound badge\n"
            } else {
                b"progenitor: directory grants get a caretaker\n"
            },
        );
    }
    // **The entropy service's own outcome** (DECISIONS §120's 2026-08-26 amendment), said here
    // rather than where it was decided: `entropy_ready` was set long before this process had a
    // terminal at all (the whole reason the build happens first, at the top of this function; see
    // that block's own comment), and [`INIT_OUT_VA`] is not mapped into the progenitor's own space until the
    // line above this one first used it, so nothing earlier in this function could `announce`
    // regardless. Silent when there was nothing to build (no device, or no `entropy` program in
    // the archive), the same posture the sink adapter and the subtree caretaker already take for a
    // missing component.
    if entropy_ready {
        announce(
            term_ep,
            if entropy_from_instruction {
                &b"progenitor: entropy service up; the kernel built it on the CPU's seed instruction\n"[..]
            } else {
                &b"progenitor: entropy service up; drew real bytes from a virtio-rng device\n"[..]
            },
        );
    }
    // **The network stack's outcome** (milestone 590 (provisional)), said here for
    // `entropy_ready`'s
    // reason. The address is the lease `net_stack` reported, so the line is evidence rather than
    // an intention: DHCP completed through the confined NIC before the prompt existed.
    if let Some((_, lease)) = network {
        let mut line = [0u8; 80];
        let head = b"progenitor: network stack up; DHCP leased ";
        line[..head.len()].copy_from_slice(head);
        let mut n = head.len();
        n += render_ipv4(lease as u32, &mut line[n..]);
        line[n] = b'\n';
        announce(term_ep, &line[..n + 1]);
    }
    // **The generated login credential** (milestone 49's boot-wiring update), said here for
    // [`entropy_ready`]'s own reason: `login_ready` was decided long before this process had a
    // terminal. Printed exactly once, before the prompt, the shape a cloud image's generated
    // first-boot password already takes: this process's own copy was wiped the moment
    // `identity_provisioner` used it (see that block's own comment), so this line is the only
    // place the password exists once it scrolls past. Silent when there is no login stack at all
    // (no entropy, no filesystem, or one of the four programs missing/unvouched), the same posture
    // every other optional boot component already takes.
    //
    // **It used to say `init: login ready` and that was the false half of milestone 233.** The
    // measurement behind this line is `identity_provisioner` answering `IDP_RPT_OK`: a credential
    // store exists and it holds this identity and this secret. That is worth reporting and is kept
    // exactly as it was. What it is not is a statement that the `login` process is alive, and for
    // an unknown number of weeks it was not: this process started it with no way to reach the
    // archive it parsed at `_start`, so it faulted before serving anything while this line went on
    // announcing a service. A reader has no way to tell a sentence's evidence from its wording, so
    // the wording now names the evidence.
    //
    // **Nothing here can honestly claim more, and the claim moved rather than being dropped.**
    // Knowing `login` survived means `login` saying so, which is a message, an endpoint and a wait
    // this process does not have, and a boot that blocks on a child's readiness is a boot that
    // hangs when the child is the thing that is broken. The survival claim lives in
    // `script/swish-check` instead, which fails the boot if the kernel reported killing any user
    // thread; see `swish_check_leg`'s own `no user thread was killed` check.
    if login_ready {
        fn push(buf: &mut [u8; SENTENCE], n: &mut usize, src: &[u8]) {
            for &b in src {
                if *n < SENTENCE {
                    buf[*n] = b;
                    *n += 1;
                }
            }
        }
        let mut buf = [0u8; SENTENCE];
        let mut n = 0usize;
        push(
            &mut buf,
            &mut n,
            b"progenitor: login credentials provisioned -- identity '",
        );
        push(&mut buf, &mut n, DEMO_IDENTITY);
        push(&mut buf, &mut n, b"' password '");
        push(&mut buf, &mut n, &login_password);
        push(&mut buf, &mut n, b"' (shown once; use it now)\n");
        announce(term_ep, &buf[..n]);
    }
    // **`term_ep` stays past this point now, and `term_out` does not** (milestone 632
    // (provisional)). A `graphical_terminal` session launched with no virtio keyboard takes its keystrokes
    // from the boot's own line discipline, in raw mode, and the only process that can grant a
    // session that endpoint is this one: the shell's copy carries no `GRANT` (deliberately, so
    // nothing at the prompt can hand the terminal, which also carries `OPERATION_READLINE`, to any
    // program it likes; that widening is exactly what milestone 50's sink adapter exists to
    // avoid), and nobody else holds the discipline's endpoint at all. Keeping it is the file
    // service pair's precedent: the spawn service holds exactly the authority a delivery needs,
    // `WRITE | GRANT` on an endpoint it never receives on. One resting slot, which the table has
    // (the peak is inside the login block above, and this capability was live through it anyway).
    cap_delete(term_out);
    // And the window onto the shell's output page, which every line above was staged through and
    // nothing below writes (milestone 95 (an unmap primitive), §249 (a running address space stays
    // nameable)): given up before the shell starts, so this process cannot read what the shell
    // prints or write into it.
    supervision_protocol::give_up_own_page(INIT_OUT_VA);

    // 6. The undertaker, out of what is left of our own budget. Three capabilities and nothing
    // else: `READ` on the supervision endpoint, so it can free a job's memory and can never spend
    // it; `WRITE` on the result endpoint, so it can say the one word a job the kernel killed cannot
    // say for itself (milestone 235 (a command that faults hangs the prompt),
    // `grant_plan::spawnproto::JOB_FAULTED`); and `WRITE` on a copy
    // of the spawn endpoint badged `spawnproto::UNDERTAKER_BADGE`, so it can say which job it
    // reaped (milestone 685 (a job is finished when its memory is back), `spawnproto::reaped`).
    // `WRITE` without `GRANT` on both: the right to speak on those channels, not to hand them on.
    //
    // Slot order is the contract with the program, which names them `DEATHS` (0), `REPORT` (1) and
    // `SPAWN` (2). The badged copy is a fresh slot of ours, deleted once the build has copied it.
    let reaper_spawn = must(badged(spawn_ep, spawnproto::UNDERTAKER_BADGE));
    let reaper = must(build_child(
        own_ut,
        own_ut,
        &reaper_elf,
        &ChildEndowment {
            caps: &[
                (deaths, abi::rights::READ),
                (result_ep, abi::rights::WRITE),
                (reaper_spawn, abi::rights::WRITE),
            ],
            stack_pages: CHILD_STACK_PAGES,
            ..ChildEndowment::new(Retention::Nothing)
        },
    ));
    cap_delete(reaper_spawn);
    must_ok(start_child(reaper, 0, 0, 0));

    // Role 0 (the prompt), and `arg1` is the rights its directory capability carries. A shell told 0
    // holds no directory and says so at every verb that would need one. `arg2` is the clock slot
    // (milestone 86), which moves with whether this boot had a disk, so it is told rather than
    // assumed.
    must_ok(start_child(shell, 0, fs_rights, sh_clock_slot));

    spawn_service(
        Channels {
            spawn_ep,
            result_ep,
            deaths,
            own_ut,
            jobs_ut,
            images_ut,
            clock_page: g.clock_page,
            config_page: g.config_page,
            // The terminal's sink, if this initrd carried an adapter to serve it. This is what a
            // declared second stream gets by default (DECISIONS §67): the shell names a file with
            // `2>` and otherwise the bytes go straight to the screen, through a process that can do
            // nothing else with them.
            term_sink: sink_elf.is_some().then_some(term_sink),
            fs,
            // **The client view of the entropy service, if this boot has one** (milestone 111).
            // The same endpoint the login stack drew its password from, kept rather than dropped,
            // so a program a *person* types can be endowed randomness the same way `date` is
            // endowed a clock. `None` here is not a boot failure: it is a `uuid` that says on its
            // second stream that it holds no entropy capability, which is the true sentence.
            entropy: entropy_client,
            // **The stack's client endpoint, if this boot built one** (milestone 590
            // (provisional)), `entropy`'s shape one service over.
            network: network.map(|(stack, _)| stack),
            // **The reboot object** (milestone 805), kept for the one program that declares it.
            reboot: is_granted(g.reboot).then_some(g.reboot),
            catalogue,
            run_unvouched,
            // **The boot line discipline's endpoint, for a `graphical_terminal` session's UART arm** (milestone
            // 632 (provisional)); see the comment where `term_out` alone is deleted for why this
            // stays ours. Unused, and unheld, on every boot that builds no session.
            boot_terminal: term_ep,
            // **The `graphical_terminal` session's measured images and the boot's verdict on the devices**
            // (milestone 632 (provisional)). `held` is false, and the caps are long deleted, on
            // every boot with no gpu; a `graphical_terminal` request on such a boot is refused below.
            screen: graphical_terminal_caps,
        },
        &progs,
        Caretakers {
            subtree: care_elf,
            nameset: set_elf,
        },
        // The session's discipline is built from the same supervisor and the same bytes the boot's
        // own was (`ts_elf`/`td_bytes` above), so a session's line editor is the boot's line editor,
        // measured once and built twice.
        SessionDiscipline {
            supervisor: ts_elf,
            discipline: td_bytes,
        },
        fs_scoped,
    )
}

/// A child's capability grants and page mappings, the two slices a `ChildEndowment` takes as `caps`
/// and `maps`. Named so the `x86_64`-vs-others split of the console and input endowments (a port
/// capability held rather than a page mapped, milestone 299) is a one-line `let` per branch without
/// clippy's type-complexity lint firing on the bare tuple. `'a` ties the slices to the `let`'s block.
type EndowmentSlices<'a> = (&'a [(u64, u64)], &'a [(u64, u64, u64)]);
