//! **`http_response`**: one HTTP/1.x `GET`, written, and the response to it, read as it arrives.
//!
//! Rung 3a of milestone 198 (a package manager, and the trivial install that makes a second
//! customer possible) fetches a package from a host on the same network over plain HTTP. DECISIONS
//! §196 (nife carries TLS) rules HTTPS for the internet rung, and under §195 (a reviewed recipe
//! vouches for a package) a package's digest, not its transport, decides whether its bytes may run,
//! so the LAN rung needs a plain client and nothing more. This crate is that client's two halves
//! that are pure logic: the request line, and a reader for the response that never holds the body.
//!
//! **The reader is incremental and holds only the head.** A response arrives a socket read at a
//! time, at most `socket_protocol::DATA_MAX` bytes each, and the body is a package that may be a
//! megabyte. [`Response::feed`] takes each read, keeps the status line and headers in a fixed
//! buffer, and hands back the part of the read that is body, so a caller can hash it and write it
//! somewhere without this crate owning a byte of it. That is the same borrowing shape
//! `package_archive` uses, for the same reason: the first consumer has no allocator.
//!
//! **The head is parsed by `httparse`**, the parser under `hyper`, which is `no_std`, has no
//! dependencies and no allocator, and is fuzzed upstream (OSS-Fuzz). Its `Partial` status is how
//! this reader knows the head has not ended yet. What stays ours is the policy around it: the head
//! cap, the request writer, and the rules below, which `httparse` does not enforce because a general
//! HTTP parser is deliberately more lenient than a package client should be.
//!
//! **It is strict, because every byte it reads came off a network.** A response it cannot read
//! exactly is refused rather than guessed at:
//!
//! - **`Content-Length` is required.** HTTP/1.0 lets a server end a body by closing the
//!   connection, but a client that accepts that cannot tell a complete package from a truncated
//!   one, and the net server reports a peer's close as a failed receive after a bounded wait, not
//!   as an end. A digest would still catch the truncation; refusing it here says why.
//! - **`Transfer-Encoding` is refused**, chunked included. A `GET` sent as HTTP/1.0 is never owed a
//!   chunked reply, so a server sending one is not speaking to us.
//! - **Lines end in CRLF** (`httparse` also accepts a bare LF, and a client that does can be made to
//!   disagree with a proxy about where a line stops), a header section is at most [`HEAD_MAX`]
//!   bytes, a duplicated `Content-Length` must agree with itself, and a body longer than it said is
//!   refused.
//!
//! # BUGS
//!
//! - **No redirects, no keep-alive, no `Range`.** One request per connection, and a `3xx` is a
//!   status like any other that the caller refuses.
//! - **A close-delimited body is refused**, per the list above. A server that omits
//!   `Content-Length` cannot serve a package to nife.
//! - **Header values other than the two it reads are not validated.** They are skipped, not
//!   interpreted, so a malformed `Date` costs nothing; a malformed line (no colon) is refused.
//! - **The head parse is `httparse`'s, and is not proved here.** What vouches for it is upstream's
//!   own fuzzing (OSS-Fuzz) and its use under `hyper`. What is ours is the glue, and
//!   `fuzz/fuzz_targets/http_response_feed.rs` checks that how the reads fall never changes the
//!   answer. No Kani harness covers the head: the reader is not on the verification path, so
//!   design/decisions/0046-dependency-rule.md takes the parser rather than proving one, and the
//!   harness the hand-written parser had never finished in a CI shard's minutes.
//! - **At most [`MAX_HEADERS`] header lines.** `httparse` parses into a caller-supplied array, and
//!   this crate has no allocator, so a head with more lines is refused as [`Error::HeadTooLong`]
//!   even when it is under the byte cap. The array lives on the stack for the length of one parse
//!   (4 KiB).
//! - **The head is parsed again on every read until it ends**, because `httparse` does not resume.
//!   Bounded by the head cap (quadratic in it, for a peer that sends one byte per read); not measured.
//! - **A minor version other than 0 or 1 is refused** (`HTTP/1.2` does not exist); `httparse`
//!   accepts only those two.
//!
//! Name: provisional 2026-09-24 (milestone 198's rung 3a consumer lane). `http` alone would claim
//! more than a request writer and a response reader; `design/naming.md` is the rule and calef's the
//! call.

#![no_std]

