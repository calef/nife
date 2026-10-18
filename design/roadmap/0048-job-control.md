---
status: PARTIAL
raised: 2026-07-31
milestone_dependencies: 47
decision_dependencies: 300
machine_requirements: none
specific_machine: none
needs_person: no
---
# 48. Job control: `jobs`, `wait`, `kill`, `fg`, `bg`, and a stopped state

**PARTIAL, 2026-10-18 (UTC).** Phase one is built for supervised jobs: a trailing `&`, `jobs`,
`wait`, `kill` and `fg`, proved at a real prompt by `script/swish-check` on all three
architectures. Phase two, `ThreadControlBlock::SUSPEND`/`RESUME` and `Ctrl-Z`, is not built.
[§300 (the paused state)](../decisions/0300-tcb-suspend-resume-the-paused-state.md) decided the
verbs on 2026-10-15: they ride the TCB capability, resume continues at the PC, and
[§26 (the fault endpoint)](../decisions/0026-fault-endpoint.md) carries no stopped event.

Why it matters. Unix job control is one of the most intricate things in a kernel: sessions, a
controlling terminal, process groups, `tcsetpgrp`, and `SIGTSTP`/`SIGCONT`/`SIGTTIN`/`SIGTTOU`. Most
of that machinery exists to answer one question (*who may read the keyboard*), and here that
question answers itself.

## A job is what the shell holds capabilities for

Structural rather than conventional. Unix's process group is a *number* with inherited, mutable
membership; "what I hold" cannot drift. What the shell holds depends on how the job was built:

- A supervised job (`interrupt_heeder` and its kind) is built by the progenitor out of a region
  the shell split off its own budget, and shares a job frame with the shell. The shell holds the
  region, so `DESTROY` tears the job down, and the frame, whose flags interrupt it and say it is
  done. That is everything job control needs short of pausing.
- Every other job is built in the progenitor's pool. The progenitor deletes its TCB at `START`
  ([§142 (what a spawner retains over a child after
  `START`)](../decisions/0142-spawn-retention.md)), it answers on the one result rendezvous every
  job shares, and `job_undertaker` holds its supervision endpoint. The shell holds nothing for it
  after the spawn, so it cannot wait on it, stop it, or keep its answer apart from the next
  command's.

So phase one covers supervised jobs, and `&` on any other program is refused with a sentence.

## Phase one: built, for supervised jobs

- `<command> &`: the line's last unquoted byte, on a line of one command
  (`swish::sequence::background`). The job starts and the prompt comes back as `[N] name running
  in the background`.
- `jobs`: each held job, running or done. A done job is reported once and its region reclaimed.
- `wait [%N]`: until that job, or every job, is done, reported as a foreground job's finish would
  be. `^C` stops the waiting and leaves the jobs running.
- `kill %N` sets the job frame's interrupt flag, [§24 (interrupting the
  foreground)](../decisions/0024-interrupting-the-foreground.md)'s cooperative tier; `kill -9 %N`
  is its forcible tier, `DESTROY` of the region.
- `fg [%N]`: the shell watches the job, the newest by default, under the same `^C` escalation a
  foreground start gets.

Foreground versus background is not a capability transfer, and the reason is in the tree: no
spawned program holds terminal input. The progenitor grants a child the terminal's sink, which
writes, and never the line discipline's `READLINE`. So `fg` has nothing to revoke. A foreground
job is one the shell is watching, with the terminal out of raw mode so its line discipline counts
`^C`; at the prompt a `^C` is a byte to the line editor and reaches no background job. A
background job that wanted to read the terminal would find no capability to read with.

The swish-check lines are in `xtask/src/swish_check/script/interrupts.rs`. The parsing is
host-tested in `grant_plan` (`job_operand`, `kill_operands`) and `swish::sequence`.

### BUGS

- `&` refuses every program but a supervised job, for the reason above.
- A finished background job is reported when `jobs`, `wait` or `fg` next looks, not before the
  prompt, because the shell is blocked reading the line.
- Background regions come off the shell's budget in start order, so finishing jobs out of order
  leaves a hole until the later ones go (`notes/object-revocation.md`).

## Phase two: the stopped state

Only `Ctrl-Z`, `bg` on a stopped job, and zsh's `suspend` need pausing a thread resumably. §24's
tiers are notify and kill, with pause deliberately absent. §300 adds `ThreadControlBlock::SUSPEND`
and `RESUME`, silent, designed for all three of §24's triggers (job control, a userspace pager,
and a debugger) rather than for job control alone.

`Ctrl-Z` then reads like `^C`: the line discipline counts it as it counts `^C`
(`OPERATION_INTRCOUNT`), the shell suspends the foreground job and returns to the prompt, and `bg`
resumes it without watching. What is open is how the shell comes to hold the TCB it suspends,
since the progenitor deletes it at `START`; that ruling is asked on the shell's phase-two pull
request.

## Follow-on

- **Outstanding.** Phase two's kernel half, `ThreadControlBlock::SUSPEND`/`RESUME` per §300 on
  all three architectures. Checked 2026-10-18: `kernel/src/sched` has no paused state.
- **Outstanding.** `Ctrl-Z` and `bg`, which wait on the kernel verbs and on how the shell comes to
  hold the TCB. Checked 2026-10-18: nothing counts `^Z` in `components/src/line_editor.rs`.
- **Recorded.** `&` on anything but a supervised job is refused, in this block's BUGS and in
  `components/src/swish/jobs.rs`.
- **Proposed.** Job control for every program:
  `design/roadmap/proposals/every-job-is-built-the-way-a-supervised-job-is.md`.

The kernel verbs are built, on all three architectures: `SUSPEND` stops a thread silently and
returns once it is off every core, and `RESUME` continues it at its saved PC
([notes/suspend-and-resume.md](../../notes/suspend-and-resume.md) has the cases). What is left is
the shell's half, `Ctrl-Z` and `bg`.

## The open question: `disown`

If the shell drops its capabilities to a job, nobody can reap it, and §26's dead-until-reaped means the
corpse persists. Unix reparents orphans to init and lets init reap them; here reparenting means
transferring the supervision endpoint, which is an explicit act rather than a rule nobody thinks
about. **Decided as DECISIONS §40**: a supervisor's death is its subtree's death, because a child's
resources come from its supervisor's region and §16's revocation reclaims the whole subtree in one act.
So `disown` means **transfer supervision upward**, not "abandon". §40 (a supervisor's death is its
subtree's death) records the hole that makes
the cascade close to the only coherent answer. §32 (a supervisor may collect a corpse) authorizes
reaping by matching the
child's recorded `fault_ep`, which nobody can satisfy once the supervisor's endpoint is gone.

Sequencing. Phase two is startable: §300 is decided. Effort: 1 lane estimated for it, noting
estimates for unbuilt work are guesses on a history-calibrated scale.

## Index row

most of it needs no new kernel surface, and the tty's most tangled feature turns out to be a
capability transfer
