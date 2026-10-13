# §241's prior art

An appendix of [`0241-a-threadbare-base.md`](../0241-a-threadbare-base.md), which keeps the heading.

## Prior art

- Fuchsia, read 2026-09-27 at `fuchsia.dev/fuchsia-src/concepts/packages/package`. Base packages
  "are immutable for the runtime of a system" and "must be updated with `fx ota`". Cache packages
  are on the device when flashed, "not updated during a system update, but are updated
  ephemerally". The cache set is the shape recommended here for the default set.
- Genode's Sculpt, read 2026-09-27 at `genode.org/documentation/articles/sculpt-24-10`. The boot
  image holds a GUI multiplexer, the config and report file systems, and "the most fundamental
  drivers". Everything else is fetched from the depot and "the change takes immediate effect". A new
  system image needs "reboot to activate".
- Debian Policy 2.5, read 2026-09-27. `required` is what "dpkg functionality depends on", and
  `standard` is "what will be installed by default". Here the floor is `required` and the default
  set is `standard`.
- Fedora Silverblue, from memory (the page refused the fetch). The OSTree image changes at a reboot.
  Flatpak applications live outside it and update live, and layering a package onto the image costs a
  reboot.
- ChromeOS keeps almost everything in the image, with DLC version-locked to it
  (`notes/packages-and-divisions.md`, read for §235).
