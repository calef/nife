//! **`mdr`: the markdown renderer** (notes/documentation.md). It reads markdown on its input and
//! writes it rendered on its output; the rendering is `crates/documentation`, host-tested, and what
//! lives here is the two ends of the sink contract and eight kilobytes of `.bss`.
//!
//! **It cannot name a page.** A Unix `man` opens the file it prints, so it can open any file its
//! caller can. This is handed bytes: `mdr notes/glob.md` is the shell resolving the name against
//! its own directory capability and streaming the file, and `mdr < notes/glob.md`, `mdr
//! notes/glob.md` and `something | mdr` are one behaviour with three sources. Finding a page is
//! `apropos`'s job, a shell builtin, because a search must read the store's index.
//!
//! It holds slot 0, the output sink (`WRITE`), and slot 1, the input source (`READ`): no memory
//! grant (the renderer never allocates), file, directory, shared page or clock. It cannot
//! tell whether its sink is a terminal, and the sink contract keeps it so, which is what makes
//! redirection one grant; so it emits plain text.
//!
//! ```text
//! $ mdr gate.txt | wc
//!   1 4 26
//! $ mdr gate.txt
//!   hello world hello world
//! $ mdr
//!   mdr: reads an input stream: name a file, redirect with '<', or pipe into it
//! ```
//!
//! `gate.txt` is `2 4 24`; the rendering is one re-flowed line plus two bytes of indent
//! (`script/swish-check`).
//!
//! # BUGS
//!
//! - `mdr <page> > out.txt` is refused, naming the fix (notes/pipes.md).
//! - No pager: a keypress needs the terminal endpoint, whose read side is the keyboard, and the
//!   spawn protocol cannot hand over one line of input without it.
//! - Eighty columns always: the terminal contract has no "how wide are you" verb.
//! - Plain text only, until the shell can say a stage ends at the terminal.
//! - A line longer than `documentation::LINE_MAX` loses its tail silently, since a diagnostic in
//!   the middle of the output would be worse.
//!
//! Name: ratified 2026-09-13 (calef). Refused `doc` (promises a lookup, and builtins shadow it),
//! `man` (a format and a lookup it does neither of), `manual` (the crate's), `markdown` and
//! `markdown_renderer` (longer than a typed command needs) and `md` (names the format).

#![no_std]
// Program entry points, not the crates/ library surface milestone 68's ratchet tracks
// (DECISIONS §107): each `[[bin]]` is its own crate root with one `_start`, and 58 of them
// documenting an OS-facing ABI entry point is not what the lint is for.
#![allow(missing_docs)]
#![no_main]

use documentation::{Renderer, Sink, Style};
use user_mode_runtime::{exit, receive, send};

/// The output sink: where the rendered bytes go, in the sink contract's framing.
const SINK: u64 = 0;
/// The input source: an endpoint we hold `READ` on, carrying markdown inbound.
const SOURCE: u64 = 1;

/// The width the renderer wraps to. See `BUGS`: there is nothing to ask.
const WIDTH: u16 = 80;

/// The renderer lives here rather than on the stack because it is about eleven kilobytes and a user
/// process gets one 4 KiB stack page. `display_terminal` keeps its `Vt` the same way and for the
/// same reason.
static mut RENDERER: Renderer = Renderer::new(Style {
    width: WIDTH,
    color: false,
});

#[unsafe(no_mangle)]
pub extern "C" fn _start(_a0: u64, _a1: u64, _a2: u64) -> ! {
    // SAFETY: this process is single-threaded (`thread::spawn` is `Unsupported` here and nothing in
    // this program creates one), so there is exactly one live reference to this static for the whole
    // run. It is taken once, here, and passed down by reference. `Renderer::new` is `const` for the
    // same reason the reference is taken this way: building one on the stack would overflow the
    // single stack page before it was ever used.
    let renderer = unsafe { &mut *core::ptr::addr_of_mut!(RENDERER) };

    let mut out = Out::default();
    loop {
        let (w0, w1, w2) = receive(SOURCE);
        let mut buf = [0u8; byte_sink_protocol::INLINE_MAX];
        let n = match byte_sink_protocol::unpack(w0, w1, w2, &mut buf) {
            byte_sink_protocol::Msg::Bytes(n) => n,
            // A malformed message ends the document rather than being skipped, for the reason `wc`
            // gives: a page that is silently missing a paragraph is indistinguishable from a page
            // that never had one.
            byte_sink_protocol::Msg::Eof | byte_sink_protocol::Msg::Malformed => break,
        };
        renderer.feed(&buf[..n], &mut out);
        if out.gone {
            break;
        }
    }
    renderer.finish(&mut out);
    out.end();
    exit();
}

/// The output side of the sink contract, as a [`Sink`] the renderer can write into.
///
/// It buffers to [`byte_sink_protocol::INLINE_MAX`] and sends a message whenever it is full, because the
/// renderer emits in runs as short as one escape sequence and a message per run would be one IPC
/// round trip per attribute change.
#[derive(Default)]
struct Out {
    buf: [u8; byte_sink_protocol::INLINE_MAX],
    used: usize,
    /// The sink has been destroyed and there is nowhere left to write. Unix's `SIGPIPE`, arriving
    /// as a return code (notes/pipes.md).
    gone: bool,
}

impl Out {
    fn flush(&mut self) {
        if self.used == 0 || self.gone {
            self.used = 0;
            return;
        }
        let (w0, w1, w2, _) = byte_sink_protocol::pack(&self.buf[..self.used]);
        self.used = 0;
        if !matches!(
            byte_sink_protocol::classify(send(SINK, w0, w1, w2)),
            byte_sink_protocol::Sent::Ok
        ) {
            self.gone = true;
        }
    }

    fn end(&mut self) {
        self.flush();
        if !self.gone {
            send(SINK, byte_sink_protocol::eof(), 0, 0);
        }
    }
}

impl Sink for Out {
    fn put(&mut self, bytes: &[u8]) {
        for &b in bytes {
            if self.used == byte_sink_protocol::INLINE_MAX {
                self.flush();
            }
            if self.gone {
                return;
            }
            self.buf[self.used] = b;
            self.used += 1;
        }
    }
}

user_mode_runtime::panic_handler!();
