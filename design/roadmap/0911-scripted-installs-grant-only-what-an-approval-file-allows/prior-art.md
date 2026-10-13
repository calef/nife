# Unattended installs: the prior art behind the approval file

An appendix of [milestone 911 (scripted installs grant only what the owner's approval file
allows)](../0911-scripted-installs-grant-only-what-an-approval-file-allows.md), moved out of the
block whole on 2026-10-12 (UTC) by maintainer/911-rollback-rulings, under §212 (a prose budget)'s
cap. The block's "Reuse:" line carries what was taken from it.

Read 2026-10-12 (UTC) unless marked.

- Debian. `apt-get -y`: *"Automatic yes to prompts; assume "yes" as answer to all prompts and run
  non-interactively"*, aborting only on a held package, an unauthenticated one or an essential
  removal ([apt-get(8)](https://manpages.debian.org/bookworm/apt/apt-get.8.en.html)).
  `DEBIAN_FRONTEND=noninteractive` makes debconf take every default (recalled, not read). Together
  they approve everything, which is the shape the block refuses. apt's three aborts are the right
  instinct: some answers are never implied.
- Android. `pm install -g`, which `adb` runs: *"Grant all permissions listed in the app manifest"*
  ([adb](https://developer.android.com/tools/adb)). Device management goes further:
  `DevicePolicyManager.setPermissionPolicy` with `PERMISSION_POLICY_AUTO_GRANT` grants runtime
  requests without prompting, new permissions arriving with app updates included, and
  `setPermissionGrantState` sets one permission for one app (read in AOSP source, not on the
  reference page, which did not render). The auto-grant policy widens silently on update; the
  per-permission form is the approval file's shape.
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
  asking. The approval file puts the list per package rather than per capability, because a
  package is what the owner approves.
- One machine and many (recalled, not read today). apt manages one machine, and Ansible or
  unattended-upgrades drive it across many. Trusting a repository key (`signed-by`) is local apt
  configuration even when Ansible places it.

