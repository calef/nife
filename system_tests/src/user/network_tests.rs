//! `network_tests`: the network path: the virtio-net drivers, DHCP, the socket contract over
//! UDP and TCP, name resolution, a package fetched over HTTP, and `std::net`.
//!
//! Name: provisional, minted 2026-10-12 (UTC) by the lane for milestone 873 (the catch-all
//! userspace test file is split by topic).

#[cfg(target_arch = "aarch64")]
use super::std_tests::assert_std_transcript_from;
#[cfg(target_arch = "aarch64")]
use super::test_programs::{blk_image, std_exerciser_image};
#[cfg(target_arch = "aarch64")]
use super::*;
#[cfg(target_arch = "aarch64")]
use crate::sched;

/// The `net_stack` program's ELF bytes (milestone 30, piece 3): the smoltcp net server, a distinct
/// binary loaded by name. Used only by the net tests, whose RISC-V twins live in
/// `riscv_virtio_tests`.
#[cfg(target_arch = "aarch64")]
fn net_stack_image() -> &'static [u8] {
    program("net_stack").expect("no net_stack program in the initial ramdisk")
}

/// The net client's test selectors and its success word, matching `components/src/socket_test_client.rs`. The
/// client is a nonzero entry role of the `net_stack` binary, so it needs no image of its own.
#[cfg(target_arch = "aarch64")]
const NET_TEST_NAME_RESOLUTION: u64 = 1;
#[cfg(target_arch = "aarch64")]
const NET_TEST_TCP_ECHO: u64 = 2;
#[cfg(target_arch = "aarch64")]
const NET_TEST_TCP_REOPEN: u64 = 3;
#[cfg(target_arch = "aarch64")]
const NET_TEST_UDP_TFTP: u64 = 4;
#[cfg(target_arch = "aarch64")]
const NET_TEST_TCP_ACCEPT: u64 = 5;
#[cfg(target_arch = "aarch64")]
const NET_TEST_HTTP_PACKAGE: u64 = 6;
#[cfg(target_arch = "aarch64")]
const NET_TEST_TCP_ACCEPT_UNHEARD: u64 = 12;
/// `TEST_HTTP_PACKAGE`'s refusal word, reported for a package whose digest disagrees.
#[cfg(target_arch = "aarch64")]
const NET_CLIENT_DIGEST_REFUSED: u64 = 3;
/// The one port the inbound gate is granted (milestone 107). The runners forward a host port to
/// exactly this one, and the client asks for `fixture::DENIED_PORT` as well to prove the grant
/// refuses. Named here because the *spawn service* is what grants it, which is the point.
///
/// Taken from `socket_contract::fixture` rather than spelled again (milestone 64): three binaries and
/// this test have to agree on the number, and a literal repeated per call site is the shape rule 7
/// exists to retire. The runners' `hostfwd` spells it a second time because a shell script cannot
/// read a Rust crate, and that drift is loud (the prober reports the guest served none).
#[cfg(target_arch = "aarch64")]
const NET_LISTEN_PORT: u16 = socket_contract::fixture::LISTEN_PORT;
/// The fixed UDP port range the accept test's stack is granted (milestone 55's UDP bind grant).
/// Named here for the same reason as the listen port: the spawn service grants it, and a program
/// cannot ask for what it was not given. `socket_test_client` binds 5354 inside it to prove a
/// granted port binds and is exclusive, and asks for 4444 outside it to prove the refusal.
///
/// The range starts at 5353 because the multicast DNS responder held that port beside the socket
/// client until milestone 298 retired it on 2026-09-15 (notes/mdns.md). The numbers carry no
/// meaning of their own now; they are kept so the client and this grant still agree.
#[cfg(target_arch = "aarch64")]
const NET_UDP_GRANT_BOTTOM: u16 = 5353;
#[cfg(target_arch = "aarch64")]
const NET_UDP_GRANT_TOP: u16 = 5354;
#[cfg(target_arch = "aarch64")]
const NET_CLIENT_OK: u64 = 1;

/// The client could not complete for an ENVIRONMENTAL reason (the host resolver never answered),
/// not because of a defect here. Only the non-gating real-DNS half of the name test can report it.
#[cfg(target_arch = "aarch64")]
const NET_CLIENT_NO_ANSWER: u64 = 2;

