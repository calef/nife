---
status: NOT-STARTED
raised: 2026-10-12
milestone_dependencies: 809, 892
decision_dependencies: 208, 252, 270
machine_requirements: none
specific_machine: none
needs_person: no
---
# 911. Scripted installs grant only what the owner's approval file allows

*(Minted 2026-10-12 (UTC) by lane `maintainer/mint-scripted-installs` at calef's request. The
number is provisional until the merge queue lands it; the title, the slug and every name below
(`--approved`, `jig approve`, `jig revoke`, "approval file", "grant record") are drafts.

Milestone-to-be 910 and decision-to-be 297 are dependencies as well, but their records are on #2022
and #2023, both open when this was written. `script/roadmap --check` refuses a dependency, and
`script/lint` a "milestone N" or section-sign citation, with no record on `main`. Until both land,
this block names 297, 909 and 910 by bare number and leaves them out of its frontmatter. That is a
marked exception: whoever lands the later of the two restores the citations and the
dependencies.)*

*Name: provisional, minted 2026-10-12 (UTC) by maintainer/911-rulings: `--json` and the file's
keys, after common package-tool spellings. calef names them.*

calef, 2026-10-12 (UTC): *"If the owner does approvals, how will we script installs?"* The
maintainer proposed the design below, and he answered: *"Yes, write it up as a milestone."*

**In brief.** The owner writes an approval file once. It names the packages that may be installed
and the exact grants each may receive. `jig install --approved <file> <pkg>` installs with no prompt
when the package's manifest asks for no more than the file allows, and fails closed, naming the
extra grant, when it asks for more. A script that installs on its own holds an installer endpoint
the owner delegated, badged to one approval file, so the progenitor refuses it anything outside
that file. The same file drives basalt's image composition and a lab machine's own updates, and an
interactive yes writes the same grant record, so every path to an installed package leaves one kind
of record.

## Why a plain `--yes` is refused

