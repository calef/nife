# xenon bench card, 2026-10-18: ioping on Linux, then the soak

*Name: provisional, minted 2026-10-18 (UTC) by lane `lane/xenon-bench-2026-10-18`, after
`notes/xenon-bench-2026-10.md`; naming is an architect's.*

One evening at xenon. The brief was three runs: risk 6's ioping comparison, milestone 801 (packages over the
internet)'s exit criterion, and the soak of §259 (a multicore soak counts toward risk 5 at ten
million crossings over three boots). **Two of the three cannot be done on nife tonight**, for the
reasons in "What is blocked, and why". What can be done, in this order:

1. Risk 6, the Linux half of step 1 of its path back to GREEN. Unmodified ioping 1.3 on a
   Fedora 44 live stick, read-only against the NVMe, five passes per boot, the log over serial to
   patagonia. About 15 minutes a boot.
2. Risk 5, the first of §259's three boots on xenon. The `soak_test` kernel, watched over serial
   by `script/board-console`, left running overnight.

Unit details (serial device names, addresses, serial numbers) are kept off-tree by the maintainers.
Where this card and a procedure page disagree, the machine decides and the page gets corrected.

Prepared 2026-10-18 (UTC) on `91be0eab3` (main at the claim).

## What is blocked, and why

- **Every release-profile x86_64 UEFI image on `91be0eab3` stops under OVMF right after the `pat`
  line**, before `cr4.smep`. The last line is
  `pat         : 0x0007040600070106 on core 0 (was ...); entry 1 is write-combining`, then nothing.
  Reproduced three ways: `cargo xtask install-boot` (twice), `cargo xtask network-bench`, and
  `cargo xtask uefi-image --release` booted by hand. The debug-profile `cargo xtask uefi-boot`
  passes, and the debug soak image boots at four cores. The installer must be release (the loader
  refuses to offer a debug image), and `network-bench` builds release, so neither can go to xenon.
  No CI job boots a release UEFI image, so main did not notice. A first-parent bisect from
  `a08efc8dc` names the merge of #1997 (`maintainer/static-mut-891`, 2026-10-12 UTC), which
  rewrote `kernel/src/arch/x86_64/segments.rs`. The fix is a kernel lane's, not a bench step.
- **The installed system has no network on xenon.** The kernel refuses the I219 at boot while
  `PCH_PROVEN_ON_SILICON` is `false` (`kernel/src/user/e1000e_service.rs`). The flag may be flipped
  only in a commit citing a xenon `network-bench` boot that ends `verdict LEASED-AND-MEASURED`
  (`notes/e1000e.md`, "calef's bench step on xenon"), and that image is release.
- So milestone 801's fetch cannot run tonight, and nife's half of the ioping comparison waits on it.
  Milestone 834 (ioping on nife and Linux) is done only when ioping is installed by `jig install`
  from basalt's index. On xenon the only way to the NVMe from a program is a file on the installed
  RedoxFS.

The order those take once release images boot again is in "When release images boot again" below.

## What calef needs at the bench

- **A monitor and a USB keyboard on xenon.** It halts at POST without a keyboard.
- The serial chain: the C4PDJ module's COM1, the null-modem barrel and the FTDI adapter into
  patagonia. Both runs log over it; nothing tonight is photographed.
- The Fedora 44 live stick used on 2026-10-04, and a FAT32 stick (`NIFE`) for the soak image and
  the ioping RPM.
- Network on xenon helps for Linux (`dnf install`), and is optional: the RPM can go on the stick.

## Before the evening, on patagonia

The soak image is built and boot-checked. Its SHA-256:

| image | built from | under OVMF | SHA-256 |
|---|---|---|---|
| `soak_test`, debug (`cargo xtask uefi-image --features soak_test`) | `91be0eab3` | 4 cores online; `soak-test: started 4 groups ... (24 user threads) on 4 online core(s)`; 61 s: `crossings=7620`, `wakerate=388/s`, `refused=0 mismatch=0 stalled=0` | `595e522e2323fc2009399cb25ef331fa20386a87a69323f3ec89e78a827c56d8` |

