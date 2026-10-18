# Credentials: what is proven, and where

An appendix to [`notes/credentials.md`](../credentials.md). It lists every host test, proof and
guest test behind the credential service, and what each pins.

Host tests (`cargo test -p credentialer -p credential_contract`, milliseconds, no emulator):

- RFC 9106's and the reference implementation's known-answer vectors, through the same entry
  point the service uses.
- The right secret matches; the wrong one does not; an unprovisioned identity does not; and one
  person's password does not open another person's account.
- Every single-byte corruption of an encoded record is rejected or decodes to a different
  record. Exhaustive over positions and four bit patterns.
- A hostile cost is refused before it reaches the library (the overflow in
  [argon2id.md](argon2id.md)).
- The lookup lands on the decoy for a miss and on the record for a hit, at every slot position.
- A miss and a hit take comparable time.

Proofs (`script/verify`, three Kani harnesses over `credential_contract`, 30 checks, 0.2 s). Both
properties are about what an adversary can send or receive, and an adversary is not limited to the
values a test author thought of:

- No request word makes the server's parse read outside the page. For every one of the 2^64
  first words a client can send, `read` either refuses it or returns two slices inside the page with
  exactly the lengths the word claimed. This is what lets the serve loop have no arithmetic in it
  that could go wrong.
- Nothing but `MATCH` authenticates, for every one of the 2^64 words a caller can receive. The
  host test sweeps a few dozen values around the boundary; this sweeps all of them.
- A request word round-trips its opcode and both lengths, over every combination `place` can build.

Guest tests (`system_tests/src/user/credential_tests.rs`, on aarch64 and riscv64, same assertions):

- Provisioning fills the store and the seal closes it. Three identities go in, and the fourth is
  refused with `FULL` rather than silently replacing somebody. The seal is accepted, and the
  service's readiness message arrives *after* the seal, which is the evidence that the provisioning loop was
  left rather than merely that a `SEAL` was answered.
- A userspace client with one endpoint and no store gets the right answer to four questions,
  over a real Argon2id verification with a salt drawn from a real virtio-rng.
- The identical endowment cannot write the store. The attacker holds exactly what the honest
  client holds and tries `PUT`, `SEAL`, an undefined opcode, and lengths outside the contract. All
  four are `MALFORMED`, and the credential it tried to install does not work.
- The service survives all of it and answers correctly afterwards, because a credential service
  a malformed request can kill is a login outage anybody can cause.
- The kernel reads the shared frame through the direct map, which no userspace program can do,
  and finds neither the presented secret nor any nonzero byte.
