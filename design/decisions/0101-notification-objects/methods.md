# §101's methods and rights, as ruled

Moved verbatim from [`0101-notification-objects.md`](../0101-notification-objects.md) on 2026-10-13
(UTC) to bring that page under §212 (a prose budget)'s 3,000-word cap by the whole-file count. The
page keeps both headings, and the text below is unchanged. This is the surface as §101 ruled it; the
shipped ABI is `crates/abi/src/notification.rs`.

## New methods

```rust
pub mod notification {
    /// invoke(cap, SIGNAL, word, _, _) -> 0.
    /// OR `word` into the notification word. If a thread is waiting, wake it and deliver the
    /// accumulated word. If a notification is bound to a TCB and that TCB is blocked in RECV on
    /// an endpoint, wake it there instead (the signal arrives as a badge on the RECV return).
    /// Never blocks, never lost. Needs WRITE.
    pub const SIGNAL: u64 = 0;

    /// invoke(cap, WAIT, _, _, _) -> word.
    /// If the notification word is non-zero, return it and clear it. If zero, block until a
    /// signal arrives. Needs READ.
    pub const WAIT: u64 = 1;

    /// invoke(cap, POLL, _, _, _) -> word.
    /// Non-blocking WAIT: return the word (possibly zero) without blocking. Needs READ.
    pub const POLL: u64 = 2;

    /// invoke(cap, BIND, tcb_slot, _, _) -> 0.
    /// Bind this notification to the TCB in `tcb_slot`. From now on, signals to this notification
    /// are delivered to that TCB even if it is blocked in RECV on an endpoint. At most one
    /// notification may be bound to a TCB. Needs WRITE on the notification and WRITE on the TCB.
    pub const BIND: u64 = 3;
}
```

## Rights

Notifications use the existing rights model:

| Right | What it permits |
|-------|----------------|
| `READ` | `WAIT`, `POLL` |
| `WRITE` | `SIGNAL`, `BIND` |
| `GRANT` | Delegate the notification capability to another process |

No new rights bits. `ENUMERATE` is not needed (a notification has no children to survey).