/// **A userspace driver completes a DHCP round trip over virtio-net.** Milestone 30, end to
/// end, and the proof the multi-queue confinement carries a real NIC.
///
/// The kernel enumerates the NIC and hands a driver at EL0 a confined `Virtio` capability, a DMA
/// page, and an interrupt. The driver brings up BOTH virtqueues (receive = 0, transmit = 1),
/// posts a receive buffer, transmits a hand-built DHCP DISCOVER, and waits for QEMU user-mode
/// networking's OFFER. It reports the offered address, which must land in slirp's 10.0.2.0/24.
/// Because a valid OFFER for our transaction is the only path to that report, a match proves the
/// DISCOVER left (TX) and the OFFER returned (RX), across both queues and both directions of the
/// confinement, with no TCP/IP stack in the loop.
// RISC-V twin: `riscv_virtio_tests::a_userspace_driver_completes_a_dhcp_round_trip_over_virtio_net`. Gated here rather than run twice: that
// module drives the same property on the other instruction set, through the same `block_driver` and
// `net_stack` binaries this leg now uses, and a second copy would double the suite's slowest tests
// to prove nothing new. (It read "through hello's roles" until milestone 291, when aarch64 stopped
// having any: the two legs differ by ISA now and by nothing else.) See this module's comment on
// the two kinds of gate.
#[cfg(target_arch = "aarch64")]
#[test_case]
fn a_userspace_driver_completes_a_dhcp_round_trip_over_virtio_net() {
    let Some(report) = virtio_service::start_net(blk_image()) else {
        // No NIC on this run (a bare boot). The test runners always attach one, so this
        // branch is not the parity gate. See helpers/qemu-runner-*.sh (NIFE_NET).
        crate::testing::skip!("no virtio-net device attached");
    };

    let yiaddr = sched::ipc_receive(report)[0] as u32;
    assert_eq!(
        yiaddr & 0xffff_ff00,
        0x0A00_0200,
        "the DHCP OFFER's yiaddr {yiaddr:#010x} is not in QEMU slirp's 10.0.2.0/24: the round \
         trip did not complete correctly",
    );
    // We do NOT assert a fresh routed interrupt here, unlike the disk read test. The net
    // driver's completion is the used ring advancing, not one interrupt per operation (the same
    // discipline the disk driver's complete loop follows, notes/dma.md), and the net test suite
    // shares one NIC across many drivers and servers (piece 3): a leftover completion from a
    // prior operator can be counted before this test's baseline and then consumed as a stale
    // wakeup, so a strict interrupt-delta is unreliable. The OFFER round trip above is the proof
    // that the interrupt path carried the completion.
}

/// The same DHCP round trip over the PCIe transport, behind the IOMMU (milestone 30, §20): the
/// NIC is confined in hardware to its DMA region, and the driver binary is byte-identical to the
/// mmio one. Proves the multi-queue confinement and the net driver work over the bus real
/// hardware uses.
// RISC-V twin: `riscv_virtio_tests::a_userspace_driver_completes_a_dhcp_round_trip_over_virtio_net_pci`. Gated here rather than run twice: that
// module drives the same property on the other instruction set, through the same `block_driver` and
// `net_stack` binaries this leg now uses, and a second copy would double the suite's slowest tests
// to prove nothing new. (It read "through hello's roles" until milestone 291, when aarch64 stopped
// having any: the two legs differ by ISA now and by nothing else.) See this module's comment on
// the two kinds of gate.
#[cfg(target_arch = "aarch64")]
#[test_case]
fn a_userspace_driver_completes_a_dhcp_round_trip_over_virtio_net_pci() {
    let Some(report) = virtio_service::start_net_pci(blk_image()) else {
        crate::testing::skip!("no virtio-net-pci device attached");
    };

    let yiaddr = sched::ipc_receive(report)[0] as u32;
    assert_eq!(
        yiaddr & 0xffff_ff00,
        0x0A00_0200,
        "the DHCP OFFER's yiaddr {yiaddr:#010x} over PCIe is not in QEMU slirp's 10.0.2.0/24",
    );
}

/// **The net server: smoltcp running DHCP over the confined NIC** (milestone 30, piece 3). The
/// integration proof and the thesis headline for networking: a real, reused TCP/IP stack
/// (smoltcp, not hand-built) runs entirely at EL0, brings the NIC up through the `Virtio`
/// capability, and completes a DHCP handshake against QEMU user-mode networking. The kernel
/// knows nothing about DHCP; it owns only the DMA confinement. The server reports the acquired
/// address, which must land in slirp's 10.0.2.0/24, so only a real DHCP round trip driven by
/// smoltcp over the confined NIC can produce it.
// RISC-V twin: `riscv_virtio_tests::the_net_server_acquires_a_dhcp_lease_over_smoltcp`. Gated here rather than run twice: that
// module drives the same property on the other instruction set, through the same `block_driver` and
// `net_stack` binaries this leg now uses, and a second copy would double the suite's slowest tests
// to prove nothing new. (It read "through hello's roles" until milestone 291, when aarch64 stopped
// having any: the two legs differ by ISA now and by nothing else.) See this module's comment on
// the two kinds of gate.
#[cfg(target_arch = "aarch64")]
#[test_case]
fn the_net_server_acquires_a_dhcp_lease_over_smoltcp() {
    let Some((report, net)) = virtio_service::start_net_server(net_stack_image()) else {
        crate::testing::skip!("no virtio-net device attached");
    };
    let [addr, nameserver, ..] = sched::ipc_receive(report);
    let addr = addr as u32;
    assert_eq!(
        addr & 0xffff_ff00,
        0x0A00_0200,
        "smoltcp's DHCP lease {addr:#010x} is not in QEMU slirp's 10.0.2.0/24",
    );
    // slirp's DHCP names 10.0.2.3 as the DNS server, and the lease report carries it in its second
    // word (`socket_contract::lease`, provisional) for a resolver's spawner (§248 (the name
    // resolver is its own confined program)).
    assert_eq!(
        socket_contract::lease::word_ipv4(nameserver),
        Some([10, 0, 2, 3]),
        "the lease report did not carry slirp's DNS server (word {nameserver:#x})",
    );
    net.release_or_fail("a net test's net_stack");
}

