---
status: PROPOSED
raised: 2026-10-17
---

# 306. The web's authority list is a package and a capability, and `jig`'s own channel never depends on it

Proposed for calef to rule on. Raised 2026-10-17 (UTC) while milestone 809 (the package client
becomes a program) was settling `jig add-repository`. calef ruled that verb takes just a URL, then
asked why nife does not ship the usual authority list, and asked for the general-program answer to
be opened as a decision.

## The question

A general HTTPS program on nife (a future `curl`, a browser at the prompt) must decide which
certificate authorities to believe for a host nobody named in advance. §196 (nife carries TLS) clause
4 holds roots per source and forbids a system-wide trust store, for the package client. That
reasoning does not extend to a program whose job is any host. What does such a program use, and does
the answer put a circularity back into `jig`?

## What is in force

- §196 clause 4: roots per source, no system store. The reason given is circularity: a store must
  update independently of the system, and the thing that updates it is the package manager.
- §250 (an image names its distribution's package index) clauses 3, 10 to 13: `jig`'s channel to `basalt.nifeos.org` is pinned to ISRG Root X1, and
  an image carries one further root, Sectigo E46, for GitHub Release downloads. The pin is compiled
  into the program (`PinnedPeer::basalt_index`, milestone 501 (a TLS client that speaks to one pinned peer)), and the client has no root store to
  add a second root to.
- §195 (a reviewed recipe vouches for a package): a digest in a reviewed recipe decides what runs, so TLS buys confidentiality and knowing
  which host answered, not authenticity.
- §208 (installing a package is granting it): installing a package is granting it; its data is a read-only directory a session binds by
  name.
- milestone 387 (a TLS stack, and the fork is not "OpenSSL or not"): "a trust store should not be ambient here": a client granted exactly the roots its
  one peer chains to cannot be induced to trust anything else.
- No program that browses or fetches an arbitrary host exists. `pinned_tls_client` takes one name
  and one root by construction.

## The proposal

1. **The web's authority list is an ordinary basalt package.** Its content is Mozilla's CA list, the
   one most systems ship. It is versioned, vouched by digest under §195, and installed and updated
   by `jig` like any other package.
2. **It is a capability, not ambient.** A program gets it only if its manifest names it and its
   grant plan allows it, the same shape as the network and directory grants. `ls` never holds it. A
   program that talks to one host keeps a pinned root and does not ask for the list.
3. **It updates through `jig update` over basalt's pinned channel**, so a distrusted authority
   leaves at the next update, not the next image.
4. **`jig`'s channel to basalt never depends on anything `jig` installs.** It stays pinned to the
   image's roots (ISRG Root X1 and Sectigo E46 today) and never consults the list. This is the rule
   that removes §196's circularity instead of walking around it: the list depends on the channel, the
   channel never depends on the list. A distrusted authority can never be the thing that breaks
   updating the list.

Tooling is not written here. The list's package, the manifest field that names it and the
verifier that consumes it are a milestone's work after a ruling.

## Fork for milestone 809: which roots check an index that `add-repository` adds

Stated as open. Not decided by this proposal, and independent of clauses 1 to 4 except that options Y and Z
need the list to exist.

calef ruled 2026-10-17 (UTC) that `jig add-repository <url>` takes just a URL ("Just a URL."), and
that an added index is checked against the image's pinned roots. Once the list exists there are
three readings.

| | An added index's TLS chain is checked against | Reach | Cost |
|---|---|---|---|
| X. The image's roots (today) | ISRG Root X1 and Sectigo E46 | Only a host serving a chain to one of those two. Most small hosts qualify through Let's Encrypt; a company's own authority does not | An owner whose index is on any other authority cannot add it with a URL alone, which re-opens milestone 809's "a second index needs a root the owner supplies" |
| Y. The list | the web's authority list | Anything a browser would open | An added index is trusted at the channel exactly as a website is, by about 120 authorities, and the owner cannot narrow it per source |
| Z. Either | the image's roots first, then the list | As Y | The chain that passed is not recorded in the URL, so what authenticated a source varies by host; the weakest of the two is the real check |

Under Y or Z, §195's digests still decide what runs, but only once the added index's own signing
root is trusted, which is milestone 809's open question 1 and is not settled here. Not checked: how a URL-only `add-repository` obtains the added repository's
TUF root. The TLS check authenticates the host, not the repository's signing root, and nothing read
for this proposal says where the second comes from.

## Alternatives to clauses 1 to 3

| | Shape | Cost | What others do |
|---|---|---|---|
| A. A frozen store in the image | The list ships in the image and is readable by programs that ask | Frozen until the next image, so a distrusted authority persists on every installed machine (DigiNotar 2011 and DST Root X3 2021 are the cases §387 cites). Ambient if every program may read it | Debian (`ca-certificates`), Fuchsia and OpenBSD (`/etc/ssl/cert.pem`) ship a store, as do macOS and Windows. Debian updates it as a package; the others with the OS (recalled, not read) |
| B. Per-program pinned root only | No list; each program carries the one root for its one peer | Nothing to curate. Cannot serve a program whose host is chosen by a person or a page | Matches §196 and `jig`. No system in the survey above ships without a store for general programs |
| C. No general HTTPS client | Nothing on nife fetches an arbitrary host | Cheapest, and forecloses every program that is not a package client, including the real workload with a real user that `AGENTS.md` principle 1 ranks first | Every system named above ships a store because it runs a browser or `curl` (recalled) |
| This proposal | Package plus capability | A curation burden for basalt, a freshness requirement, and one more grant to explain | The same data as A, with the update path and the scope that Unix lacks |

The lines marked recalled were not checked against any source for this proposal; they are priors,
not findings. Fuchsia's and OpenBSD's layouts in particular were not read.

## What was measured, 2026-10-17 (UTC), and what was not

- Mozilla's list, as extracted by curl (`curl.se/ca/cacert.pem`, last modified 2026-09-25 03:12 GMT):
  121 roots, 129,143 bytes of DER, 188,900 bytes of PEM. This corrects the "roughly 150" and
  "~150" in milestone 387 and the dependency census, which were recalled in 2026-09. The list is
  small enough that carrying it as package data is not a size question.
- The local `/etc/ssl/cert.pem` on the maintainer's Mac (OpenBSD's file, `$OpenBSD: cert.pem,v 1.22.2.1
  2021/09/30`) holds 128 roots in 333,483 bytes. It is a 2021 snapshot on a different vendor's
  curation, so it measures only that the figure is "about 120 to 130", not what is current.
