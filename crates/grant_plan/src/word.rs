//! **Quoting: what a word is, and therefore what a command designates** (milestone 67,
//! notes/swish-language.md).
//!
//! In a shell where naming a resource is granting it, a name you cannot type (`my notes.txt`) is a
//! resource you cannot grant. Quoting is what makes such names nameable.
//!
//! # Quoting delimits a word. It never rewrites one.
//!
//! Every token in this shell is a **slice of the line you typed**, which is what lets a crate with
//! no allocator hand a name straight to the grant planner (and `line::split` keep a stage's command
//! text as a slice, notes/pipes.md). A backslash escape would remove a byte from the middle of a
//! word, and a word with a byte removed is not a slice of anything. So:
//!
//! - `'...'` and `"..."` **wrap a whole word**, and the word is the span between them.
//! - There is **no backslash escape**. A `\` is an ordinary byte in a name.
//! - Two quoted pieces are **not joined**. `a"b c"d` is [`Refusal::PartlyQuoted`].
//!
//! So **what is between the quotes is exactly what is designated, byte for byte**, and a preview
//! (`caps`, `echo`) and a grant cannot disagree about what a name is.
//!
//! # What quoting does and does not change about authority
//!
//! It changes a word's boundaries, so a name with a space, `>` or `|` can be named at all. It
//! **suppresses pattern expansion**, so `rm "*.txt"` designates one name and `rm *.txt` the set, a
//! narrowing visible before anything moves (`echo` prints both). Nothing else: no quoting form
//! widens a grant or skips a manifest check; `"secret.txt"` is planned exactly as `secret.txt` is.
//!
//! `'` and `"` are both literal today. POSIX distinguishes them by `$` expansion, and this shell
//! has no variables (milestone 47 (navigation and naming) owns them), so the distinction waits for them. They differ only in
//! which quote byte they let you write: `echo "it's"` and `echo 'say "hi"'`.
//!
//! # BUGS
//!
//! - **No backslash escape, and none is planned while tokens are slices.** `my\ notes.txt` names a
//!   file whose name contains a backslash, and `nav::component_fits` refuses that byte, so the line
//!   is refused rather than misread.
//! - **Adjacent pieces are not joined.** `a"b"` and `'it''s'` are [`Refusal::PartlyQuoted`] where
//!   POSIX would join them. Write `"it's"`.
//! - **`"$?"` prints `$?`**, because both forms are literal. When variables arrive the two forms
//!   have to stop being the same thing, and that decision belongs with them.

use crate::Refusal;

/// The two quote characters. Named because three files scan for them and a bare `b'\''` in a match
/// arm is the sort of thing that gets typed as `b'"'` once.
pub const SINGLE: u8 = b'\'';
/// See [`SINGLE`].
pub const DOUBLE: u8 = b'"';

/// Which quote a [`Cursor`] is inside.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Quote {
    /// Not inside a quoted span. An operator here is an operator.
    #[default]
    Bare,
    /// Inside `'...'`.
    Single,
    /// Inside `"..."`.
    Double,
}

/// **A cursor that says, byte by byte, whether what it is looking at is an operator or a letter.**
///
/// Three scanners need this and none of them can share a tokenizer: [`crate::tokenize`] splits on
/// whitespace, [`crate::line::split`] splits on `>`, `<` and `|`, and the shell's sequence splitter
/// splits on `;`, `&&` and `||`. What they have in common is the question "is this byte quoted",
/// which is a two-bit state machine, so that is what is shared.
///
/// ```
/// use grant_plan::word::Cursor;
///
/// // `echo "a > b"` has no redirection in it: the `>` arrives while the cursor is open.
/// let line = b"echo \"a > b\"";
/// let mut c = Cursor::new();
/// let mut redirects = false;
/// for &b in line {
///     if c.step(b) && b == b'>' {
///         redirects = true;
///     }
/// }
/// assert!(!redirects);
/// assert!(!c.is_open(), "the quote was closed");
/// ```
#[derive(Clone, Copy, Debug, Default)]
pub struct Cursor {
    quote: Quote,
}

impl Cursor {
    /// A cursor at the start of a line, outside any quote.
    pub fn new() -> Self {
        Self::default()
    }