/// The most a status line plus headers may take. A package server's head is a few hundred bytes;
/// this is enough for a real server's, and it is a bound, so a peer that never ends its headers is
/// refused rather than read forever.
pub const HEAD_MAX: usize = 2048;

/// The most header lines a head may hold. `httparse` fills a caller-supplied array and this crate
/// has no allocator, so the array is a bound. 128 is enough for the tests' 5,291-byte redirect head,
/// which is built of 123 short lines; each slot is 32 bytes of stack.
const MAX_HEADERS: usize = 128;

/// **A head big enough for `github.com`'s redirect**, for a reader made with
/// [`Response::with_room`]. A GitHub Release download answers `302` with a head of 5,291 bytes
/// (measured 2026-10-13 UTC with an HTTP/1.0 `GET`), about 970 of them the `Location` URL and most
/// of the rest security headers. `pinned_tls_client` reads with this bound, so a package client can
/// follow the one redirect calef ruled on 2026-10-12 (UTC) on #2048; the plain LAN reader keeps
/// [`HEAD_MAX`], because its consumers include a `no_std` test component with a small stack.
///
/// Name: provisional, minted 2026-10-13 (UTC) by lane `lane/basalt-on-pages-and-releases`.
pub const REDIRECT_HEAD_MAX: usize = 8192;

/// Why a response was refused. Every variant is a peer that is not speaking the HTTP this reader
/// accepts; none of them is recoverable by reading more.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// The status line and headers ran past the reader's bound ([`HEAD_MAX`] unless it was made
    /// with more room) without the blank line that ends them, or held more than [`MAX_HEADERS`]
    /// header lines.
    HeadTooLong,
    /// The first line is not `HTTP/1.<digit> <three digits> ...`.
    BadStatusLine,
    /// A header line is not CRLF-terminated `name: value`, or a line ends in a bare LF.
    BadHeader,
    /// No `Content-Length`, so the end of the body could not be told from a dropped connection.
    NoContentLength,
    /// `Content-Length` is not a decimal number that fits a `u64`, or appears twice with two values.
    BadContentLength,
    /// A `Transfer-Encoding` header, which a response to an HTTP/1.0 request should never carry.
    TransferEncoding,
    /// More body arrived than `Content-Length` said there would be.
    BodyTooLong,
}

/// Write `GET <path> HTTP/1.0` with a `Host` header into `out`, returning its length, or `None` if
/// it does not fit or if `host` or `path` could smuggle a second line.
///
/// HTTP/1.0 rather than 1.1 so the server owes a plain, length-delimited or close-delimited body
/// and never a chunked one; [`Response`] refuses the close-delimited kind on its own terms.
pub fn get_request(host: &str, path: &str, out: &mut [u8]) -> Option<usize> {
    let clean = |s: &str| !s.is_empty() && s.bytes().all(|b| b > b' ' && b != 0x7f);
    if !clean(host) || !clean(path) || !path.starts_with('/') {
        return None;
    }
    let parts: [&[u8]; 5] = [
        b"GET ",
        path.as_bytes(),
        b" HTTP/1.0\r\nHost: ",
        host.as_bytes(),
        b"\r\n\r\n",
    ];
    let mut at = 0;
    for part in parts {
        let end = at + part.len();
        out.get_mut(at..end)?.copy_from_slice(part);
        at = end;
    }
    Some(at)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    /// Collecting the head; `head_len` bytes of it are in `head`.
    Head,
    /// The head is read; `seen` of `length` body bytes have been handed back.
    Body { length: u64, seen: u64 },
}

/// A response being read. See the crate documentation for what it accepts. `N` is the most its
/// status line and headers may take: [`HEAD_MAX`] unless it was made with [`Response::with_room`].
///
/// # Examples
///
/// ```
/// use http_response::{Response, get_request};
///
/// let mut request = [0u8; 128];
/// let n = get_request("10.0.2.9", "/uptime-0.1.0-aarch64.nifepkg", &mut request).unwrap();
/// assert!(request[..n].starts_with(b"GET /uptime-0.1.0-aarch64.nifepkg HTTP/1.0\r\n"));
///
/// let mut response = Response::new();
/// // The reads a socket hands back need not line up with anything.
/// let body = response.feed(b"HTTP/1.0 200 OK\r\nContent-Le").unwrap();
/// assert!(body.is_empty());
/// let body = response.feed(b"ngth: 5\r\n\r\nhel").unwrap();
/// assert_eq!(body, b"hel");
/// assert_eq!(response.status(), Some(200));
/// assert!(!response.is_complete());
/// assert_eq!(response.feed(b"lo").unwrap(), b"lo");
/// assert!(response.is_complete());
/// ```
pub struct Response<const N: usize = HEAD_MAX> {
    head: [u8; N],
    head_len: usize,
    status: Option<u16>,
    state: State,
}