- The curl extract contains no distrust-after dates and no name constraints (a grep for both finds
  nothing). Mozilla's own `certdata.txt` carries them for partial distrust. A package built from the
  curl extract would lose that, so the package's source must be the NSS data or a derivative that
  keeps them. Not checked: which of `webpki-roots` or a direct `certdata.txt` build preserves them
  in the form `rustls-webpki` consumes.
- Not checked: whether `rustls-webpki` as `pinned_tls_client` uses it accepts 121 anchors without
  a size or time cost worth measuring. Milestone 501's costs are for one root. No build was run for
  this proposal.
- Not checked: how long Mozilla's removals take to reach a basalt release in practice, which is
  clause 3's actual latency. The owner decides whether and when a machine updates (§250's 2026-10-07
  amendment), so a machine that does not update keeps a distrusted authority; the list leaves "at the
  next update" only on a machine that takes one.

## What it costs, and what it does not

- Basalt owns a package whose content is a security decision for every machine that grants it. The
  recipe is reviewed (§195 clause 1), but the review is of Mozilla's change, not of each authority.
- Clause 4 means two root sets coexist: the image's two roots, frozen until the next image, and the
  list. §250's BUGS already record the frozen set's problem: X1's cross-signatures expire 2032-09-02,
  so the image's pin has its own rotation. This proposal does not solve that and does not make it
  worse.
- Granting the list to a program lets it trust any listed authority for any host. The grant is the
  scope. A browser is exactly the program that needs the wide grant, so it is flagged like a
  root-zone resolver ask (milestone 892 (a std program at the prompt holds the network and a name resolver), "Item 3, ruled").

## Reversibility, and who has acted on it

No program asks for the list and no image carries one, so nothing depends on it. Clauses 1 to 3 are a
package and a manifest field: reversible. Clause 4 is a rule about what `jig` may read, and it only
narrows. The irreversible part is the fork: if milestone 809 builds option Y, owners who add
indexes by URL are trusting the list, and moving back to X breaks the indexes they added.

## The equal-cost test

Would the proposal be chosen if A, B and the proposal cost the same? Yes over A: a store the next
image alone can change is the circularity §196 named, restated. Yes over C if the project wants any
program beyond `jig` to reach the web. C is cheapest and says no. Over B alone the answer depends on
whether such a program is wanted. That is a question about the project, not about effort. The
fork's options X, Y and Z are not about effort: they differ in who `add-repository` trusts.

## What a ruling unblocks

- A milestone to build the package, the manifest field and the verifier, which any browser or `curl`
  waits on.
- Milestone 809's `add-repository` has an answer to its open question 1 (item 7) under whichever of
  X, Y or Z is ruled.

## What I need from calef

1. Adopt clauses 1 to 4, or choose A (frozen store), B (per-program pin only) or C (no general
   client) instead.
2. For `add-repository`: X (image roots), Y (the list) or Z (either).
3. If clauses 1 to 4 are adopted: the list's source must keep Mozilla's partial-distrust data. Say
   whether that is a requirement of the package's recipe.

## BUGS

- The measurements above are one day's, taken from a third-party extract. The figures move with
  every Mozilla release.
- No code path in the tree uses a root list, so none of this has been run.