The copy is kept off-tree by the maintainers. To rebuild it instead, in a checkout at that commit:

```sh
cargo xtask uefi-image --features soak_test     # -> target/esp/EFI/BOOT/BOOTX64.EFI
shasum -a 256 target/esp/EFI/BOOT/BOOTX64.EFI   # compare with the table
```

Put the ioping RPM on the `NIFE` stick too, for a Linux boot with no network. It is Fedora's own
build of the same release, 1.3:

```sh
curl -LO https://dl.fedoraproject.org/pub/fedora/linux/releases/44/Everything/x86_64/os/Packages/i/ioping-1.3-10.fc44.x86_64.rpm
mkdir -p /Volumes/NIFE/EFI/BOOT
cp ioping-1.3-10.fc44.x86_64.rpm /Volumes/NIFE/
cp <soak image> /Volumes/NIFE/EFI/BOOT/BOOTX64.EFI
diskutil eject /Volumes/NIFE
```

The NIFE stick boots the soak, so it must not be in xenon during the Linux boots unless F12 picks
the Fedora stick.

## Run 1: ioping on Linux, three boots, five passes each

calef ruled on 2026-10-18 (UTC) that ioping is the benchmark that explains or corrects risk 6's
2.16x read: unmodified, on nife and on Linux on xenon, with real logs and repeated passes. Tonight
is its Linux row.

**Nothing here writes to the NVMe.** ioping reads; it refuses to write a device without `-WWW`, and
no command below passes `-W`. The Oct 4 install on the disk stays intact.

On patagonia, before powering xenon on:

```sh
ls /dev/cu.usbserial-*
cd <nife checkout>/target && screen -L /dev/cu.usbserial-<id> 115200   # logs to ./screenlog.0; ctrl-a k ends
```

On xenon: Fedora stick in, power on, F12, the stick's `UEFI:` entry, "Try Fedora". Open a terminal.

```sh
sudo dnf install -y ioping nvme-cli || sudo rpm -i /run/media/liveuser/NIFE/ioping-1.3-10.fc44.x86_64.rpm
sudo stty -F /dev/ttyS0 115200 cs8 -cstopb -parenb -crtscts onlcr
```

Then the whole run as one command, so every byte goes to the serial log. `B` is the boot number,
1 to 3:

```sh
B=1; {
  date -u +%FT%TZ; uname -r; ioping -v
  cat /sys/block/nvme0n1/queue/scheduler /sys/module/nvme/parameters/poll_queues
  cat /sys/block/nvme0n1/device/model; sudo nvme smart-log /dev/nvme0n1 | grep -i temp
  for p in 1 2 3 4 5; do
    echo "=== boot $B pass $p cached-io-invalidated $(date -u +%FT%TZ)"
    sudo ioping -c 1000 -i 0 -s 4k -S 64m -o 1m /dev/nvme0n1
  done
  for p in 1 2 3 4 5; do
    echo "=== boot $B pass $p direct $(date -u +%FT%TZ)"
    sudo ioping -D -c 1000 -i 0 -s 4k -S 64m -o 1m /dev/nvme0n1
  done
  sudo nvme smart-log /dev/nvme0n1 | grep -i temp
} 2>&1 | sudo tee /dev/ttyS0
```

What the arguments are: 1000 requests (`-c`), no pause between them (`-i 0`), 4 KiB each (`-s`).
They land at random 4 KiB-aligned offsets inside a 64 MiB window (`-S`) starting 1 MiB in (`-o`).
That is the window risk 6's nife figure and the Oct 4 `fio` run used. ioping prints one line per request with
its latency, then `min/avg/max/mdev`. The first set invalidates the page cache before each read,
which is ioping's default and the mode nife can repeat. The second set adds `-D` (`O_DIRECT`), the
Linux reference, since what direct I/O means on nife is milestone 834's open question.

Power off fully between boots, as risk 6's own runs did, since the Oct 4 Linux reads varied 74 to
188 MB/s with drive state. After boot 3, quit `screen` and keep `screenlog.0`.