/// The head buffer is left out: it is up to `N` bytes of whatever the peer sent.
impl<const N: usize> core::fmt::Debug for Response<N> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Response")
            .field("status", &self.status)
            .field("state", &self.state)
            .finish_non_exhaustive()
    }
}

impl Default for Response {
    fn default() -> Self {
        Self::new()
    }
}

impl Response {
    /// A reader that has seen nothing, with room for a [`HEAD_MAX`] head.
    pub const fn new() -> Self {
        Self::with_room()
    }
}

impl<const N: usize> Response<N> {
    /// A reader that has seen nothing, with room for an `N`-byte head: `Response::<{
    /// REDIRECT_HEAD_MAX }>::with_room()` for a host whose redirects are long.
    pub const fn with_room() -> Self {
        Self {
            head: [0; N],
            head_len: 0,
            status: None,
            state: State::Head,
        }
    }

    /// **The value of the first header called `name`** (any case), once the head has been read,
    /// trimmed of spaces and tabs. `None` before the head is complete or when there is none. A
    /// package client reads `Location` here, and only on a `3xx`.
    pub fn header(&self, name: &[u8]) -> Option<&[u8]> {
        if self.state == State::Head {
            return None;
        }
        let mut headers = [httparse::EMPTY_HEADER; MAX_HEADERS];
        let mut parsed = httparse::Response::new(&mut headers);
        parsed.parse(&self.head[..self.head_len]).ok()?;
        parsed
            .headers
            .iter()
            .find(|h| h.name.as_bytes().eq_ignore_ascii_case(name))
            .map(|h| h.value.trim_ascii())
    }

    /// Take the next bytes off the connection, and return the part of them that is body.
    ///
    /// Before the head is complete this returns an empty slice; the read that completes the head
    /// returns whatever body followed it in the same read. Once the body is complete, any further
    /// byte is [`Error::BodyTooLong`]. An error is final: the reader does not recover, and neither
    /// should the caller.
    pub fn feed<'a>(&mut self, input: &'a [u8]) -> Result<&'a [u8], Error> {
        let mut rest = input;
        if self.state == State::Head {
            let before = self.head_len;
            let take = (N - before).min(input.len());
            self.head[before..before + take].copy_from_slice(&input[..take]);
            self.head_len += take;
            let Some(head) = read_head(&self.head[..self.head_len])? else {
                // No room left means no read can end it.
                if self.head_len == N {
                    return Err(Error::HeadTooLong);
                }
                return Ok(&[]);
            };
            self.status = Some(head.status);
            self.state = State::Body {
                length: head.length,
                seen: 0,
            };
            // The head ended inside this read; `before` bytes of it came from earlier ones.
            self.head_len = head.end;
            rest = input.get(head.end.saturating_sub(before)..).unwrap_or(&[]);
        }
        let State::Body { length, seen } = self.state else {
            return Ok(&[]);
        };
        let left = length - seen;
        if rest.len() as u64 > left {
            return Err(Error::BodyTooLong);
        }
        self.state = State::Body {
            length,
            seen: seen + rest.len() as u64,
        };
        Ok(rest)
    }

    /// The status code, once the head has been read.
    pub fn status(&self) -> Option<u16> {
        self.status
    }

    /// The body length the server declared, once the head has been read.
    pub fn content_length(&self) -> Option<u64> {
        match self.state {
            State::Body { length, .. } => Some(length),
            State::Head => None,
        }
    }

    /// Whether every byte the server declared has arrived. A response is only ever used when this
    /// is true; a connection that ends before it is a truncated response, whatever its digest says.
    pub fn is_complete(&self) -> bool {
        matches!(self.state, State::Body { length, seen } if length == seen)
    }
}

/// A head `httparse` found complete: where it ends, the status, and the declared body length.
struct Head {
    end: usize,
    status: u16,
    length: u64,
}

