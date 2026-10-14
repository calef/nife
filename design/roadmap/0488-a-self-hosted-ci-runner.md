---
status: REFUSED
raised: 2026-09-20
refused_by: 81, 119, 448
---
# 488. A self-hosted CI runner

Refused by milestone 119 (design/roadmap/0119-merge-throughput.md) and milestone 81
(design/roadmap/0081-hvf-leg.md), each of which records it in its Follow-on. Milestone 448
(design/roadmap/0448-a-refusal-gets-a-number.md) gives a refusal that names work a number, a status
and a condition that would change it.

## The refusal, in its own words

From '119. The merge queue is the bottleneck, and the long pole is one prover', under `##
Follow-on`:

> A self-hosted runner on `cordoba`, whose 23 GB would fit `VERIFY_JOBS=4` where the hosted 16 GB
> fits two. Sharding needed no new infrastructure and no new failure mode, and owning a runner for
> a public repository that accepts outside pull requests is a security posture rather than a
> configuration.
>
> -- design/roadmap/0119-merge-throughput.md

From '81. An HVF leg: the test suite on the physical core', under `## Follow-on`:

> A self-hosted runner on the dev machine, which is the only way to put HVF in hosted CI. GitHub's
> macOS arm64 runners are themselves virtual machines with no nested virtualization. It couples CI
> to a laptop that sleeps. The loud skip was taken instead so a transcript can never be misread as
> having had silicon coverage.
>
> -- design/roadmap/0081-hvf-leg.md

## Why it is here rather than only there

Two blocks refused the same machine for different jobs. One wanted cordoba's 23 GB so that
`VERIFY_JOBS=4` would fit where the hosted 16 GB fits two, and took sharding instead, which needed
no new infrastructure and no new failure mode. The other wanted the dev machine, because a
self-hosted runner is the only way to put HVF in hosted CI at all: GitHub's macOS arm64 runners are
themselves virtual machines with no nested virtualization. It took a loud skip instead, so a
transcript can never be misread as having had silicon coverage.

## Revisit

- **Condition.** The security posture changing, or a job that only local silicon can run becoming
  load-bearing. The first refusal is explicit that owning a runner for a public repository accepting
  outside pull requests is a posture rather than a configuration, and the second is explicit that the
  alternative it took, a loud skip, is a report rather than coverage.

## Index row

One piece of infrastructure, refused twice for different jobs, with the security argument on one
side and a laptop that sleeps on the other. Gathering them makes the standing position legible: this
project does not own a runner, and it has said why twice.