/// The net server over the PCIe transport, behind the IOMMU (milestone 30, §20): smoltcp drives
/// a NIC confined in hardware and still gets its lease.
// RISC-V twin: `riscv_virtio_tests::the_net_server_acquires_a_dhcp_lease_over_smoltcp_pci`. Gated here rather than run twice: that
// module drives the same property on the other instruction set, through the same `block_driver` and
// `net_stack` binaries this leg now uses, and a second copy would double the suite's slowest tests
// to prove nothing new. (It read "through hello's roles" until milestone 291, when aarch64 stopped
// having any: the two legs differ by ISA now and by nothing else.) See this module's comment on
// the two kinds of gate.
#[cfg(target_arch = "aarch64")]
#[test_case]
fn the_net_server_acquires_a_dhcp_lease_over_smoltcp_pci() {
    let Some((report, net)) = virtio_service::start_net_server_pci(net_stack_image()) else {
        crate::testing::skip!("no virtio-net-pci device attached");
    };
    let addr = sched::ipc_receive(report)[0] as u32;
    assert_eq!(
        addr & 0xffff_ff00,
        0x0A00_0200,
        "smoltcp's DHCP lease {addr:#010x} over PCIe is not in QEMU slirp's 10.0.2.0/24",
    );
    net.release_or_fail("a net test's net_stack");
}

/// **The socket contract, UDP end to end** (milestone 30, piece 3 phase B; DECISIONS §25). A
/// client process holds a `Stack` rendezvous and its own untyped, mints a shared frame, delegates
/// it, opens a UDP socket by id, sends a datagram, and reads the reply back through the same
/// frame. No ambient network: the client acts only through the capability it was granted, and the
/// bytes cross in the shared frame, never in a message. Proves the whole path, client to `net_stack` to
/// smoltcp to the confined NIC, over the mmio transport.
///
/// The peer is **slirp's own TFTP server** (10.0.2.2:69), served inside libslirp, so the exchange
/// is deterministic and never leaves the emulator. This test used to query 10.0.2.3:53, which
/// reads like a local resolver but is not one: libslirp NATs that address to the *host's*
/// nameserver, so the gate depended on the developer's DNS answering at that instant and flaked
/// (~2.5% per query, measured). The real-resolution case still runs, non-gating, below.
// RISC-V twin: `riscv_virtio_tests::a_client_completes_a_udp_round_trip_through_the_socket_contract`. Gated here rather than run twice: that
// module drives the same property on the other instruction set, through the same `block_driver` and
// `net_stack` binaries this leg now uses, and a second copy would double the suite's slowest tests
// to prove nothing new. (It read "through hello's roles" until milestone 291, when aarch64 stopped
// having any: the two legs differ by ISA now and by nothing else.) See this module's comment on
// the two kinds of gate.
#[cfg(target_arch = "aarch64")]
#[test_case]
fn a_client_completes_a_udp_round_trip_through_the_socket_contract() {
    let Some((report, net)) = virtio_service::start_net_stack(
        net_stack_image(),
        NET_TEST_UDP_TFTP,
        false,
        socket_contract::NO_LISTEN_GRANT,
    ) else {
        crate::testing::skip!("no virtio-net device attached");
    };
    let verdict = sched::ipc_receive(report)[0];
    assert_eq!(
        verdict, NET_CLIENT_OK,
        "the UDP round trip against slirp's TFTP server failed (client code {verdict:#x})",
    );
    net.release_or_fail("a net test's net_stack");
}