/// Parse `buf` as a status line and headers, or `None` if the blank line that ends them has not
/// arrived. `httparse` supplies the grammar; the rules that make this reader stricter are here.
fn read_head(buf: &[u8]) -> Result<Option<Head>, Error> {
    // `httparse` skips leading empty lines; a response that starts with one is not speaking HTTP.
    if matches!(buf.first(), Some(b'\r' | b'\n')) {
        return Err(Error::BadStatusLine);
    }
    let mut headers = [httparse::EMPTY_HEADER; MAX_HEADERS];
    let mut parsed = httparse::Response::new(&mut headers);
    let end = match parsed.parse(buf) {
        Ok(httparse::Status::Complete(end)) => end,
        Ok(httparse::Status::Partial) => return Ok(None),
        Err(httparse::Error::Version | httparse::Error::Status) => {
            return Err(Error::BadStatusLine);
        }
        Err(httparse::Error::TooManyHeaders) => return Err(Error::HeadTooLong),
        Err(_) => return Err(Error::BadHeader),
    };
    require_crlf(&buf[..end])?;
    let status = parsed.code.filter(|c| (100..=599).contains(c));
    let status = status.ok_or(Error::BadStatusLine)?;

    let mut length = None;
    for header in parsed.headers.iter() {
        if header.name.eq_ignore_ascii_case("transfer-encoding") {
            return Err(Error::TransferEncoding);
        }
        if header.name.eq_ignore_ascii_case("content-length") {
            let n = parse_decimal(header.value.trim_ascii()).ok_or(Error::BadContentLength)?;
            if length.is_some_and(|earlier| earlier != n) {
                return Err(Error::BadContentLength);
            }
            length = Some(n);
        }
    }
    let length = length.ok_or(Error::NoContentLength)?;
    Ok(Some(Head {
        end,
        status,
        length,
    }))
}

/// Refuse a CR that is not half of a CRLF and an LF that is not the other half, which `httparse`
/// accepts as line endings. In the status line that is [`Error::BadStatusLine`], elsewhere
/// [`Error::BadHeader`].
fn require_crlf(head: &[u8]) -> Result<(), Error> {
    let first_line_ends = head.iter().position(|&b| b == b'\n').unwrap_or(head.len());
    for (at, &byte) in head.iter().enumerate() {
        let bare = match byte {
            b'\r' => head.get(at + 1) != Some(&b'\n'),
            b'\n' => at == 0 || head[at - 1] != b'\r',
            _ => false,
        };
        if bare {
            return Err(if at <= first_line_ends {
                Error::BadStatusLine
            } else {
                Error::BadHeader
            });
        }
    }
    Ok(())
}

fn parse_decimal(digits: &[u8]) -> Option<u64> {
    if digits.is_empty() {
        return None;
    }
    digits.iter().try_fold(0u64, |n, &d| {
        if !d.is_ascii_digit() {
            return None;
        }
        n.checked_mul(10)?.checked_add(u64::from(d - b'0'))
    })
}

#[cfg(test)]
mod tests {
    extern crate std;
    use std::vec::Vec;

    use super::*;

    /// Feed `bytes` in reads of `step`, collecting the body, the way a socket would hand it over.
    fn read_in(bytes: &[u8], step: usize) -> Result<(Response, Vec<u8>), Error> {
        let mut response = Response::new();
        let mut body = Vec::new();
        for chunk in bytes.chunks(step) {
            body.extend_from_slice(response.feed(chunk)?);
        }
        Ok((response, body))
    }

    const OK: &[u8] = b"HTTP/1.0 200 OK\r\nServer: host\r\nContent-Length: 11\r\n\r\nabcdefghijk";

    #[test]
    fn a_response_reads_the_same_however_the_reads_fall() {
        for step in 1..=OK.len() {
            let (response, body) = read_in(OK, step).unwrap();
            assert_eq!(response.status(), Some(200), "step {step}");
            assert_eq!(body, b"abcdefghijk", "step {step}");
            assert!(response.is_complete(), "step {step}");
        }
    }

    #[test]
    fn a_truncated_body_is_not_complete() {
        let (response, body) = read_in(&OK[..OK.len() - 1], 7).unwrap();
        assert_eq!(body, b"abcdefghij");
        assert!(!response.is_complete());
    }

    #[test]
    fn a_body_longer_than_it_said_is_refused() {
        let mut long = OK.to_vec();
        long.push(b'!');
        assert_eq!(read_in(&long, 5).unwrap_err(), Error::BodyTooLong);
        // And a byte after completion, in a later read, is refused the same way.
        let (mut response, _) = read_in(OK, OK.len()).unwrap();
        assert_eq!(response.feed(b"x"), Err(Error::BodyTooLong));
    }

