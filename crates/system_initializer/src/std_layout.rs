//! `std_layout`: where a `std` child's endowment goes in its table and its space. Moved out of
//! `lib.rs` unchanged but for visibility and one link by milestone 812 (`std::thread::spawn` runs
//! real threads in one address space), whose three new slots grew it past §266 (a Rust source file
//! stays under 2,000 lines)'s baseline.
//!
//! Name: provisional (milestone 812's lane, 2026-10-10 UTC).

/// **A `std` child's endowment, placed where nife's `std` reads it** (milestone 595 (provisional)).
///
/// The native spawn fills a child's table from slot 0, output first; nife's `std` fixes a slot per
/// authority (`crates/std_runtime_protocol`), and a native child's output slot is a `std` child's
/// heap budget, so a `std` program built natively dies on its first `println!`. This is the other
/// shape, from the decisions [`spawn_service`](super::spawn_service) already made.
///
/// - slot 0: the job's own region, `SPEND_ONLY`, as the heap's budget. See
///   [`grant_plan::STD_REGION_PAGES`] for why it is the same region the child is built from.
/// - slot 1: the output, exactly the capability a native child would get at slot 0.
/// - slot 4 and a page at `FS_PAGE`: the caretaker's narrowed endpoint, if the line granted a
///   directory. The same frame the caretaker and the file server map, at the `std` address.
/// - slot 5 and a read-only page at `CLOCK_PAGE`, slot 7 and one at `CONFIG_PAGE`: the manifest's
///   clock and configuration pages.
/// - slot 6: the entropy service, `WRITE`, if the manifest declared it and this boot built one.
/// - slot 8 and a read-only page at `ARGS_PAGE`: the line's argv (milestone 205, DECISIONS §170),
///   if the shell sent one. The page is the child's own, copied out of the shell's frame.
/// - slots 2 and 3: the network stack, `WRITE`, and the job's region again, `ALL`, from which the net
///   PAL lends socket pages (milestone 809 (the package client becomes a program)) since milestone 824.
/// - slots 15, 16 and 17: the installer endpoint, its reply endpoint and the catalog page, mapped at
///   `CATALOG_PAGE` (milestone 809; `grant_plan::INSTALLER_SLOT` has the rights).
/// - slot 9: the name resolver, `WRITE`, badged with `Prog::resolver_zone`'s zone, for a job holding
///   the network (milestone 892 (a std program at the prompt holds the network and a resolver)).
///
/// Name: provisional.
///
/// # BUGS
///
/// - **The child holds `WRITE` on the region it is built in**, because that region is its heap
///   (`grant_plan::STD_REGION_PAGES` says why one region). `WRITE` on a region is also `SPLIT`,
///   and a region that has been split cannot be destroyed until its children are, so a program
///   that splits its own heap pins its job region: `job_undertaker` never reclaims it, and the
///   pool is one region smaller until reboot. nife's `std` never splits (its allocator only
///   `MAP`s), so this takes a program written to do it. Closing it needs a right that allows `MAP`
///   and not `SPLIT`, which is the syscall surface and an architect's call.
/// - **The directory half is built and never exercised at the prompt.** No `std` manifest declares
///   a directory yet. DECISIONS §170 (how a foreign program is told what to do) ruled that the
///   directories granted on a line bound what a word reaches, and granting one to a `std` program
///   from the prompt is milestone 205's designation half, not built yet. The kernel harness proves
///   the same slot and page from its side (`fs_service::start_std_full`).
pub(super) struct StdLayout {
    pub(super) caps: [(u64, u64); 2],
    placed: [(u64, u64, u64); 14],
    placed_n: usize,
    maps: [(u64, u64, u64); 5],
    maps_n: usize,
}

/// **What a `std` child is granted beyond its heap and its output**, each `None` when the manifest
/// did not ask or the boot has none: the pieces [`StdLayout::new`] places, named so the call site
/// says which is which.
pub(super) struct StdGrants {
    /// The directory grant's endpoint and the page it shares.
    pub(super) dir: Option<(u64, u64)>,
    pub(super) clock: Option<u64>,
    pub(super) config: Option<u64>,
    pub(super) entropy: Option<u64>,
    /// The child's own copy of the argv page.
    pub(super) args: Option<u64>,
    /// The network stack's endpoint (milestone 809).
    pub(super) network: Option<u64>,
    /// The installer endpoint and its reply endpoint (milestone 809, §270).
    pub(super) installer: Option<(u64, u64)>,
    /// The child's own copy of the image's catalog (milestone 809).
    pub(super) catalog: Option<u64>,
    /// The name resolver's endpoint, badged with the zone this job was granted (milestone 892).
    pub(super) resolver: Option<u64>,
}