    /// **Feed the byte at the cursor and say whether it is bare.**
    ///
    /// `true` means the byte is outside every quote and is not itself a quote character, so a
    /// caller scanning for operators may act on it. `false` means it is quoted text or the quote
    /// itself, and the caller must treat it as an ordinary byte of the word.
    pub fn step(&mut self, b: u8) -> bool {
        match (self.quote, b) {
            (Quote::Bare, SINGLE) => {
                self.quote = Quote::Single;
                false
            }
            (Quote::Bare, DOUBLE) => {
                self.quote = Quote::Double;
                false
            }
            (Quote::Single, SINGLE) | (Quote::Double, DOUBLE) => {
                self.quote = Quote::Bare;
                false
            }
            (Quote::Bare, _) => true,
            _ => false,
        }
    }

    /// Whether the cursor is inside a quoted span right now.
    ///
    /// A whitespace byte never changes this state, so a scanner skipping whitespace can consult
    /// this instead of stepping, which is what [`crate::tokenize`] does.
    pub fn is_open(&self) -> bool {
        self.quote != Quote::Bare
    }
}

/// **A token with its quotes taken off, and whether they were there.**
///
/// The flag is not decoration. It is what stops `rm "*.txt"` from being expanded, and it is
/// therefore part of what the line designates rather than part of how it was spelled.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Word<'a> {
    /// The bytes the token designates: the span between the quotes, or the whole token when it had
    /// none. Always a slice of the line.
    pub text: &'a [u8],
    /// Whether quotes were what made this word a word. A quoted word is never a pattern, whatever
    /// bytes are in it.
    pub quoted: bool,
}

impl<'a> Word<'a> {
    /// A word nobody quoted. The shape every token had before this module existed.
    pub fn bare(text: &'a [u8]) -> Self {
        Word {
            text,
            quoted: false,
        }
    }
}

/// **Take the quotes off one whole token.**
///
/// The token is what a quote-aware tokenizer already produced, so the quotes (if any) are on the
/// ends. Three shapes are legal and everything else is refused:
///
/// - no quote byte anywhere: the token is the word,
/// - a matching pair on the ends with no copy of itself inside: the word is what is between them,
/// - and that is all.
///
/// Refusing rather than guessing is the same rule the rest of this crate follows for an unplaceable
/// token: **the line means exactly what it says or it does not run.** A shell that read `a"b"` as
/// `ab` would be joining pieces, and a shell that read it as the four literal bytes would be
/// silently disagreeing with every other shell a person has used.
///
/// # Examples
///
/// ```
/// use grant_plan::word::read;
///
/// // The gap this closes: a name with a space in it.
/// assert_eq!(read(b"\"my notes.txt\"").unwrap().text, b"my notes.txt");
/// // Quoted, so nothing will expand it.
/// assert!(read(b"'*.txt'").unwrap().quoted);
/// // Bare, so it is a pattern and designates a set.
/// assert!(!read(b"*.txt").unwrap().quoted);
/// // The other quote is an ordinary byte inside a quoted word.
/// assert_eq!(read(b"\"it's\"").unwrap().text, b"it's");
/// ```
pub fn read(token: &[u8]) -> Result<Word<'_>, Refusal> {
    let Some(&first) = token.first() else {
        return Ok(Word::bare(token));
    };
    if first != SINGLE && first != DOUBLE {
        return if token.contains(&SINGLE) || token.contains(&DOUBLE) {
            Err(Refusal::PartlyQuoted)
        } else {
            Ok(Word::bare(token))
        };
    }
    let inner = &token[1..];
    match inner.iter().position(|&b| b == first) {
        // The closing quote is the last byte, so the pair wraps the whole token.
        Some(at) if at + 2 == token.len() => Ok(Word {
            text: &token[1..token.len() - 1],
            quoted: true,
        }),
        // It closes early, so something outside the quotes would have to be joined on.
        Some(_) => Err(Refusal::PartlyQuoted),
        None => Err(Refusal::UnclosedQuote),
    }
}