    #[test]
    fn the_body_length_must_be_stated_and_agree_with_itself() {
        let none = b"HTTP/1.0 200 OK\r\n\r\nbody";
        assert_eq!(read_in(none, 4).unwrap_err(), Error::NoContentLength);
        let twice = b"HTTP/1.0 200 OK\r\nContent-Length: 1\r\ncontent-length: 2\r\n\r\n";
        assert_eq!(read_in(twice, 4).unwrap_err(), Error::BadContentLength);
        let same = b"HTTP/1.0 200 OK\r\nContent-Length: 1\r\nCONTENT-LENGTH:1\r\n\r\nx";
        assert_eq!(read_in(same, 4).unwrap().1, b"x");
        let huge = b"HTTP/1.0 200 OK\r\nContent-Length: 99999999999999999999\r\n\r\n";
        assert_eq!(read_in(huge, 4).unwrap_err(), Error::BadContentLength);
        let signed = b"HTTP/1.0 200 OK\r\nContent-Length: -1\r\n\r\n";
        assert_eq!(read_in(signed, 4).unwrap_err(), Error::BadContentLength);
    }

    #[test]
    fn chunked_is_refused() {
        let chunked =
            b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nhello\r\n0\r\n\r\n";
        assert_eq!(read_in(chunked, 9).unwrap_err(), Error::TransferEncoding);
    }

    #[test]
    fn a_status_line_that_is_not_http_1_is_refused() {
        for bad in [
            &b"HTTP/2 200 OK\r\nContent-Length: 0\r\n\r\n"[..],
            b"HTTP/1.0 20 OK\r\nContent-Length: 0\r\n\r\n",
            b"HTTP/1.0 2000 OK\r\nContent-Length: 0\r\n\r\n",
            b"HTTP/1.0 099 Low\r\nContent-Length: 0\r\n\r\n",
            b"http/1.0 200 OK\r\nContent-Length: 0\r\n\r\n",
            b"\r\n\r\n",
        ] {
            assert_eq!(
                read_in(bad, 3).unwrap_err(),
                Error::BadStatusLine,
                "{bad:?}"
            );
        }
        // A status with no reason phrase is legal.
        let bare = b"HTTP/1.1 404\r\nContent-Length: 0\r\n\r\n";
        let (response, _) = read_in(bare, 3).unwrap();
        assert_eq!(response.status(), Some(404));
        assert!(response.is_complete());
    }

    #[test]
    fn a_header_that_is_not_a_header_is_refused() {
        let no_colon = b"HTTP/1.0 200 OK\r\nContent-Length 3\r\n\r\n";
        assert_eq!(read_in(no_colon, 3).unwrap_err(), Error::BadHeader);
        let bare_lf = b"HTTP/1.0 200 OK\r\nX: y\nContent-Length: 0\r\n\r\n";
        assert_eq!(read_in(bare_lf, 3).unwrap_err(), Error::BadHeader);
        let spaced = b"HTTP/1.0 200 OK\r\nContent Length: 0\r\n\r\n";
        assert_eq!(read_in(spaced, 3).unwrap_err(), Error::BadHeader);
    }

    #[test]
    fn a_head_that_never_ends_is_refused_at_the_bound() {
        let mut endless = b"HTTP/1.0 200 OK\r\n".to_vec();
        while endless.len() <= HEAD_MAX {
            endless.extend_from_slice(b"X-Padding: aaaaaaaaaaaaaaaa\r\n");
        }
        assert_eq!(read_in(&endless, 100).unwrap_err(), Error::HeadTooLong);
    }

    #[test]
    fn a_request_cannot_carry_a_second_line() {
        let mut out = [0u8; 256];
        assert!(get_request("host", "/a\r\nX: y", &mut out).is_none());
        assert!(get_request("host\r\n", "/a", &mut out).is_none());
        assert!(get_request("host", "a", &mut out).is_none());
        assert!(get_request("host", "/a b", &mut out).is_none());
        assert!(get_request("", "/a", &mut out).is_none());
        let n = get_request("h", "/p", &mut out).unwrap();
        assert_eq!(&out[..n], b"GET /p HTTP/1.0\r\nHost: h\r\n\r\n");
        // Too small a buffer is `None`, not a truncated request.
        assert!(get_request("h", "/p", &mut out[..n - 1]).is_none());
    }

