use super::*;

/// A minimal node: a link and nothing else, the way the proofs like it.
struct N {
    next: Option<NonNull<N>>,
}

impl N {
    fn new() -> Self {
        N { next: None }
    }
}

// SAFETY: `next` and `set_next` read and write the same `next` field and nothing else, which is
// the whole of the `Node` contract.
unsafe impl Node for N {
    fn next(&self) -> Option<NonNull<Self>> {
        self.next
    }
    fn set_next(&mut self, next: Option<NonNull<Self>>) {
        self.next = next;
    }
}

/// Mint `n`'s token.
///
/// # Safety
/// `n` is a harness local declared before the rendezvous, on no queue, and minted only once.
unsafe fn mint(n: &mut N) -> Unqueued<N> {
    // SAFETY: this function's own contract is `Unqueued::new`'s.
    unsafe { Unqueued::new(NonNull::from(n)) }
}

/// Put `e` into an arbitrary valid state: at most one queue non-empty (modeled as one
/// waiter), and a symbolic pending count. Safe now: the waiters arrive as tokens, so "valid,
/// distinct and unqueued" is what they are rather than what this function asks for. A token
/// that is not queued is dropped, which strands its node and nothing else.
fn seed(e: &mut Rendezvous<N>, sender: Unqueued<N>, receiver: Unqueued<N>) {
    e.pending = kani::any();
    // Symbolic too, so every harness covers a rendezvous that carries an interrupt as well as
    // one that does not, including one bound after a sender had already queued.
    e.bound_to_interrupt = kani::any();
    match kani::any::<u8>() {
        0 => e.senders.push_back(sender),
        1 => e.receivers.push_back(receiver),
        _ => {} // both empty
    }
}

/// Falsification: replayable `crates/inter_process_communication/falsifications/verification.send_preserves_the_invariant.patch`
#[kani::proof]
fn send_preserves_the_invariant() {
    let (mut s, mut r, mut me) = (N::new(), N::new(), N::new());
    let mut e: Rendezvous<N> = Rendezvous::new();
    // SAFETY: three distinct fresh locals declared before `e`, each minted once (module note).
    let (s, r, me) = unsafe { (mint(&mut s), mint(&mut r), mint(&mut me)) };
    seed(&mut e, s, r);
    let _ = e.send(me);
    assert!(e.one_queue_invariant());
}

/// Falsification: replayable `crates/inter_process_communication/falsifications/verification.receive_preserves_the_invariant.patch`
#[kani::proof]
fn receive_preserves_the_invariant() {
    let (mut s, mut r, mut me) = (N::new(), N::new(), N::new());
    let mut e: Rendezvous<N> = Rendezvous::new();
    // SAFETY: as in `send_preserves_the_invariant` above.
    let (s, r, me) = unsafe { (mint(&mut s), mint(&mut r), mint(&mut me)) };
    seed(&mut e, s, r);
    let _ = e.receive(me);
    assert!(e.one_queue_invariant());
}

/// Falsification: unfalsifiable. No minimal defect in `signal` can turn this harness red, and
/// that is a fact about the operation rather than a gap in the effort. The invariant is "at
/// most one queue is non-empty", and `signal` takes no node: its two branches pop a receiver
/// (which can only empty a queue) and increment a counter. Nothing it can be mistakenly
/// written to do enqueues anything, so every mutation of it either leaves the invariant true
/// or is caught by a neighbouring harness instead. The harness earns its place as a guard
/// against `signal` growing an enqueue path later, not as evidence about the code today.
/// Recording a patch here would mean patching a sibling operation and filing the red under
/// this name, which is the manufactured evidence DECISIONS §134 (a harness carries a
/// machine-replayable falsification record, or it is not evidence) exists to refuse.
#[kani::proof]
fn signal_preserves_the_invariant() {
    let (mut s, mut r) = (N::new(), N::new());
    let mut e: Rendezvous<N> = Rendezvous::new();
    // SAFETY: two distinct fresh locals declared before `e`, each minted once.
    let (s, r) = unsafe { (mint(&mut s), mint(&mut r)) };
    seed(&mut e, s, r);
    let _ = e.signal();
    assert!(e.one_queue_invariant());
}

