//! stdout/stderr over the **sink contract** (`crates/byte_sink_contract`, milestone 50), on the endpoint
//! the std slot convention puts in slot 1 (see `pal/nife/rt.rs`): SEND, register-only, up to 16
//! bytes per message.
//!
//! Bulk data over a register-only IPC is deliberate, and since milestone 50 it is forced rather
//! than merely cheap. The three-word message is the ABI's whole fastpath (notes/abi.md §2) and
//! std's own `LineWriter` batches user writes, but the real argument is substitutability: a sink
//! that also required a page mapped at an agreed address could not be swapped for another by
//! putting a different capability in this slot, and *that* is what redirection is here. See
//! notes/sink-protocol.md.
//!
//! stdin is the console, for a program granted it, and end of input for every other (milestone 892
//! (a std program at the prompt holds the network and a resolver)). The terminal endpoint, badged as
//! standard input, sits at `rt::STDIN_SLOT`; each read `CALL`s it for up to eight bytes of the line
//! being typed, which the terminal's own discipline echoes and edits. An empty slot answers an error
//! word rather than a count, and that is end of input at once.
//!
//! stderr shares the endpoint, so out and err interleave by 16-byte chunk. Recorded as a caveat in
//! notes/std.md; one endpoint is what the contract grants today.
//!
//! # A dead reader ends this program, and a missing one does not
//!
//! This used to swallow every failed SEND, on the grounds that "a program without a console still
//! runs, it just prints into the void, which is what every OS does to a process whose stdout is
//! closed". That is right for a program with no console and wrong for a pipeline, where `yes | head`
//! must end when `head` exits. `byte_sink_contract::classify` splits the two, and each half is routed
//! through the seam std already has for exactly this:
//!
//! - **No sink** ([`byte_sink_contract::Sent::NoSink`]) is [`io::ErrorKind::Unsupported`], which
//!   [`is_ebadf`] accepts, so `io::stdio`'s `handle_ebadf` swallows it and the program keeps
//!   running. It is also the same answer every other ungranted capability gives in this PAL.
//! - **A destroyed sink** ([`byte_sink_contract::Sent::Gone`]) is [`io::ErrorKind::BrokenPipe`], which
//!   `is_ebadf` rejects, so it propagates and `println!` panics. The target is panic=abort, so the
//!   process dies and the kernel attributes the fault. That is what a Rust program on Linux does
//!   when its reader exits, and it is milestone 50's "`SIGPIPE` becomes a return code".

use crate::io;
use crate::sync::atomic::{AtomicU64, Ordering};
use crate::sys::pal::nife::{rt, sinkproto};

pub struct Stdin;
pub struct Stdout;
pub type Stderr = Stdout;

/// The refusal that means "this program holds no output stream". Not an error the caller should
/// act on; `is_ebadf` turns it back into success one layer up.
fn no_sink() -> io::Error {
    io::const_error!(
        io::ErrorKind::Unsupported,
        "this program was not granted an output sink"
    )
}

/// The refusal that must end the program: there *was* a sink and it has been destroyed.
fn sink_gone() -> io::Error {
    io::const_error!(
        io::ErrorKind::BrokenPipe,
        "the sink this program was writing to no longer exists"
    )
}

/// One message on the sink. Returns how many bytes it carried.
fn send_chunk(bytes: &[u8]) -> io::Result<usize> {
    let (w0, w1, w2, n) = sinkproto::pack(bytes);
    match sinkproto::classify(rt::send(rt::STDOUT_SLOT, w0, w1, w2)) {
        sinkproto::Sent::Ok => Ok(n),
        sinkproto::Sent::Gone => Err(sink_gone()),
        sinkproto::Sent::NoSink => Err(no_sink()),
    }
}

/// **Tell the sink the stream is over.** Called from the PAL's `cleanup`, which std's runtime runs
/// after `main` returns and after it has flushed stdout, so this is the last thing on the wire.
///
/// A file sink closes its handle here and the reader of a pipe stops reading; without it a reader
/// would block forever on a writer that has exited. Errors are dropped on purpose: a program on its
/// way out cannot act on one, and a sink that is already gone has already heard everything it needs.
pub fn end_of_stream() {
    let _ = rt::send(rt::STDOUT_SLOT, sinkproto::eof(), 0, 0);
}

impl Stdin {
    pub const fn new() -> Stdin {
        Stdin
    }
}

/// Bytes a read took from the terminal and a short buffer could not hold, kept for the next read:
/// the count in the top byte and up to seven bytes below it, little-endian.
static STDIN_AHEAD: AtomicU64 = AtomicU64::new(0);

impl io::Read for Stdin {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        let ahead = STDIN_AHEAD.swap(0, Ordering::Relaxed);
        let (n, packed) = match (ahead >> 56) as usize {
            0 => {
                let (n, packed) = rt::call(rt::STDIN_SLOT, rt::STDIN_READ_REQUEST, 0);
                // A count is 0 to 8; anything else is a refusal (an empty slot, a raw-mode
                // terminal), and a refusal is end of input.
                (if n > 8 { 0 } else { n as usize }, packed)
            }
            n => (n, ahead & ((1 << 56) - 1)),
        };
        let bytes = packed.to_le_bytes();
        let took = n.min(buf.len());
        buf[..took].copy_from_slice(&bytes[..took]);
        if took < n {
            let mut rest = [0u8; 8];
            rest[..n - took].copy_from_slice(&bytes[took..n]);
            STDIN_AHEAD.store(((n - took) as u64) << 56 | u64::from_le_bytes(rest), Ordering::Relaxed);
        }
        Ok(took)
    }
}

impl Stdout {
    pub const fn new() -> Stdout {
        Stdout
    }
}

impl io::Write for Stdout {
    /// Writes as much as fits and reports it, which is what `Write` asks for; `write_all` loops.
    /// A partial write that then fails is reported as the bytes that landed, so the caller cannot
    /// mistake a torn write for a whole one.
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut written = 0;
        while written < buf.len() {
            match send_chunk(&buf[written..]) {
                Ok(n) => written += n,
                Err(e) if written == 0 => return Err(e),
                Err(_) => break,
            }
        }
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// **The buffer `io::stdin()` reads through.** Zero would hand every read an empty buffer, which
/// reads as end of input before the terminal is asked. A reply carries at most eight bytes, so a
/// small buffer loses nothing.
pub const STDIN_BUF_SIZE: usize = 64;

/// **The seam that decides whether a failed print is fatal.** `io::stdio`'s `handle_ebadf` swallows
/// an error this accepts and propagates everything else, and a propagated error makes `println!`
/// panic. This used to return `true` unconditionally, which is why the old code's "print into the
/// void" applied to a dead sink as well as a missing one.
pub fn is_ebadf(err: &io::Error) -> bool {
    err.kind() == io::ErrorKind::Unsupported
}

pub fn panic_output() -> Option<impl io::Write> {
    Some(Stdout::new())
}
