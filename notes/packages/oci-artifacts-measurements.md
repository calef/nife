# Serving packages as OCI artifacts, measured

*An appendix to [container-format-measurements.md](container-format-measurements.md), measured by
the same lane. Name: provisional (2026-10-18).*

This is about where a package is stored and how it is served, not what is inside it: the payload
stays one of the four files the parent note measures. Homebrew serves its bottles this way from ghcr.io, GitHub's
Open Container Initiative (OCI) registry. Measured 2026-10-17 and 2026-10-18 (UTC) against a local
`registry:2.8.3` container with oras 1.2.3, and against Homebrew's public `ripgrep` bottle on
ghcr.io. Nothing was pushed to ghcr.io.

## One package as an artifact

`oras push` of basalt's `ripgrep-14.1.1-aarch64.nifepkg` gave a 613-byte manifest:

| part | media type | content |
|---|---|---|
| manifest | `application/vnd.oci.image.manifest.v1+json` | `artifactType` `application/vnd.nifeos.package.v1`, one config, one layer, a `created` annotation |
| config | `application/vnd.oci.empty.v1+json` | the 2-byte `{}`, inlined in the manifest |
| layer (the blob) | `application/vnd.nifeos.package.nifepkg1` | the package file, byte for byte, with its file name as `org.opencontainers.image.title` |

The two `vnd.nifeos` media types are this measurement's own and name nothing in force.
`oras pull` returned a byte-identical file. Homebrew's bottle has the same shape with one more
level: a tag resolves to an image index of 7 per-platform manifests, each with one
`tar+gzip` layer.

## What a client must do

Every response below carried `Content-Length` and answered an HTTP/1.0 request, which is what
`http_response` requires.

| step | request | answer |
|---|---|---|
| 1 | `GET https://ghcr.io/token?scope=repository:<name>:pull&service=ghcr.io` | 200, JSON `{"token": ...}`, 73 bytes; needed even for an anonymous pull (a blob `GET` without it is 401) |
| 2 | `GET /v2/<name>/manifests/<tag>` with `Accept` and `Authorization: Bearer` | 200, the manifest (6,504 bytes for one Homebrew platform; 28,732 for its index) |
| 3 | `GET /v2/<name>/blobs/sha256:<digest>` | 307 to `pkg-containers.githubusercontent.com`; the head is 849 bytes, its `Location` 484 |
| 4 | `GET` the `Location` | 200, the blob; its SHA-256 matched the digest |

Step 2 is skippable. A recipe already holds the blob's digest, and step 3 fetched Homebrew's bottle
by digest with no manifest read. Today's GitHub Release fetch is two connections (the asset URL and
its one redirect). By digest, a registry is three; through a tag, four. Each is its own TLS
connection under `http_response`'s one request per connection.

What that adds to `jig`, which parses no JSON today:

| piece | measured | what it is |
|---|---|---|
| `Accept` and `Authorization` request headers | not measured | two header lines in the request writer |
| the token reply, by `serde-json-core` 0.7.0 | 10,276 `.text` at opt 3, 9,560 at `s` | MIT OR Apache-2.0, `no_std` and allocation-free, released 2026-10-08, 4.6 million downloads; this figure also parses the manifest below |
| the manifest, by `serde_json` 1.0.154 instead | 33,988 at opt 3, 29,980 at `s`, plus 25,146 `.rodata` | MIT OR Apache-2.0, needs an allocator, which `jig` has |
| following a 307 to another host | already built | the release fetch follows one redirect, with a 5,291-byte head |

Both JSON figures come from the same harness as the parent note's reader table, parsing the token and a
one-layer manifest into borrowed fields. They are on the bare-metal target, not `jig`'s.

## Digests and trust

- The layer digest is the SHA-256 of the package file. For ripgrep aarch64 it is
  `sha256:f8a03aa8…71f8`, the same string basalt's `recipes/ripgrep.toml` records, so §195 (a
  reviewed recipe vouches for a package)'s digest is the blob's address. Homebrew's bottle carries its own `sh.brew.bottle.digest`, and that equals
  its layer digest too.
- The manifest's digest is a different hash, over the manifest's JSON. oras writes the push time
  into `created`, so pushing the same file twice gave two manifest digests. With `created` fixed,
  two pushes agreed. `oras cp` to another repository kept the manifest digest.
- §306 (the web's authority list is a package and a capability) rule 4 pins `jig`'s channel to the
  image's two roots, ISRG Root X1 and Sectigo E46. ghcr.io serves an RSA chain ending at Sectigo
  Public Server Authentication Root R46, cross-signed by USERTrust RSA, and refuses an ECDSA-only
  handshake. Neither root is one the image carries. The blob host chains to ISRG Root X1, which it
  does carry. A basalt index on ghcr.io needs a third image root.
- For an added repository, rule 5 checks against the web list. Mozilla's list (`curl.se`'s
  extract, fetched the same day, 121 roots) holds both Sectigo R46 and USERTrust RSA.

## What it buys and what it costs

| | registry (ghcr.io or any OCI registry) | GitHub Releases, today |
|---|---|---|
| hosting cost | free for public packages, storage and bandwidth (GitHub's billing docs, read 2026-10-18) | free |
| mirrors | any registry speaking the distribution spec; `oras cp` kept every digest | any static HTTPS host |
| tools that fetch it | oras, skopeo, crane; `curl` with the token dance above | `curl`, a browser |
| stock-tool inspection (macOS, Ubuntu images) | `curl` only, by hand; none of oras, skopeo or crane ships with either | `curl` |
| client cost in `jig` | the token step, a JSON parser, two headers, a third root for ghcr.io | built |
| lock-in | the protocol is open; ghcr.io's token realm and its blob host are GitHub's | the URL layout and the redirect are GitHub's |

## What this did not measure

- A push to ghcr.io, its anonymous rate limits, and skopeo or crane themselves.
- A registry client in `jig`; the JSON figures are the parser alone.