/// **A send rendezvouses exactly when a receiver was waiting**, and with exactly *that*
/// receiver, else blocks. So a message is never dropped, a sender never blocks past a ready
/// receiver, and the rendezvous partner is the queued thread and no other.
///
/// Unless the rendezvous carries an interrupt (DECISIONS §101 (notification objects), ruling
/// B, 2026-09-26): then the send is refused and changes nothing. Folded into this harness rather
/// than given its own, because it is the same decision's other branch. Removing the refusal
/// from `send` turns this red at both `!bound` assertions, checked 2026-09-26 by
/// milestone 603 (an interrupt's endpoint refuses every send); the replayable record below is
/// the older defect, since the convention holds one patch per harness.
/// Falsification: replayable `crates/inter_process_communication/falsifications/verification.send_rendezvous_iff_a_receiver_waited.patch`
#[kani::proof]
fn send_rendezvous_iff_a_receiver_waited() {
    let (mut s, mut r, mut me) = (N::new(), N::new(), N::new());
    let receiver_ptr = NonNull::from(&mut r);
    let mut e: Rendezvous<N> = Rendezvous::new();
    // SAFETY: three distinct fresh locals declared before `e`, each minted once. `receiver_ptr`
    // is only ever compared, never dereferenced.
    let (s, r, me) = unsafe { (mint(&mut s), mint(&mut r), mint(&mut me)) };
    seed(&mut e, s, r);

    let had_receiver = !e.receivers.is_empty();
    let bound = e.bound_to_interrupt;
    let before = (e.senders.is_empty(), e.receivers.is_empty(), e.pending);
    match e.send(me) {
        Send::Rendezvous(got, _) => {
            assert!(had_receiver && !bound);
            assert!(
                got == receiver_ptr,
                "rendezvoused with a thread nobody queued"
            );
        }
        Send::Blocked => assert!(!had_receiver && !bound),
        // Refused, and nothing touched: not the waiting driver popped, not the sender parked
        // where the driver's next receive would find it, not the pending interrupt count.
        Send::Refused(_) => {
            assert!(bound);
            assert_eq!(
                (e.senders.is_empty(), e.receivers.is_empty(), e.pending),
                before
            );
        }
    }
}

/// **A caller's token comes back exactly when its node did not join a queue, and it is the
/// caller's own.** The obligation milestone 139's round 10 created: the kernel files the token
/// a `send` or `receive` hands back onto the calling thread's TCB (`sched::hold_token`), so a
/// verdict that returned the *partner's* token in the caller's place would leave the running
/// thread holding a token for a thread that is about to be queued by its waker, which is two
/// tokens for one node and the
/// double-queue this type exists to make impossible. And a verdict that queued the caller and
/// also returned its token would be the same defect from the other side.
///
/// So for both operations, over every seeded state: the verdict that holds no token for the
/// caller is exactly the one that grew the caller's queue, and every other verdict hands back a
/// token naming the caller and leaves the caller's queue as it was.
/// Falsification: replayable `crates/inter_process_communication/falsifications/verification.a_token_comes_back_exactly_when_its_node_did_not_queue.patch`
#[kani::proof]
fn a_token_comes_back_exactly_when_its_node_did_not_queue() {
    let (mut s, mut r, mut me) = (N::new(), N::new(), N::new());
    let me_ptr = NonNull::from(&mut me);
    let mut e: Rendezvous<N> = Rendezvous::new();
    // SAFETY: three distinct fresh locals declared before `e`, each minted once. `me_ptr` is
    // only ever compared.
    let (s, r, me) = unsafe { (mint(&mut s), mint(&mut r), mint(&mut me)) };
    seed(&mut e, s, r);
    let (senders, receivers) = (e.senders.len(), e.receivers.len());

    if kani::any() {
        match e.send(me) {
            Send::Blocked => assert_eq!(e.senders.len(), senders + 1),
            Send::Rendezvous(_, back) | Send::Refused(back) => {
                assert!(back == me_ptr, "handed back somebody else's token");
                assert_eq!(e.senders.len(), senders);
            }
        }
    } else {
        match e.receive(me) {
            Receive::Blocked => assert_eq!(e.receivers.len(), receivers + 1),
            Receive::Signal(back) | Receive::FromSender(_, back) => {
                assert!(back == me_ptr, "handed back somebody else's token");
                assert_eq!(e.receivers.len(), receivers);
            }
        }
    }
}

