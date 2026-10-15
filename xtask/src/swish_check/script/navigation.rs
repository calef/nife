//! **The shell's own editing keys** (milestone 47, DECISIONS §227): Tab finishing a
//! program name and a file name, and `^C` discarding a line at the prompt.
//!
//! Cut from the table in `script.rs` verbatim by milestone 900 (the swish-check script
//! splits by topic before it crosses the ceiling).
//!
//! Name: provisional (milestone 900's lane, 2026-10-15 UTC).

use super::*;

/// The navigation rows of [`SWISH_CHECK_SCRIPT`], in table order.
pub(super) const LINES: &[Line] = &[
    // **The shell edits its own line** (milestone 47 (navigation and naming), DECISIONS §227 (how
    // Tab reaches the shell) option D). Tab in command position finishes a program name from the
    // names the image carries, and the finished line runs; Tab after it finishes a file name from
    // the directory the word leads into, read with the shell's own `ENUMERATE`. Each fails if the
    // shell is not in raw mode (the terminal would drop the Tab and the echo would not match), if
    // the completion inserted the wrong text, or if the completed line did not run.
    keys(1, "printe\t", "printenv ", &["TZ=UTC"]),
    keys(
        0,
        "caps wc doc/kernel/ipc-nam\t",
        "caps wc doc/kernel/ipc-naming.md ",
        &["input    ipc-naming.md"],
    ),
    // **`^C` at the prompt is a byte now**, and the shell's editor discards the line: this one must
    // answer nothing (the check is `interrupted_at_prompt`'s), and the prompt must come back for the
    // next. The supervised jobs below still take `^C` through the terminal's count, which this
    // gate presses under `interrupt_heeder` and `interrupt_ignorer`.
    keys(0, "echo abandoned\x03", "echo abandoned^C", &[]),
    line(0, "echo kept", &["kept"]),
];
