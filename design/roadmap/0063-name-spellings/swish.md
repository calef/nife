# Milestone 63: `swish`, the shell's name (calef, 2026-08-01)

Moved on 2026-10-12 (UTC) from [`design/roadmap/0063-name-spellings.md`](../0063-name-spellings.md)
under §212 (a prose budget), where it was the section "`swish`: the shell has a name now". The page
holds the standard, the renames and the rest of the naming record. Three bold spans became plain
text and one long sentence was split in moving.

`shell` is a category, not a name. `bash`, `zsh`, `fish` and `rc` are names; this project's most
demonstrable artifact was filed under the noun for what kind of thing it is.

`capsh` was the obvious candidate and is unavailable. Linux's libcap ships `capsh(1)`, a
"capability shell wrapper" for testing POSIX capabilities, which is adjacent enough that a reader who
knows Linux capabilities would assume ours is that tool.

Why `swish` rather than something descriptive. Shell names are identities rather than
descriptions: `fish` describes nothing and nobody minds. But this one happens to carry the thesis
anyway, which is the combination shell names almost never manage.

A swish is the basketball shot that goes through the net touching nothing. That is least
authority in one word: the command reaches exactly what it designated and nothing else. `wc
report.txt` touches `report.txt` and not one thing more, and it does so structurally rather than by
a check that could be wrong.

It also reads as a shell on sight, because the `sh` is built in, the same trick `bash` plays with a
pun.

`sheesh` was considered and set aside on two grounds, both recorded because they are the kind of
thing that is obvious only once said. It carries a timestamp: the word spiked as a meme around
2020-21, where `bash` and `fish` are era-neutral, and this project expects to be shown off years from
now. And *sheesh* is an interjection of exasperation, while this shell's most characteristic
behavior is refusing things by design. The name and the experience would have pointed the same
direction, and "the shell that says no" reading as a complaint is a risk a name should not carry for
free. `swish` inverts both: a precision word on a precision property.

Two wrinkles, neither disqualifying. Sweden's mobile payment system is called Swish (different
domain, no confusion in a terminal). And `swish` contains "wish", which is faintly the wrong idea for
a system where you do not ask for authority, you hold it.
