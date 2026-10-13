---
status: NOT-STARTED
raised: 2026-10-12
milestone_dependencies: 755, 607, 611, 810, 911
decision_dependencies: 297, 235, 241, 250
machine_requirements: none
specific_machine: none
needs_person: no
---
# 909. basalt composes the bootable image from its packages

*(Minted 2026-10-12 (UTC) by lane `maintainer/foreign-programs-via-jig`, under §297 (software
reaches a nife system as a package `jig` installs from basalt's index, the kernel included). The
number is provisional until the merge queue lands it; the title and slug are drafts.)*

calef, 2026-10-12 (UTC): *"The image should be composed from basalts packages."* The programs a
system needs before `jig` runs reach it this way. They are the kernel, the progenitor, the shell,
`jig` and the network stack.

## Where it starts

- Milestone 755 (basalt v0 pins nife and runs its gate) is built. basalt pins a nife commit and runs
  that commit's own image build and tests. The image it tests is built in nife's tree, from binaries
  nife compiled.
- Milestone 607 (nife is built and updated from packages, and the repository divides along what
  releases together) planned this step as its item 1: the image is assembled from base packages by
  digest, on the host. It did not say who assembles it. §297 says basalt does.
- Milestone 611 (every program and crate belongs to a package, and every package has a home) draws
  the package boundaries this build reads.
- Milestone 810 (`ripgrep` is packaged in basalt and installed with `jig`) builds basalt's first
  package. This milestone extends that build to the base.
- Milestone 911 (scripted installs grant only what the owner's approval file allows) gives the
  composition the check it runs with nobody at a prompt. Each package's asks are tested against the
  image's approval file, as the same grant records an interactive install writes. 911's block names
  this dependency and leaves the edge here.

## The work

1. basalt builds every base package from the pinned nife commit: the kernel, the progenitor, the
   shell, `jig` and the network stack. The rest of the base is what §241 (a threadbare base: the
   boot slot holds the kernel and what boots and repairs, and every other program is a package)
   names.
2. basalt composes each architecture's image from those packages, by digest, as 607's item 1
   describes. The image names basalt's index (§250 (an image names its distribution's package
   index, at `basalt.nifeos.org`, and a package's bytes may live anywhere)).
3. Each package in the image passes 911's host-side check against the image's approval file, and
   the image's first activation set holds the resulting grant records. A package whose manifest
   asks for more than the file allows fails the build, naming the extra grant.
4. basalt's gate boots the composed image rather than the one nife's tree built.
5. nife's own image build stays for nife's tests, composed from the same package definitions, so a
   nife change can be tested before basalt pins it.

## Done when

Under QEMU on aarch64, riscv64 and x86_64, basalt's gate boots an image it composed from its own
packages and reaches the swish prompt. `script/swish-check`'s rows pass on that image. No binary in
the image came from nife's tree build rather than from a package basalt built.

Reuse: 607's design for assembly by digest, 755's gate and 810's package build, extended rather
than replaced.

## BUGS

- The image's trust root still pins the base in the kernel until 607's item 6 lands, so a base
  package change still relinks the kernel.

## Index row

The bootable image is composed by basalt from the packages it builds, so the kernel and the base
reach a system the same way every other program does.