Recording: commit `screenlog.0` whole as `bench/xenon-2026-10-18/ioping-linux.log`, stripped
with `LC_ALL=C tr -cd '\11\12\15\40-\176'` (`notes/bench-runbook.md`'s BUGS) and nothing else.

## Run 2: the soak, boot 1 of §259's three

§259 counts at least ten million crossings over at least three plain boots per architecture. radon
crossed at 44 to 150 a second, so xenon needs something like 20 to 60 hours over three boots;
tonight starts the first. A plain boot soaks until the watcher stops; it is not the
`reboot_soak_test` build. The procedure is `notes/soak.md`, "On radon at a bench", with xenon's
stick and `--board xenon`.

On patagonia, with nothing else holding the port (`lsof /dev/cu.usbserial-*`; quit `screen`
first):

```sh
script/board-console --board xenon --port /dev/cu.usbserial-<id> --for 12h --until none \
    --log target/xenon-soak-$(date -u +%Y%m%dT%H%M%SZ).log
```

Then: `NIFE` stick in xenon, power on, F12, the stick. Before walking away, read the first beat:

- `soak-test: started 4 groups of one responder, 3 callers, 1 grinder and 1 tick waiter (24 user
  threads) on 4 online core(s)`. Four cores is xenon's i5-7500T; fewer means an AP did not come up.
- `wakerate` near 400/s (100 per core), and `crossings` rising between beats.

If either is wrong, stop: hours of a soak that does not cross cores answer nothing. A
`soak-test: FAILED` line, or three missed beats, ends the watch on its own; the log is the artifact.

Recording: the log goes to `bench/xenon-2026-10-18/soak-boot1.log`, and its last beat's
`rounds`, `rate`, `wakes` and `crossings`, with the duration, go in `notes/soak.md`'s table and
milestone 225 (run the soak on radon, argon and xenon)'s block.

## When release images boot again

In order, because each needs the one before:

1. The I219 boot (`notes/e1000e.md`, "calef's bench step on xenon"): `cargo xtask network-bench
   --stage-only --peer "$P:9494"`, an Ethernet cable, a `nc` peer on patagonia. It must end
   `verdict LEASED-AND-MEASURED`. Then a lane flips `PCH_PROVEN_ON_SILICON`, citing the log.
2. Milestone 801's exit: `NIFE_INDEX_SOURCES=basalt cargo xtask install-boot` builds the
   installer with `package_index::BASALT_SOURCES` (`repository basalt.nifeos.org 443 rolling`, by
   name, pinned to ISRG Root X1) and proves it installs under OVMF. Install from the stick as the
   October card's step 2 does and boot from the disk with Ethernet in. At the prompt, over serial:

   ```text
   jig update
   jig install ioping
   ```

   Success reads `index updated: N packages for x86_64, from basalt.nifeos.org over TLS`, then
   `fetched ioping-1.3-x86_64 from ... over TLS` and `fetched and installed; generation N is live`.
   Failure names its cause: `index unreachable at basalt.nifeos.org: ...`, `passed over https://...`,
   `no source sent the package; nothing was installed`, or `refused: the index does not vouch for
   the bytes that arrived`.
3. nife's ioping row: in `packages`, `ioping -c 1000 -i 0 -s 4k <file>` on the installed
   system, five passes a boot over three boots. Unvouched, ioping holds only its own installed
   file, read-only (the BUGS of milestone 906 (ioping is packaged in basalt)), so the window
   cannot be 64 MiB at 1 MiB as on Linux.
   Matching the two is milestone 834's first question, not a bench step.

## BUGS

- Tonight's ioping run is half a comparison. The nife half waits on the release-image defect, the
  I219 flag and 801, in that order.
- The Linux run reads the raw device and nife's will read a file through RedoxFS and the file
  server. Even with identical arguments that is not apples-to-apples, and the comparison must say
  so.
- The soak image is a debug build, as `cargo xtask uefi-image` makes by default. radon's were too;
  a release soak would cross faster and is not what §259's radon count measured.
- Every xenon-side line above is a prediction. xenon has booted four cores (2026-09-17) and has
  never run the soak.