/// The same UDP round trip over the PCIe transport, behind the IOMMU.
// RISC-V twin: `riscv_virtio_tests::a_client_completes_a_udp_round_trip_through_the_socket_contract_pci`. Gated here rather than run twice: that
// module drives the same property on the other instruction set, through the same `block_driver` and
// `net_stack` binaries this leg now uses, and a second copy would double the suite's slowest tests
// to prove nothing new. (It read "through hello's roles" until milestone 291, when aarch64 stopped
// having any: the two legs differ by ISA now and by nothing else.) See this module's comment on
// the two kinds of gate.
#[cfg(target_arch = "aarch64")]
#[test_case]
fn a_client_completes_a_udp_round_trip_through_the_socket_contract_pci() {
    let Some((report, net)) = virtio_service::start_net_stack(
        net_stack_image(),
        NET_TEST_UDP_TFTP,
        true,
        socket_contract::NO_LISTEN_GRANT,
    ) else {
        crate::testing::skip!("no virtio-net-pci device attached");
    };
    let verdict = sched::ipc_receive(report)[0];
    assert_eq!(
        verdict, NET_CLIENT_OK,
        "the UDP round trip over PCIe failed (client code {verdict:#x})",
    );
    net.release_or_fail("a net test's net_stack");
}

/// **An accept nobody answers ends at `net_stack`'s bound** (the riscv-cpu-net-flake lane,
/// 2026-10-13 UTC). One `ACCEPT` on the PCI NIC, which no `hostfwd` reaches, so no SYN ever
/// arrives and smoltcp holds no timer. The stack's wait has to end by its own 15 s bound and refuse.
/// Before `nic_wait` took the caller's deadline it blocked for a frame forever: the listener
/// retries built on that bound never ran, and on 2026-10-12 a prober connection held to nobody
/// turned the CPU matrix's std listener test into a 60 s watchdog. Without the fix this test is
/// that watchdog. x86_64 has no twin: its NIC is the polled `e1000e`, whose wait is a 1 ms sleep,
/// so the bound held there all along.
///
/// Falsification: replayable `system_tests/falsifications/user.network_tests.an_accept_nobody_answers_ends_at_the_stacks_bound.patch`
#[cfg(target_arch = "aarch64")]
#[test_case]
fn an_accept_nobody_answers_ends_at_the_stacks_bound() {
    let started = crate::arch::timer::now();
    let Some((report, net)) = virtio_service::start_net_stack(
        net_stack_image(),
        NET_TEST_TCP_ACCEPT_UNHEARD,
        true,
        socket_contract::listen_grant(NET_LISTEN_PORT, NET_LISTEN_PORT),
    ) else {
        crate::testing::skip!("no virtio-net-pci device attached");
    };
    let verdict = sched::ipc_receive(report)[0];
    let waited_ms = (crate::arch::timer::now() - started) * 1000 / crate::arch::timer::frequency();
    assert_eq!(
        verdict, NET_CLIENT_OK,
        "an accept nobody could reach did not come back refused (client code {verdict:#x})",
    );
    assert!(
        waited_ms >= 15_000,
        "the accept was refused after {waited_ms} ms, before net_stack's 15 s bound",
    );
    net.release_or_fail("a net test's net_stack");
}

/// **A host name resolves through the socket contract, and the lies do not** (milestone 384 (in a
/// capability system the resolver is a grant)). Two halves in one client, because a second
/// `net_stack` would need a virtio slot past `MAX_DEVICES`.
///
/// **The first half gates.** The client asks the runners' name server peer
/// (`helpers/name-server-peer`, a `guestfwd` at 10.0.2.9:53, DNS over TCP) about six names through
/// `domain_name_system`. Two resolve, one through a compressed CNAME, and four must be refused,
/// each by its own check: a name that does not exist, the right answer under the wrong transaction
/// id, an address for a name nobody asked about, and a compression pointer to itself. Then it
/// connects to the address it resolved and gets an echo back. Nothing else in the tree turns a
/// name into an address.
///
/// **The second half is the real-DNS check this test used to be, and it does not gate.** It asks
/// 10.0.2.3, which libslirp NATs to the host's configured nameserver (`get_dns_addr_libresolv`),
/// so an answer is a fact about the developer's machine. No answer skips, with a reason that says
/// the first half passed (milestone 214's partial shape). A reply that arrives and is not a valid
/// answer to our query still fails, because that would be ours.
// RISC-V twin: `riscv_virtio_tests::a_name_resolves_through_the_stack_and_a_real_one_when_the_host_answers`. Gated here rather than run twice: that
// module drives the same property on the other instruction set, through the same `block_driver` and
// `net_stack` binaries this leg now uses, and a second copy would double the suite's slowest tests
// to prove nothing new. (It read "through hello's roles" until milestone 291, when aarch64 stopped
// having any: the two legs differ by ISA now and by nothing else.) See this module's comment on
// the two kinds of gate.
/// Falsification: replayable `system_tests/falsifications/user.network_tests.a_name_resolves_through_the_stack_and_a_real_one_when_the_host_answers.patch`
#[cfg(target_arch = "aarch64")]
#[test_case]
fn a_name_resolves_through_the_stack_and_a_real_one_when_the_host_answers() {
    let Some((report, net)) = virtio_service::start_net_stack(
        net_stack_image(),
        NET_TEST_NAME_RESOLUTION,
        false,
        socket_contract::NO_LISTEN_GRANT,
    ) else {
        crate::testing::skip!("no virtio-net device attached");
    };
    let [gating, real, ..] = sched::ipc_receive(report);
    assert_eq!(
        gating, NET_CLIENT_OK,
        "name resolution against the runners' name server failed (client code {gating:#x}: \
         0xE1 then the case in NAME_SERVER_CASES then the stage, where F is a wrong verdict)",
    );
    net.release_or_fail("a net test's net_stack");
    if real == NET_CLIENT_NO_ANSWER {
        // The partial shape milestone 214 (a test that prints "skipping" and returns is counted as
        // passed) settled on: the gating half ran and was asserted above, and the conditioned half
        // was never put to the test, so the reason says which is which.
        crate::testing::skip!(
            "the name server half passed; the host's resolver did not answer, so no real DNS name \
             was resolved this run"
        );
    }
    assert_eq!(
        real, NET_CLIENT_OK,
        "a DNS response came back from the host's resolver but was not a valid answer to our \
         query (client code {real:#x}): a defect here, not a network problem",
    );
}