// `caps` lands in order from slot 0, so the two in-order slots must be 0 and 1. Checked here rather
// than assumed, because the contract crate could renumber them and this would build a child whose
// heap and output were swapped.
const _: () = assert!(
    std_runtime_protocol::MEMORY_REGION_SLOT == 0 && std_runtime_protocol::STDOUT_SLOT == 1
);

impl StdLayout {
    pub(super) fn new(region: u64, out: (u64, u64), g: StdGrants) -> Self {
        use std_runtime_protocol as rt;
        let StdGrants {
            dir,
            clock,
            config,
            entropy,
            args,
            network,
            installer,
            catalog,
            resolver,
        } = g;
        // The heap's budget is spend-only, unless the child holds the installer: `jig` stages a
        // package in pages retyped from its heap and lends them to the progenitor, and since
        // milestone 824 (a retype mints no right its budget lacks) a page carries its budget's
        // rights, so the lending `GRANT` has to be on the budget.
        let heap = if installer.is_some() {
            abi::rights::ALL
        } else {
            abi::rights::SPEND_ONLY
        };
        let mut l = StdLayout {
            caps: [(region, heap), out],
            placed: [(0, 0, 0); 14],
            placed_n: 0,
            maps: [(0, 0, 0); 5],
            maps_n: 0,
        };
        let place = |l: &mut StdLayout, slot: u64, cap: u64, rights: u64| {
            l.placed[l.placed_n] = (slot, cap, rights);
            l.placed_n += 1;
        };
        let map = |l: &mut StdLayout, va: u64, cap: u64, mode: u64| {
            l.maps[l.maps_n] = (va, cap, mode);
            l.maps_n += 1;
        };
        if let Some((ep, page)) = dir {
            place(&mut l, rt::FS_DIR_SLOT, ep, abi::rights::WRITE);
            map(&mut l, rt::FS_PAGE, page, abi::address_space::MAP_RW);
        }
        if let Some(page) = clock {
            place(&mut l, rt::CLOCK_SLOT, page, abi::rights::READ);
            map(&mut l, rt::CLOCK_PAGE, page, abi::address_space::MAP_RO);
        }
        if let Some(ep) = entropy {
            place(&mut l, rt::ENTROPY_SLOT, ep, abi::rights::WRITE);
        }
        if let Some(page) = config {
            place(&mut l, rt::CONFIG_SLOT, page, abi::rights::READ);
            map(&mut l, rt::CONFIG_PAGE, page, abi::address_space::MAP_RO);
        }
        if let Some(page) = args {
            place(&mut l, rt::ARGS_SLOT, page, abi::rights::READ);
            map(&mut l, rt::ARGS_PAGE, page, abi::address_space::MAP_RO);
        }
        if let Some(stack) = network {
            place(&mut l, rt::STACK_SLOT, stack, abi::rights::WRITE);
            // Lent pages carry this budget's rights, milestone 824 (a retype mints no right its
            // budget lacks), so `GRANT` is delegated here rather than minted with a page.
            place(&mut l, rt::NET_MEMORY_REGION_SLOT, region, abi::rights::ALL);
        }
        if let Some((endpoint, reply)) = installer {
            place(
                &mut l,
                grant_plan::INSTALLER_SLOT,
                endpoint,
                abi::rights::WRITE,
            );
            place(
                &mut l,
                grant_plan::INSTALLER_REPLY_SLOT,
                reply,
                abi::rights::READ,
            );
        }
        if let Some(endpoint) = resolver {
            place(&mut l, rt::RESOLVER_SLOT, endpoint, abi::rights::WRITE);
        }
        if let Some(page) = catalog {
            place(&mut l, grant_plan::CATALOG_SLOT, page, abi::rights::READ);
            map(&mut l, rt::CATALOG_PAGE, page, abi::address_space::MAP_RO);
        }
        // **Threads** (milestone 812 (`std::thread::spawn` runs real threads in one address
        // space)): every `std` program is built as a process of its own and may add threads to it,
        // within its own region's budget. Its own process with `BIND` and nothing more, its own
        // first thread to set its thread pointer through, and its own space with `READ` for the
        // futex waits its locks make. None of the three reaches anything outside the program.
        place(
            &mut l,
            rt::PROCESS_SLOT,
            supervision_protocol::CHILDS_OWN_PROCESS,
            abi::rights::BIND,
        );
        place(
            &mut l,
            rt::THREAD_SLOT,
            supervision_protocol::CHILDS_OWN_THREAD,
            abi::rights::WRITE,
        );
        place(
            &mut l,
            rt::SPACE_SLOT,
            supervision_protocol::CHILDS_OWN_SPACE,
            abi::rights::READ,
        );
        l
    }

    pub(super) fn placed(&self) -> &[(u64, u64, u64)] {
        &self.placed[..self.placed_n]
    }

    pub(super) fn maps(&self) -> &[(u64, u64, u64)] {
        &self.maps[..self.maps_n]
    }
}
