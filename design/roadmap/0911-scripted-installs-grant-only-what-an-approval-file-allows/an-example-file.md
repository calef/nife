# An example approval file

An appendix of [milestone 911 (scripted installs grant only what the owner's approval file
allows)](../0911-scripted-installs-grant-only-what-an-approval-file-allows.md), moved out of the
block on 2026-10-12 (UTC) by maintainer/911-rollback-rulings, under §212 (a prose budget)'s cap.
The block's "The approval file" section says what a file must hold and the subset test it passes;
this shows one, in TOML, with provisional keys.

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

Under `lab-runner`, a monit update whose manifest adds a zone is refused, for example
`refused: monit 5.36 asks for zone example.net, which lab-runner does not allow`. Neither package
may be removed by a script holding this approval.
