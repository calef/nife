//! The synchronous-rendezvous state machine: the decision core of the kernel's IPC. It owns the two
//! wait queues and the pending-signal count, and decides what a send, a receive or a signal does
//! with them; the kernel wraps it with what the queues cannot express (mailboxes, waking a thread
//! onto a run queue, the one-shot Reply). The policy lives here, proved, so the scheduler calls it
//! rather than hand-rolling the same branch six times.
//!
//! The wait queues are intrusive (`crates/intrusive_fifo`): in the kernel a queue entry is the TCB,
//! threaded through the link the run queues use, so a blocked thread waits on exactly one
//! rendezvous because there is one field, and queueing cannot allocate.
//!
//! The load-bearing invariant: **at most one wait queue is ever non-empty.** A sender that finds a
//! receiver rendezvouses instead of queueing, so a thread queues only when nobody was waiting for
//! it. Every operation is proved to preserve it over the real intrusive queues; the `Fifo`'s own
//! ordering is proved in its crate.
//!
//! Name: ratified 2026-09-18 (calef, §154 (the acronym test is whether the phrase is spoken)),
//! deratifying the 2026-08-01 ratification: "inter-process communication" is a phrase people say,
//! where "peripheral component interconnect" is not. IPC the concept keeps its name everywhere
//! else. Refused `ipc`.

#![cfg_attr(not(test), no_std)]

use core::ptr::NonNull;

use intrusive_fifo::{Fifo, Node, Unqueued};

pub mod futex;
pub mod notification;
pub mod timer;

/// Machine-checked proofs of the rendezvous state machine (DECISIONS §14, milestone 18; restated
/// over the intrusive queues at milestone 14 phase A.3, so the rewire did not demote proved code
/// back to argued code).
///
/// Every operation is proved to preserve the one-queue invariant, and each decision is proved to
/// match the rule. These are inductive-step proofs: assume a valid state, apply one operation,
/// check. A non-empty queue is modeled with a single waiter, because the decision and the
/// invariant depend only on whether a queue is *empty*, never on its length: an operation pops
/// (only shrinking) and pushes only to a queue that was empty, so the emptiness pattern
/// transitions identically for one waiter or many. FIFO order within a queue is the `intrusive_fifo`
/// crate's own proof; these harnesses prove the decisions made over it.
///
/// # The obligations the one `unsafe` call in each harness discharges
///
/// Since milestone 139's round 10 the operations are safe and take [`Unqueued`] tokens, so the only
/// `unsafe` left is minting the tokens, once per harness, through [`mint`]. Its obligations are
/// stated here once rather than at each site.
///
/// **Every node outlives the rendezvous.** Each harness declares its `N`s in one `let` before it
/// declares `e`, and Rust drops locals in reverse declaration order, so the `Rendezvous` is destroyed
/// first. A node still parked on a queue when the harness returns was therefore valid for the whole
/// of its time there, which is rule 2 of the token's contract.
///
/// **Every node is on no queue when it is minted, and minted once.** Each `N::new()` starts with a
/// null link, `e` starts empty, and no harness mints the same node twice. That is why the harnesses
/// carry a separate `me` (and, in one case, a `me2`) rather than reusing `s` or `r`.
///
/// Both are properties the *harness* has, not properties Kani checks. Kani would catch a dangling
/// dereference if one of these were false and a proof reached it, but nothing here proves the
/// contract is kept; that is what the comments are for.
#[cfg(kani)]
mod verification;

#[cfg(test)]
mod tests;

mod rendezvous;

pub use rendezvous::{Receive, Rendezvous, Send};
