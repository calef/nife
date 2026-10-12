# §123 appendix: option (d), a lazy trigger

One of the options [§123 (boot-time re-derivation)](../0123-boot-time-rederivation-privilege.md)
considered. Moved out of that page on 2026-10-12 (UTC) under §212 (a prose budget). No ruled word
changed.

**(d) Trigger re-derivation lazily, on the first scheduled job's fire time, rather than all-at-once at
boot.** Considered briefly and set aside as a scheduling-shape question rather than a privilege-shape
one; it does not change what grants the privilege or how it is scoped, only when the privileged
process runs. It is compatible with the recommendation below (the boot-only process could re-derive
everything at once, or could itself be structured to defer per-session work) and is not this
decision's fork. Noted so it is not silently assumed away.