/// **Read the token starting at `line[i]`, quotes included, and say where it ends.**
///
/// Used where a scanner is already mid-line and cannot re-tokenize: a redirection's name
/// ([`crate::line::split`]) and `echo`'s words. It stops at the first bare byte that `stop` says
/// ends a token, which lets the redirection scanner stop at an operator and `echo`'s stop at
/// whitespace only.
///
/// The returned index is one past the token. An unterminated quote runs to the end of the line and
/// is [`Refusal::UnclosedQuote`] when [`read`] looks at it, so this function does not have to
/// decide anything.
pub fn span(line: &[u8], i: usize, stop: &dyn Fn(u8) -> bool) -> usize {
    let mut c = Cursor::new();
    let mut j = i;
    while j < line.len() {
        if !c.is_open() && stop(line[j]) {
            break;
        }
        c.step(line[j]);
        j += 1;
    }
    j
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_word_with_no_quote_in_it_is_the_word_it_always_was() {
        for t in [&b"report.txt"[..], b"*.txt", b"--mem", b"a\\b", b""] {
            let w = read(t).expect("no quotes to get wrong");
            assert_eq!(w.text, t);
            assert!(!w.quoted, "{:?} was not quoted", core::str::from_utf8(t));
        }
    }

    #[test]
    fn a_wholly_quoted_word_is_the_span_between_the_quotes() {
        assert_eq!(read(b"'my notes.txt'").unwrap().text, b"my notes.txt");
        assert_eq!(read(b"\"my notes.txt\"").unwrap().text, b"my notes.txt");
        // The empty word, which is a word: `echo ""` prints nothing and is not `echo` with no
        // operand. Whether anything downstream can *use* it is the manifest's question.
        assert_eq!(read(b"''").unwrap().text, b"");
        assert!(read(b"''").unwrap().quoted);
    }

    #[test]
    fn the_other_quote_is_an_ordinary_byte_inside_a_quoted_word() {
        // The whole of what the two forms differ in today.
        assert_eq!(read(b"\"it's\"").unwrap().text, b"it's");
        assert_eq!(read(b"'say \"hi\"'").unwrap().text, b"say \"hi\"");
    }

    /// **A pattern that was quoted is not a pattern**, which is the flag's whole job: it is what
    /// makes `rm "*.txt"` designate one name and `rm *.txt` designate a set.
    #[test]
    fn quoting_is_carried_because_it_decides_what_the_word_designates() {
        assert!(read(b"'*.txt'").unwrap().quoted);
        assert!(!read(b"*.txt").unwrap().quoted);
        // And the text is the same either way, which is the point: nothing was rewritten.
        assert_eq!(read(b"'*.txt'").unwrap().text, read(b"*.txt").unwrap().text);
    }

    #[test]
    fn a_quote_that_never_closes_is_refused() {
        assert_eq!(read(b"'my notes.txt"), Err(Refusal::UnclosedQuote));
        assert_eq!(read(b"\"abc"), Err(Refusal::UnclosedQuote));
        assert_eq!(read(b"'"), Err(Refusal::UnclosedQuote));
    }

    /// The corner this design costs, refused loudly rather than misread. Joining `a` and `b c` into
    /// one word means building a byte string, and every token here is a slice of the line.
    #[test]
    fn pieces_are_never_joined() {
        assert_eq!(read(b"a\"b\""), Err(Refusal::PartlyQuoted));
        assert_eq!(read(b"\"a\"b"), Err(Refusal::PartlyQuoted));
        assert_eq!(read(b"'it''s'"), Err(Refusal::PartlyQuoted));
        // A bare word carrying a stray quote is the same mistake seen from the other side.
        assert_eq!(read(b"don't"), Err(Refusal::PartlyQuoted));
    }

    #[test]
    fn a_cursor_reports_quoted_bytes_as_not_bare() {
        let mut c = Cursor::new();
        let mut bare = Vec::new();
        for &b in b"a'b'c" {
            bare.push(c.step(b));
        }
        // `a` and `c` are bare; the quotes and the `b` between them are not.
        assert_eq!(bare, [true, false, false, false, true]);
        assert!(!c.is_open());
    }

    #[test]
    fn a_span_stops_at_a_bare_delimiter_and_not_at_a_quoted_one() {
        let line = b"date > 'my out.txt' extra";
        let at = 7;
        let end = span(line, at, &|b| b.is_ascii_whitespace());
        assert_eq!(&line[at..end], b"'my out.txt'");
        // ... and the same scan on a bare word stops at the first space.
        let line = b"date > my out.txt";
        let end = span(line, 7, &|b| b.is_ascii_whitespace());
        assert_eq!(&line[7..end], b"my");
    }

    extern crate std;
    use std::vec::Vec;
}