/// **Taking a waiter back off a queue preserves the one-queue invariant**, for either queue and
/// whether or not the victim was ever there.
///
/// Milestone 133 needed [`remove_receiver`](Rendezvous::remove_receiver) and asks both removes
/// on every victim, because the kernel's recorded wait role and the queue that actually holds
/// the thread genuinely disagree for a `CALL` caller that met no server. So the interesting
/// case is not the hit, it is the **miss**: a drain-and-repush over a queue the victim is not
/// on rebuilds that queue from nothing and must put it back exactly as it was. A version that
/// dropped the survivors, or left both queues non-empty, would break the invariant every other
/// proof in this module rests on.
///
/// **This one needs a loop bound, and the two facts behind that are worth the paragraph**,
/// because every other harness in this module is unbounded and finishes in a tenth of a
/// second. The first shape of it ran both removes twice each and CBMC was still working after
/// **seventeen minutes**; with `unwind(3)` and a single symbolic remove it takes 0.2 seconds.
///
/// The bound is sound rather than a shortcut: Kani checks the unwinding assertion, so a loop
/// that needed a fourth iteration would **fail** this proof rather than pass it quietly, and
/// `seed` queues at most one waiter, so two is the most a drain-and-repush can take. What the
/// bound costs is exactly what `seed` already costs everywhere in this module: the proof is
/// over a rendezvous holding at most one waiter, and a queue of many is argued rather than
/// proved.
///
/// **One remove per run, chosen symbolically, rather than the four the kernel makes**, and
/// nothing is lost by that: the removes share no state between calls, each reading one queue
/// and writing it back, so proving one over an arbitrary seeded rendezvous covers a sequence.
///
/// Since milestone 139's round 10 a hit returns the victim's token, so the harness also checks
/// that a returned token names the victim: the kernel frees or requeues the node it names.
/// Falsification: replayable `crates/inter_process_communication/falsifications/verification.removing_a_waiter_preserves_the_invariant.patch`
#[kani::proof]
#[kani::unwind(3)]
fn removing_a_waiter_preserves_the_invariant() {
    let (mut s, mut r, mut stranger) = (N::new(), N::new(), N::new());
    let (sender_ptr, receiver_ptr) = (NonNull::from(&mut s), NonNull::from(&mut r));
    let stranger_ptr = NonNull::from(&mut stranger);
    let mut e: Rendezvous<N> = Rendezvous::new();
    // SAFETY: two distinct fresh locals declared before `e`, each minted once. The three
    // pointers above are only ever compared.
    let (s, r) = unsafe { (mint(&mut s), mint(&mut r)) };
    seed(&mut e, s, r);

    // A symbolic victim: one of the two nodes `seed` may have queued, or a third that is on no
    // queue at all, which is the miss the kernel takes on every call.
    let victim = match kani::any::<u8>() {
        0 => sender_ptr,
        1 => receiver_ptr,
        _ => stranger_ptr,
    };
    let found = if kani::any() {
        e.remove_sender(victim)
    } else {
        e.remove_receiver(victim)
    };
    assert!(e.one_queue_invariant());
    // A remove that reports a hit really emptied the queue it was given: `seed` queues at most
    // one waiter, so a hit leaves both queues empty and the rendezvous idle.
    assert!(found.is_none() || e.is_idle());
    assert!(found.is_none_or(|t| t == victim));
}

/// **A pending signal is taken before a queued sender.** A receive drains a counted signal
/// first, so an async signal delivered with nobody waiting is never lost behind a later
/// synchronous sender.
/// Falsification: replayable `crates/inter_process_communication/falsifications/verification.receive_drains_a_pending_signal_first.patch`
#[kani::proof]
fn receive_drains_a_pending_signal_first() {
    let (mut s, mut r, mut me) = (N::new(), N::new(), N::new());
    let mut e: Rendezvous<N> = Rendezvous::new();
    // SAFETY: three distinct fresh locals declared before `e`, each minted once.
    let (s, r, me) = unsafe { (mint(&mut s), mint(&mut r), mint(&mut me)) };
    seed(&mut e, s, r);
    if e.pending > 0 {
        let outcome = e.receive(me);
        assert!(matches!(outcome, Receive::Signal(_)));
    }
}

/// **A collected sender is forgotten by the rendezvous.** The rendezvous half of the one-shot
/// Reply guarantee (DECISIONS §12): a `CALL`er queues as a sender and blocks; when a server's
/// receive collects it, the pop is destructive, so afterwards the rendezvous holds no name for
/// the caller in either queue and no later receive can produce it again. From that moment the
/// kernel-minted Reply capability is the *only* name for the blocked caller anywhere, and the
/// capability side (consume-on-use, proved in `crates/capability`) makes that name single-use.
///
/// One waiter covers the general case here as everywhere in this module, plus one fact the
/// queue cannot see: a blocked thread cannot run, so it cannot enqueue itself a second time.
/// Stated through emptiness (the decision core's own vocabulary; a membership scan would hand
/// the solver an unbounded loop for no added meaning).
/// Falsification: replayable `crates/inter_process_communication/falsifications/verification.a_collected_sender_is_forgotten.patch`
#[kani::proof]
fn a_collected_sender_is_forgotten() {
    let (mut s, mut r, mut me, mut me2) = (N::new(), N::new(), N::new(), N::new());
    let mut e: Rendezvous<N> = Rendezvous::new();
    // SAFETY: four distinct fresh locals declared before `e`, each minted once. `me2` exists so
    // the second receive does not need `me`'s token back from the first.
    let (s, r, me, me2) = unsafe { (mint(&mut s), mint(&mut r), mint(&mut me), mint(&mut me2)) };
    seed(&mut e, s, r);
    if matches!(e.receive(me), Receive::FromSender(..)) {
        assert!(e.senders.is_empty() && e.receivers.is_empty());
        assert!(!matches!(e.receive(me2), Receive::FromSender(..)));
    }
}