/// **The socket contract, TCP end to end** (milestone 30, piece 3 phase B). A client opens a TCP
/// socket by id, connects to slirp's guestfwd echo peer (10.0.2.9:7777, piped to `/bin/cat`),
/// sends a payload, receives the echo, and closes. The full round trip, handshake through
/// bidirectional data to teardown, deterministic and zero-host-setup (nothing outlives QEMU),
/// through the client, `net_stack`, smoltcp, and the confined NIC.
// RISC-V twin: `riscv_virtio_tests::a_client_echoes_over_tcp_through_the_socket_contract`. Gated here rather than run twice: that
// module drives the same property on the other instruction set, through the same `block_driver` and
// `net_stack` binaries this leg now uses, and a second copy would double the suite's slowest tests
// to prove nothing new. (It read "through hello's roles" until milestone 291, when aarch64 stopped
// having any: the two legs differ by ISA now and by nothing else.) See this module's comment on
// the two kinds of gate.
#[cfg(target_arch = "aarch64")]
#[test_case]
fn a_client_echoes_over_tcp_through_the_socket_contract() {
    let Some((report, net)) = virtio_service::start_net_stack(
        net_stack_image(),
        NET_TEST_TCP_ECHO,
        false,
        socket_contract::NO_LISTEN_GRANT,
    ) else {
        crate::testing::skip!("no virtio-net device attached");
    };
    let verdict = sched::ipc_receive(report)[0];
    assert_eq!(
        verdict, NET_CLIENT_OK,
        "the TCP echo round trip through the socket contract failed (client code {verdict:#x})",
    );
    net.release_or_fail("a net test's net_stack");
}

/// **A package fetched over plain HTTP is accepted only by the digest the image vouches for**
/// (milestone 198 (a package manager) rung 3a). The client `GET`s `uptime 0.1.0` from a host process
/// on the same network (`helpers/package-http-peer`, a `guestfwd` peer), hashes it as it arrives
/// through `http_response` and the net stack, and compares against the image's package catalogue,
/// which the archive build packed above the measurement table. **Then it fetches the peer's copy with
/// one byte flipped, which must be refused**: that half is what gives the first its meaning, because
/// the tampered response is a complete, correct HTTP exchange and only the digest can tell. Both
/// fetches share one `net_stack`, because every stack a test starts holds a virtio slot for the rest
/// of the boot (`MAX_DEVICES`). Nothing else in the tree fetches a file over TCP or checks a
/// package's digest on a target.
// RISC-V twin: `riscv_virtio_tests::a_package_fetched_over_http_is_accepted_only_by_the_image_digest`.
#[cfg(target_arch = "aarch64")]
#[test_case]
fn a_package_fetched_over_http_is_accepted_only_by_the_image_digest() {
    let catalogue = program(package_archive::CATALOG)
        .expect("no package catalogue in the initial ramdisk: the archive build packs one");
    let Some((report, net)) = virtio_service::start_package_fetch(
        net_stack_image(),
        NET_TEST_HTTP_PACKAGE,
        catalogue.len() as u64,
        catalogue,
    ) else {
        crate::testing::skip!("no virtio-net device attached");
    };
    let [genuine, tampered, ..] = sched::ipc_receive(report);
    assert_eq!(
        genuine, NET_CLIENT_OK,
        "the package fetched over HTTP was not accepted (client code {genuine:#x}; \
         {NET_CLIENT_DIGEST_REFUSED} means its digest disagreed with the image's catalogue)",
    );
    assert_eq!(
        tampered, NET_CLIENT_DIGEST_REFUSED,
        "a package with one byte flipped was not refused by digest (client code {tampered:#x}; \
         {NET_CLIENT_OK} would mean it was accepted)",
    );
    net.release_or_fail("the package fetch's net_stack");
}