    /// A bare CR inside a line is not a line ending and not a value: a peer that sends one is
    /// refused, in the status line and in a header alike (the pair a smuggler would use to make two
    /// parsers disagree about where a line stops).
    #[test]
    fn a_bare_carriage_return_inside_a_line_is_refused() {
        let in_header = b"HTTP/1.0 200 OK\r\nX: a\rb\r\nContent-Length: 0\r\n\r\n";
        assert_eq!(read_in(in_header, 64).unwrap_err(), Error::BadHeader);
        let in_status = b"HTTP/1.0 200 O\rK\r\nContent-Length: 0\r\n\r\n";
        assert_eq!(read_in(in_status, 64).unwrap_err(), Error::BadStatusLine);
    }

    /// The declared length is unknown until the head has been read, and is exactly the declared one
    /// after (not zero, not one).
    #[test]
    fn the_content_length_appears_with_the_head_and_not_before() {
        let mut response = Response::new();
        assert_eq!(response.content_length(), None);
        response.feed(b"HTTP/1.0 200 OK\r\nContent-Len").unwrap();
        assert_eq!(response.content_length(), None);
        response.feed(b"gth: 11\r\n\r\n").unwrap();
        assert_eq!(response.content_length(), Some(11));
        let (done, _) = read_in(b"HTTP/1.0 200 OK\r\nContent-Length: 0\r\n\r\n", 64).unwrap();
        assert_eq!(done.content_length(), Some(0));
    }

    /// A reader printed in a log says what it is and where it stands, and never the up to 2 KiB of
    /// head the peer sent.
    #[test]
    fn debug_names_the_status_and_leaves_the_head_out() {
        let (response, _) = read_in(OK, 64).unwrap();
        let shown = std::format!("{response:?}");
        assert!(shown.starts_with("Response"), "{shown}");
        assert!(shown.contains("200"), "{shown}");
        assert!(!shown.contains("Server"), "{shown}");
    }

    /// GitHub's redirect, cut down: the `Location` comes back whole and trimmed, in any case, and
    /// a header nobody sent is `None`. Before the head ends there are no headers at all.
    #[test]
    fn a_redirects_location_is_read_from_the_head() {
        let mut r = Response::new();
        r.feed(b"HTTP/1.1 302 Found\r\nDate: x\r\nlocation:  https://a.example/p?q=1 \r\n")
            .unwrap();
        assert_eq!(r.header(b"Location"), None);
        r.feed(b"Content-Length: 0\r\n\r\n").unwrap();
        assert_eq!(r.status(), Some(302));
        assert_eq!(r.header(b"LOCATION"), Some(&b"https://a.example/p?q=1"[..]));
        assert_eq!(r.header(b"Server"), None);
        assert!(r.is_complete());
    }

    /// A head of GitHub's measured size (5,291 bytes) is refused by the default reader and read by
    /// one made with [`REDIRECT_HEAD_MAX`] of room.
    #[test]
    fn a_long_head_needs_a_reader_made_with_room() {
        let mut head = Vec::from(&b"HTTP/1.1 302 Found\r\nContent-Length: 0\r\n"[..]);
        while head.len() < 5291 - 4 {
            head.extend_from_slice(b"X-Filler: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\r\n");
        }
        head.extend_from_slice(b"\r\n");
        assert_eq!(Response::new().feed(&head), Err(Error::HeadTooLong));
        let mut roomy = Response::<REDIRECT_HEAD_MAX>::with_room();
        assert_eq!(roomy.feed(&head), Ok(&b""[..]));
        assert_eq!(roomy.status(), Some(302));
    }

    /// A head that fills the reader's buffer to its last byte is accepted, so its headers must be
    /// readable too: the lookup's slice ends inside the buffer even when nothing is left over.
    #[test]
    fn a_head_that_exactly_fills_the_buffer_still_answers_header() {
        const HEAD: &[u8] = b"HTTP/1.1 302 Found\r\nLocation: /p\r\nContent-Length: 0\r\n\r\n";
        let mut r = Response::<{ HEAD.len() }>::with_room();
        assert_eq!(r.feed(HEAD), Ok(&b""[..]));
        assert_eq!(r.header(b"location"), Some(&b"/p"[..]));
    }
}