Three rulings of today made the install confirmation carry real authority. Under §208 (installing a
package is granting it, and the activation set is versioned), `jig install` shows everything the
install grants and one confirmation covers it. 910 (a package declares a service: install starts
it, boot starts it, update restarts it), pending on #2022, adds "runs as a service, now and at
every boot" to that list. Item 3 of milestone 892 (a std program at the prompt holds the network and
a name resolver) was ruled "c" on #2011. A manifest asks for its resolver zones, and the owner
grants them at install. Several zones are allowed, with one badge each under §252 (a resolver grant
is one zone per client badge, and a client speaks the socket contract's shape). A root-zone ask is
flagged.

A `--yes` answers that confirmation without reading it. It would approve a root-zone ask blindly.
On the next update it would approve whatever the new version asks, so a package that gained a zone,
a service or a directory would get it with nobody having seen the request. That is the one
thing the confirmation exists to prevent. The approval file keeps the owner's judgment and moves it
earlier: the owner reads the grants once, when writing the file, and a script can only replay that
answer.

## What exists, and who else touches this

Checked 2026-10-12 (UTC) on `main` and in the open pull requests #2011, #2022 and #2023. Nothing
owns pre-approved or unattended installs. Each neighbor owns a piece this milestone reads.

- Milestone 809 (the package client becomes a program, `jig`, with the verbs an index needs),
  PARTIAL, owns `install`, `remove` and `rollback`. This adds a flag and two verbs to it.
- §270 (a package manager holds an installer endpoint, not the spawn endpoint) gives `jig` a copy
  of the progenitor's spawn endpoint, badged per job and narrowed to `WRITE`. Only the owner's
  console grants it, and the kernel refuses to pass it on. Delegation here is a second badge on that
  same endpoint, not a new object.
- 910 (a package declares a service: install starts it, boot starts it, update restarts it),
  pending on #2022, owns the service grant. Milestone 892 owns the zone grant. This milestone owns
  neither; it records and compares them.
- 297 (software reaches a nife system as a package `jig` installs from basalt's index, the kernel
  included), pending on #2023, makes every program, the kernel included, an install. Every one
  of them then passes a confirmation, which is why scripting it cannot wait.
- 909 (basalt composes the bootable image from its packages), pending on #2023, composes an image
  with nobody at a prompt. It is a consumer of this file: the composition reads it, and a package
  whose manifest asks for more than the file allows fails the build.
- Milestone 858 (lab machines update themselves through packages, and only a new kernel reboots
  them) ruled in its Fork 6 that automatic updates are an owner setting. Nothing said what bounds
  such an update. This file does: an unattended update installs only within it. 858 now depends on
  this milestone.

909 and 858 are not dependencies of this block, though both use it. The edge sits on their side: 858
waits on silicon and a person, and putting it here would hold scripted installs behind a board.
909's block lives on #2023; once it lands, it gains this dependency in its own pull request.

## The approval file

The file is TOML, like the manifests of milestone 611 (every program and crate belongs to a
package), ruled by calef on 2026-10-12 (UTC) after he asked for "something more standard like JSON
or yaml". What it must hold:

- Its name, so a delegation and an error can say which file refused.
- Per package, by name:
  - the resolver zones it may receive, each one a separate entry (§252's one badge per zone), with
    `root` spelled out rather than implied;
  - whether it may run as a service (910);
  - the directories it may hold, each with its rights;
  - any other capability a manifest can ask for (the network's socket budget, the installer endpoint
    itself), by the same names `jig install`'s confirmation prints;
  - whether an update may be installed (yes or no), and whether the package may be removed.
- Nothing else. No wildcards over packages, no "any zone", and no version ranges. Which version
  installs is the question of §195 (a reviewed recipe vouches for a package, and the machine's owner
  may overrule it), not of this file.

An illustration, with provisional keys:

```toml
approval = "lab-runner"

[package.monit]
zones   = []
service = true
dirs    = { "monit/state" = "rw" }
update  = true
remove  = false

[package.vaultwarden]
zones   = ["example.org"]
service = true
dirs    = { "vault" = "rw" }
update  = true
remove  = false
```

The rule is a subset test. An install or update passes when the manifest's asks are a subset of the
file's entry for that package, and the grants given are exactly the asks. When the asks exceed the
file, the install fails before anything is fetched past the manifest, and the error names each extra
grant, for example `refused: monit 5.36 asks for zone example.net, which lab-runner does not
allow`. It never grants the intersection: a program that silently got less than it asked would fail
at run time, far from the cause. A removal passes when the entry says `remove = true`.

Only `jig` parses the file. It compiles it, on the owner's console, into canonical grant records:
one per package, sorted, in the encoding the activation set already stores (`crates/activation_set`,
§208). A human-readable file is for the owner; the progenitor compares records and never parses the
file.

## The grant record, which every path writes

The grant record is the part of an activation-set row that says what the owner granted that
package: its zones, its service, its directories and its other capabilities. It is one canonical
encoding, with no timestamp and no trace of how the answer was given. Three paths write it:

1. An interactive `jig install` at the owner's console. The owner reads the list and says yes; the
   record is the asks.
2. A scripted `jig install --approved <file>`, or a delegated holder. The asks pass the subset test;
   the record is the asks.
3. basalt's image composition (909), on the host. Each package in the image passes the same test
   against the image's approval file, and the image's first activation set holds the records.

For the same grants, the three records are byte-identical. How the answer was given is recorded
beside the record, not inside it, in a column like §270's manager column: `console`, or the
approval's name and digest. That keeps the audit question ("who allowed this?") answerable without
making two equal grants look different to anything that compares them.

## The authority is a capability

`--approved` at the owner's console needs nothing new: the owner already holds the authority, and
the file only replaces the prompt with a check that fails closed. Delegation is the part that
differs from `apt -y`, which runs with all of root's authority whatever the script meant to do.

**How a delegation is made.** At the owner's console, `jig approve <file>` prints every grant the
file contains, flags any root-zone entry the way an interactive install does, and asks once. On
yes, `jig` sends the compiled records to the progenitor on its installer endpoint as a new
activation request, `Approve`, which only an owner's (undelegated) badge may send. The progenitor
stores the approval by name and digest. Approving a name again shows only the grants changed since
the stored digest. On yes the name's records are replaced, and delegations already running under it
are checked against them from their next request, with no restart and no new endpoint. The owner
then starts the job that will install, a script or a service, with the grant "installer, within
approval X". The progenitor places an installer endpoint whose badge carries X, through §230
(badged endpoint capabilities: how a server learns which client's frame a request is in), so no
holder can forge or widen it. That grant has two forms:

- one run: the owner's console starts a script's job with it, and it ends when the job is reaped;
- standing: a service package (910) whose manifest asks for "installer, within approval X", granted
  at install like any other ask and recorded in the activation set. This is how a lab machine's
  updater (858) holds it across reboots.

**What the progenitor checks.** Every `Install`, `Remove` and `Rollback` arriving under a badge that
carries X is checked against X's stored records before anything is written. The check is in the
installer, not in `jig`, because a delegated holder is trusted to be nothing more than a holder of
that badge. `jig` runs the same test first only to give a clear error early. A holder under X cannot
send `Approve` or `Vouch`, so it can neither widen X nor make a new approval.

**How a delegation is revoked.** `jig revoke X` at the owner's console. The progenitor drops X, and
every later request under a badge that carries X is answered `Revoked` with nothing read past its
first message. Held endpoint copies die with their jobs, as §270's reply endpoints already do at
reap. Removing the service package that held a standing grant drops that grant with it. Editing the
file changes nothing until the owner runs `jig approve` again (ruled 2026-10-12).

## `jig` stays on one machine

calef, 2026-10-12 (UTC): *"maybe we are overloading jig. It should handle installs to the local
system. So adding repositories, removing repositories, installing, upgrading, removing packages,
updating its local index. There are other layers handling the automation of groups of servers,
etc."* And: *"I just want to ensure that a fleet options is possible. I don't want to define it
now."*

So `jig` covers one machine: repositories (add, remove), its local index, install, upgrade, remove,
rollback and approvals. Managing a fleet (role files, schedules, signing, fleet status) is a
separate layer, out of scope and not defined. Two hooks keep it possible:

1. A constraint: `Approve` leaves room for a later second form, the records plus an owner's
   signature checked against a key the owner trusted at the console. Nothing is built for it, and
   nothing in the format rules it out.
2. Results and refusals in a machine-readable form, such as `--json` with stable exit codes (item 7
   of the work).

## Prior art

Read 2026-10-12 (UTC) unless marked.

- Debian. `apt-get -y`: *"Automatic yes to prompts; assume "yes" as answer to all prompts and run
  non-interactively"*, aborting only on a held package, an unauthenticated one or an essential
  removal ([apt-get(8)](https://manpages.debian.org/bookworm/apt/apt-get.8.en.html)).
  `DEBIAN_FRONTEND=noninteractive` makes debconf take every default (recalled, not read). Together
  they approve everything, which is the shape refused above. apt's three aborts are the right
  instinct: some answers are never implied.
- Android. `pm install -g`, which `adb` runs: *"Grant all permissions listed in the app manifest"*
  ([adb](https://developer.android.com/tools/adb)). Device management goes further:
  `DevicePolicyManager.setPermissionPolicy` with `PERMISSION_POLICY_AUTO_GRANT` grants runtime
  requests without prompting, new permissions arriving with app updates included, and
  `setPermissionGrantState` sets one permission for one app (read in AOSP source, not on the
  reference page, which did not render). The auto-grant policy widens silently on update; the
  per-permission form is this file's shape.
- NixOS. The owner writes *"a description of the desired configuration of your system"*,
  `nixos-rebuild switch` builds and activates it, and each change is a boot entry to *"roll back to a
  previous configuration if something goes wrong"*
  ([manual](https://nixos.org/manual/nixos/stable/#sec-changing-config)). The declaration bounds
  which packages run, but a Nix package carries no grants, so it cannot bound what they may do.
- Fuchsia. Product assembly composes a system image from a product configuration naming its base
  packages ([product configuration codelab](https://fuchsia.dev/fuchsia-src/concepts/software_assembly/product_configuration_codelab)).
  Component manager's security policy holds `capability_policy` allowlists: for each routable
  capability, the component monikers that may use it
  ([`cm_config::SecurityPolicy`](https://docs.fuchsia.dev/rust/cm_config/struct.SecurityPolicy.html)).
  That routing enforces them is our reading of the `routing::policy` module, not a sentence read.
  That is the nearest model: allowlists checked by the authority that grants, not by the client
  asking. This file puts the list per package rather than per capability, because a package is what
  the owner approves.
- One machine and many (recalled, not read today). apt manages one machine, and Ansible or
  unattended-upgrades drive it across many. Trusting a repository key (`signed-by`) is local apt
  configuration even when Ansible places it.

## The work

1. The grant record: one canonical encoding in `crates/activation_set`, written by interactive
   install, and the "how it was answered" column beside it. Host tests that two paths agree.
2. The approval file, in TOML: `jig` parses it and compiles it to grant records. Host tests for
   the subset test and its error, which names every extra grant.
3. `jig install --approved <file>`, and the same flag on update and `remove`.
4. `Approve` and `Revoke` on the installer endpoint, and the badge that carries an approval, as
   §270's semantics amended in `design/decisions/` (a new method on an existing endpoint, recorded
   there per §10 (process model: capability-based, microkernel; untyped memory deferred)).
   Re-approval as ruled, and room in `Approve` for the signed form.
5. The two delegation forms: a one-run grant at spawn from the owner's console, and the standing ask
   in a service manifest beside 910's declaration.
6. A host-side check basalt's composition can call, sharing item 1's crate, so 909 can adopt it.
7. A machine-readable form of `jig`'s results and refusals (`--json`), with documented, stable
   exit codes. A refusal names each extra grant as data.

## Done when

Under QEMU on aarch64, riscv64 and x86_64, in `script/swish-check`, with fixture packages:

1. A scripted install with a matching approval file succeeds with no prompt, and the package runs
   with exactly its asked grants.
2. An update whose manifest adds a zone fails, names the zone, and leaves the previous version
   installed and running.
3. A delegated installer endpoint cannot install a package outside its file, nor a listed package
   asking more than its entry, nor send `Approve`. After `jig revoke`, it can install nothing.
4. The grant record from an interactive yes is byte-identical to the scripted one for the same
   grants.
5. An edited file changes nothing until `jig approve` runs again, which shows only the changed
   grants; a delegation already running then installs within the new records, with no restart and
   no new endpoint.
6. A refused scripted install exits with its documented code, and `--json` names each extra
   grant.

Reuse: §270's badge and activation request, `crates/activation_set`, and 910's and 892's grant
encodings, extended rather than duplicated. The file's design takes Fuchsia's checked-by-the-granter
allowlist and Android's per-permission grant state; debconf preseeding was read as the shape that
pre-answers questions without bounding them.

## Ruled 2026-10-12 (UTC)

calef answered "Yes, record them", recorded on #2030, and the body now carries each. Open question
1 is (a): an approval binds to the file's digest at `jig approve`. The file is TOML. `jig` keeps to
one machine.

## Open questions for calef

2. **May a delegated holder roll back?** (a) No; rollback stays the owner's. (b) Yes, when every row
   the previous generation restores lies within its approval. *Recommended: (b).* 858's updater must
   undo a bad update with nobody present, and a rollback inside the file grants nothing the owner
   did not approve. (a) would leave a lab machine on a broken package until calef reached it.
3. **Does `jig rollback` undo an approval?** (a) Approvals live beside the generations, so a rollback
   of packages leaves them alone. (b) They are rows in the versioned set and roll back with it.
   *Recommended: (a).* An approval is the owner's standing word, not a state of the machine; rolling
   back a bad package should not silently narrow or widen what a script may do.

## BUGS

- Nothing is built. Item 3 of 892 and 910 are themselves unbuilt, so the grants this file
  bounds do not yet exist at the prompt.

## Index row

Scripted installs without a blind yes. The owner approves packages and their exact grants once, in a
file. `jig install --approved` and a delegated, badged installer endpoint install only within it and
fail closed, naming the extra grant; basalt's image and lab updates use the same file.