/// The same TCP echo round trip over the PCIe transport, behind the IOMMU.
// RISC-V twin: `riscv_virtio_tests::a_client_echoes_over_tcp_through_the_socket_contract_pci`. Gated here rather than run twice: that
// module drives the same property on the other instruction set, through the same `block_driver` and
// `net_stack` binaries this leg now uses, and a second copy would double the suite's slowest tests
// to prove nothing new. (It read "through hello's roles" until milestone 291, when aarch64 stopped
// having any: the two legs differ by ISA now and by nothing else.) See this module's comment on
// the two kinds of gate.
#[cfg(target_arch = "aarch64")]
#[test_case]
fn a_client_echoes_over_tcp_through_the_socket_contract_pci() {
    let Some((report, net)) = virtio_service::start_net_stack(
        net_stack_image(),
        NET_TEST_TCP_ECHO,
        true,
        socket_contract::NO_LISTEN_GRANT,
    ) else {
        crate::testing::skip!("no virtio-net-pci device attached");
    };
    let verdict = sched::ipc_receive(report)[0];
    assert_eq!(
        verdict, NET_CLIENT_OK,
        "the TCP echo round trip over PCIe failed (client code {verdict:#x})",
    );
    net.release_or_fail("a net test's net_stack");
}

/// **Regression: reusing a socket id is safe** (the ephemeral-port fix). A client opens a TCP
/// socket on id 0, connects to the echo peer, closes it, then reopens the same id and connects
/// again. `net_stack` derived the local port from the socket id, so the reopen reused the exact port and
/// the second connect stalled on a slirp flow that had not cleared; the rotating allocator hands
/// the reopen a fresh port, so both connects complete. The client reports OK only if they do.
// RISC-V twin: `riscv_virtio_tests::a_reopened_socket_id_connects_again_over_tcp`. Gated here rather than run twice: that
// module drives the same property on the other instruction set, through the same `block_driver` and
// `net_stack` binaries this leg now uses, and a second copy would double the suite's slowest tests
// to prove nothing new. (It read "through hello's roles" until milestone 291, when aarch64 stopped
// having any: the two legs differ by ISA now and by nothing else.) See this module's comment on
// the two kinds of gate.
#[cfg(target_arch = "aarch64")]
#[test_case]
fn a_reopened_socket_id_connects_again_over_tcp() {
    let Some((report, net)) = virtio_service::start_net_stack(
        net_stack_image(),
        NET_TEST_TCP_REOPEN,
        false,
        socket_contract::NO_LISTEN_GRANT,
    ) else {
        crate::testing::skip!("no virtio-net device attached");
    };
    let verdict = sched::ipc_receive(report)[0];
    assert_eq!(
        verdict, NET_CLIENT_OK,
        "reopening a socket id and connecting again failed (client code {verdict:#x}): the \
         ephemeral local port is not independent of the socket id",
    );
    net.release_or_fail("a net test's net_stack");
}

/// **The guest is connected TO, on a port it was granted** (milestone 107). Every network exchange
/// this project had proved was outbound: the TCP gate connects to a slirp `guestfwd` peer, the UDP
/// gate is a request, the DHCP bring-up is a client. nife could reach the network and could
/// not be reached.
///
/// Here a **host process** opens a TCP connection to a port QEMU forwards into the guest
/// (`hostfwd`, the mirror of the `guestfwd` the outbound gate uses), sends a payload, and reads
/// back an answer the guest *composed*. The guest holds only a `Stack` rendezvous and a shared frame,
/// listens on the one port its spawn granted, accepts, reads, answers, and does the whole thing
/// again on the same listener, which is what proves the listener re-arms rather than serving one
/// connection and going deaf. The host side is xtask's inbound prober, running beside the suite the
/// way the scanout referee does.
///
/// **The grant half rides in the same exchange**, before the first connection: 8080 must be refused
/// as a matter of *authority* (`LISTEN_DENIED`, a distinct answer from "in use", because the two
/// call for opposite responses from a client), 7778 must bind, and asking for 7778 again on a second
/// socket id must collide. That is who-binds-the-port answered concretely: a port is an exclusive
/// name in a shared namespace, so it is authority, and the **spawn service** decides the range.
/// Note that no frame is attached until after all of it, because a listener carries no bytes.
///
/// It is one test rather than two because, when it was written, two net servers did not fit: the
/// second cost an untyped region nothing reclaimed, and the boot had no such run left (see
/// `virtio::MAX_DEVICES`). **That constraint was lifted on 2026-08-16**, when a net service became
/// reclaimable (notes/frames.md); the tests stay merged because splitting them is its own change
/// with its own argument. The stage codes stand in for the names the second test would have had.
///
/// **The UDP bind grant rides in this same spawn too** (milestone 55's stack half), for the same
/// memory reason, re-measured by the lane that built it: a twelfth net server died as
/// `Unmappable(OutOfPageFrames)` in an unrelated later test. After the accept rounds, the client
/// proves that binding a fixed UDP port is an authority: a port outside the spawn's
/// `udp_bind_grant` is refused, the granted one binds and is exclusive, and since this spawn's
/// word carries both grants, the composed packing is what the machine exercises.
///
/// **Two more clients rode this spawn once, and both are gone.** Milestone 54's SMB adapter, with
/// an authenticated fs-backed share and the credential service beside it, until 2026-08-30
/// (notes/smb.md). And milestone 55's multicast DNS responder, whose group-addressed answers xtask's
/// multicast prober decoded off a frame-level hub, until milestone 298 retired it on 2026-09-15
/// (notes/mdns.md). With it went the only check that a datagram addressed to a joined group is
/// accepted and that a multicast `SENDTO` reaches the wire. The source endpoint on a UDP `RECEIVE` did
/// not go with it: the TFTP exchange proves that (stage codes 0xE046 and 0xE047).
// RISC-V twin: `riscv_virtio_tests::a_host_process_connects_to_the_guest_and_is_answered`.
#[cfg(target_arch = "aarch64")]
#[test_case]
fn a_host_process_connects_to_the_guest_and_is_answered() {
    // E2's baseline (milestone 134): `sched::thread_count()` counts every live thread in the
    // WHOLE boot, and the full suite runs 279 `#[test_case]`s in one continuous boot rather than
    // one per test, so an absolute reading taken here would include whatever earlier tests left
    // allocated (a first attempt read 95, most of which turned out to be exactly that). Taking the
    // delta against this baseline is what isolates "how many threads did wiring THIS topology
    // create" from "how many threads exist in the boot at this point in the suite".
    let e2_baseline_threads = sched::thread_count();
    let Some((report, net)) = virtio_service::start_net_stack(
        net_stack_image(),
        NET_TEST_TCP_ACCEPT,
        false,
        socket_contract::listen_grant(NET_LISTEN_PORT, NET_LISTEN_PORT)
            | socket_contract::udp_bind_grant(NET_UDP_GRANT_BOTTOM, NET_UDP_GRANT_TOP),
    ) else {
        crate::testing::skip!("no virtio-net device attached");
    };
    // E2 (milestone 134, design/roadmap/0134-the-measurements-that-decide.md): the thread census on
    // the customer path. Every process this topology needs is already spawned by this point
    // (`net_stack` and the echo client), and neither spawns another kernel
    // thread per connection or per request (each is a single-threaded event loop over its own
    // rendezvous), so this count is already the peak: it does not grow further as the host prober's
    // connections arrive. See notes/benchmarks.md and this milestone's register entry for what this
    // settles. Reported as a delta against `e2_baseline_threads` (see this function's top), not as
    // the absolute reading: the absolute count includes whatever earlier tests in this suite's one
    // continuous boot left allocated, which is not this measure's subject.
    //
    // The census covered a wider topology until 2026-08-30: it also held the block server, the FS
    // server, the SMB adapter and the credential service, all wired for the SMB gate. notes/smb.md
    // records what that boot was and why it is gone, and the number here is not comparable to the
    // one milestone 134 recorded against it. It lost the multicast DNS responder on 2026-09-15
    // (milestone 298, notes/mdns.md), one thread fewer again.
    crate::println!(
        "    (E2 thread census: {} threads created by wiring this customer-path topology \
         ({} live now, {e2_baseline_threads} live before this test wired anything): main + \
         net_stack + echo client)",
        sched::thread_count().saturating_sub(e2_baseline_threads),
        sched::thread_count(),
    );
    let verdict = sched::ipc_receive(report)[0];
    assert_eq!(
        verdict, NET_CLIENT_OK,
        "the guest did not serve the inbound exchange (client code {verdict:#x}). 0xE050 or \
         0xE080 mean a port outside a grant was bound anyway, which is the capability failure; \
         0xE060 or 0xE070 means nobody ever connected, which is the host side: is the runner \
         adding a hostfwd (NIFE_HOSTFWD_PORT) and is xtask's inbound prober running beside this \
         suite? 0xE082 or 0xE084 mean the UDP bind grant admitted or refused the wrong port",
    );
    net.release_or_fail("a net test's net_stack");
}

/// The exact transcript `std_exerciser` prints when it is granted the network **and refused every
/// listening port**. Pinned so a drift in the net PAL, the contract, or the demo is a loud diff
/// rather than a mystery.
///
/// `listen refused` is milestone 64's negative control and it costs this boot nothing: the stack
/// this program is spawned with carries `socket_contract::NO_LISTEN_GRANT`, so
/// `std::net::TcpListener::bind` answers `PermissionDenied`, and the program says so on its way
/// past rather than quietly running a smaller demo. A lane that broke the grant check open would
/// turn this line into `listen ok` and fail here, in a transcript comparison, rather than passing
/// with more authority than it was given.
#[cfg(target_arch = "aarch64")]
const STD_NET_EXPECTED: &[u8] = b"std net on nife\nlisten refused\nudp ok\ntcp echo ok\n";

/// The exact transcript the same binary prints when its stack **is** granted the listening port
/// (milestone 64's inbound half). Four lines, and each is a separate claim: the granted port binds,
/// a port outside the grant is refused as a matter of authority, the granted port is exclusive, and
/// the listener served `socket_contract::fixture::ROUNDS` connections one after another.
#[cfg(target_arch = "aarch64")]
const STD_LISTEN_EXPECTED: &[u8] =
    b"std net on nife\nlisten ok\ndenied refused\nin use refused\nserved 2\n";

/// **`std::net` end to end over the socket contract** (milestone 27 phase two): the `std_exerciser`
/// std binary, given the network, does a real UDP DNS query and a TCP echo round trip through
/// `std::net::{UdpSocket, TcpStream}`, whose PAL binds to `net_stack`'s contract. The program never
/// sees a capability or a socket id; it writes to a socket and reads from it. This closes the
/// `net honestly unsupported` gap from phase one: std's networking runs on the native ABI,
/// reaching the same path the hand-written client does through std's blocking API. Its stdout
/// is reassembled off the rendezvous and compared byte for byte, the `std_exerciser` discipline.
// RISC-V twin: `riscv_virtio_tests::std_net_runs_over_the_socket_contract`. Gated here rather than run twice: that
// module drives the same property on the other instruction set, through the same `block_driver` and
// `net_stack` binaries this leg now uses, and a second copy would double the suite's slowest tests
// to prove nothing new. (It read "through hello's roles" until milestone 291, when aarch64 stopped
// having any: the two legs differ by ISA now and by nothing else.) See this module's comment on
// the two kinds of gate.
#[cfg(target_arch = "aarch64")]
#[test_case]
fn std_net_runs_over_the_socket_contract() {
    if std_service::std_exerciser_image().is_none() {
        crate::testing::skip!(std_service::NO_STD_EXERCISER);
    }
    let Some((report, writer, net)) = virtio_service::start_net_std(
        net_stack_image(),
        std_exerciser_image(),
        socket_contract::NO_LISTEN_GRANT,
    ) else {
        crate::testing::skip!("no virtio-net device attached");
    };

    assert_std_transcript_from(report, writer, STD_NET_EXPECTED, "std net", false);
    net.release_or_fail("a net test's net_stack");
}

/// **A `std::net::TcpListener` serves a port it was granted, and refuses one it was not**
/// (milestone 64's inbound half, on milestone 107's contract).
///
/// The ordinary `std_exerciser` binary, spawned over a stack whose listen grant is exactly
/// `NET_LISTEN_PORT`, binds that port through `std::net`, accepts connections a **host** process
/// opens through QEMU's `hostfwd`, reads each request and composes an answer. The program names no
/// capability and no socket id; it calls `bind`, `accept`, `read_exact` and `write_all`, which is
/// what a Rust server is written out of. This is the difference between "a crate compiles" and "a
/// server runs", and it is what milestone 55's Samba-shaped workload stands on.
///
/// **Two connections, and the second is the load-bearing one.** A listener that accepts once and
/// goes deaf would pass a one-round gate and is precisely what a file server cannot use; the re-arm
/// happens inside `ACCEPT` (notes/net/the-inbound-half.md) and nothing but a second `accept()` proves it.
///
/// **The refusals ride in this same spawn rather than in a test of their own**, which is the
/// machine's call and not a preference: a net test spends minutes in `net_stack`'s userspace
/// smoltcp poll, so a boot is the expensive unit. `denied refused` is
/// `socket_contract::fixture::DENIED_PORT`, outside the grant this stack carries, answered
/// `PermissionDenied`; `in use refused` is the granted port asked for twice, answered `AddrInUse`.
/// The **whole-stack** refusal is the sibling test above, whose stack carries no grant at all.
///
/// The host's half is `xtask`'s inbound prober, which requires its own bytes back from the guest
/// and fails the leg if nobody answered. The guest's half covers "the connection was served"; the
/// host's covers "and what came back was the guest's own answer", which the guest cannot know.
// RISC-V twin: `riscv_virtio_tests::a_std_program_serves_a_granted_listening_port`.
#[cfg(target_arch = "aarch64")]
#[test_case]
fn a_std_program_serves_a_granted_listening_port() {
    if std_service::std_exerciser_image().is_none() {
        crate::testing::skip!(std_service::NO_STD_EXERCISER);
    }
    let Some((report, writer, net)) = virtio_service::start_net_std(
        net_stack_image(),
        std_exerciser_image(),
        socket_contract::listen_grant(NET_LISTEN_PORT, NET_LISTEN_PORT),
    ) else {
        crate::testing::skip!("no virtio-net device attached");
    };

    assert_std_transcript_from(report, writer, STD_LISTEN_EXPECTED, "std listen", true);
    net.release_or_fail("a net test's net_stack");
}
