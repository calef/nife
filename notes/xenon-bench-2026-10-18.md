# xenon bench card: the I219, 801's fetch, ioping on both systems, then the soak

*Name: provisional, minted 2026-10-18 (UTC) by lane `lane/xenon-bench-2026-10-18`, after
`notes/xenon-bench-2026-10.md`; naming is an architect's.*

One evening at xenon, not yet scheduled; the file takes the evening's date when it is. Five steps,
in this order, because each needs the one before:

1. The I219 boot: milestone 494 (a driver for the network card a PC actually has)'s bench boot,
   which must end `verdict LEASED-AND-MEASURED`. About 20 minutes.
2. On patagonia: flip `PCH_PROVEN_ON_SILICON` in a commit citing that log, and build the
   installer from it. About 5 minutes.
3. Milestone 801 (packages over the internet)'s exit criterion: install, boot from the disk, then
   `jig update` and `jig install ioping` from `basalt.nifeos.org`. About 15 minutes. Waits on
   #2370 (below).
4. Risk 6, step 1 of its path back to GREEN: ioping on nife (the package step 3 installed), then on
   Linux, three boots of five passes each. About an hour.
5. Risk 5: the first of the three boots §259 (a multicore soak counts toward risk 5 at ten million
   crossings over three boots) needs on xenon, left running overnight.

Unit details (serial device names, addresses, serial numbers) are kept off-tree by the maintainers.
Where this card and a procedure page disagree, the machine decides and the page gets corrected.

## Status

Not ready to run (see "What stands between this card and the evening"). Every image below was built from main at
`805620590` and booted under OVMF.

| image | build | under OVMF | SHA-256 |
|---|---|---|---|
| I219 boot, house-LAN peer | `cargo xtask network-bench --stage-only --peer <patagonia's LAN address>:9494` | `release build`, `82574L ... link up`, `preflight dmar scope : PASS`, `dhcp : leased 10.0.2.15`; gate `verdict LEASED-AND-MEASURED` | `36a35b926d78c25fd8e376c597b8ff75b7b788b82aaa40b941e52d72372431a3` |
| I219 boot, Internet Sharing peer | `... --peer 192.168.2.1:9494` | the same lines | `15b1d0ad206f4a0170ebfcf559d8876edc6ef003a1b2afdd31697381084c6062` |
| installer, basalt's sources, flag unflipped (rehearsal only) | `cargo xtask std-exerciser`, then `NIFE_INDEX_SOURCES=basalt cargo xtask install-boot` | `install-boot: PASS`, three boots; packs `repository basalt.nifeos.org 443 rolling` | `052b23ec58b256840ef1d67786c5170233fea288cb22dc147109c434f2c3016c` |
| soak, debug | `cargo xtask uefi-image --features soak_test` | 4 cores online, 24 user threads; at 56 s `crossings=6021`, `wakerate=341/s`, `refused=0 mismatch=0 stalled=0` | `d8f64631ba9447beef0d40c4f6564ffb8daab50549961a0520127ae3e47fe085` |

The I219 image's peer address is compiled in, so there is one image per network arrangement. The
installer that goes to xenon is built at the bench in step 2, so its hash is recorded then. Copies
are kept off-tree by the maintainers.

## What stands between this card and the evening

Two things, neither a bench step nor a ruling.

- Step 3 needs #2370 (the name resolver gets a budget it can lend from) on main. Rehearsed under
  QEMU on its branch at `3ed3256c9`, from an installed disk, the whole of step 3 passes:
  `index updated: 6 packages for x86_64, from basalt.nifeos.org over TLS`, then `fetched
  ioping-1.3-x86_64 from release-assets.githubusercontent.com via github.com over TLS` and
  `fetched and installed; generation 1 is live`. On main at `805620590` the same `jig update`
  fails with `NetworkUnreachable`. The images in the table are rebuilt once #2370 merges.
- Step 4's nife row does not yet measure the device. In the same rehearsal, `ioping -c 20 -i 0 -s
  4k ioping/1.3/ioping` runs but prints `non-cached I/O not supported by this platform`. It sees
  the file as `0 B`, so every request reads offset 0, and steady reads take 41 to 42 us. That is a
  cached read of one 4 KiB block, not the NVMe, so it cannot explain or correct the 2.16x.
  Milestone 834 (ioping on nife and Linux) already says what follows: "If the file path goes
  through a cache, the run uses the block window instead, and the row says so", and Linux makes
  the same choice. So the nife row waits on engineering, not a ruling: why nife reports the file
  as 0 B, why ioping cannot do non-cached I/O there, and a direct-I/O target in the block window.
  The step 4 commands below change when that lands.

## What calef needs at the bench

- **A monitor and a USB keyboard on xenon.** It halts at POST without a keyboard.
- The serial chain: the C4PDJ module's COM1, the null-modem barrel and the FTDI adapter into
  patagonia. Every step logs over it. The installer's `INSTALL` is typed there, since nife has no
  USB keyboard driver.
- Ethernet on xenon with a path to the internet. Its room has no port, so either a long cable to the
  house router or a cable to patagonia with macOS Internet Sharing on (patagonia is then
  `192.168.2.1`). Pick before building step 1's image. Internet Sharing meets milestone 494's
  criterion 2 only in spirit; the Results row says which.
- Two FAT32 sticks: `NIFE` for the nife images and the ioping RPM, and the Fedora 44 live stick.
- Someone at patagonia for step 2, which is a commit and a build.

## Before the evening, on patagonia

In a checkout of the commit under test (a fresh worktree builds nothing it was not asked to):

```sh
pgrep -l qemu                                    # nothing of yours should be running
git log -1 --format=%h                           # goes in every Results row
cargo xtask std-exerciser                        # builds jig; without it the image has no jig
P=$(ipconfig getifaddr en0)                      # or P=192.168.2.1 with Internet Sharing
cargo xtask network-bench --stage-only --peer "$P:9494"
shasum -a 256 target/esp-network-bench/EFI/BOOT/BOOTX64.EFI
curl -LO https://dl.fedoraproject.org/pub/fedora/linux/releases/44/Everything/x86_64/os/Packages/i/ioping-1.3-10.fc44.x86_64.rpm
```

`cargo xtask std-exerciser` matters: `jig` is packed only when it has been built, and an image
without it answers `jig update` with `could not spawn (the progenitor is out of memory)`.

## Step 1: the I219 boot

The procedure is `notes/e1000e.md`, "calef's bench step on xenon", with its table of every ending.
In short: start the peer in a terminal on patagonia,

```sh
while true; do head -c 104857600 /dev/zero | nc -l 9494; done
```

then capture serial in a second terminal, copy the image and boot xenon from it:

```sh
script/board-console --board xenon --port /dev/cu.usbserial-<id> --for 10m --until none \
    --log target/xenon-i219-$(date -u +%Y%m%dT%H%M%SZ).log
cp target/esp-network-bench/EFI/BOOT/BOOTX64.EFI /Volumes/NIFE/EFI/BOOT/BOOTX64.EFI && diskutil eject /Volumes/NIFE
```

Success is `network-bench: nic : 8086:15e3 I219-LM5 (SPT), link up`, a `dhcp : leased` line,
a `transfer` line and `network-bench: verdict LEASED-AND-MEASURED`. Anything else ends the evening's
network half: route it by that table, and go to step 4's Linux half and step 5.

## Step 2: flip the flag, build the installer

Only after step 1 ended `LEASED-AND-MEASURED`. On a branch from the commit under test:

```sh
git switch -c lane/i219-proven-on-xenon
sed -i '' 's/^pub const PCH_PROVEN_ON_SILICON: bool = false;/pub const PCH_PROVEN_ON_SILICON: bool = true;/' kernel/src/user/e1000e_service.rs
git commit -am "e1000e: the I219 is proven on xenon (bench log <file>, verdict LEASED-AND-MEASURED)"
NIFE_INDEX_SOURCES=basalt cargo xtask install-boot          # must end install-boot: PASS
shasum -a 256 target/esp-install/EFI/BOOT/BOOTX64.EFI
cp target/esp-install/EFI/BOOT/BOOTX64.EFI /Volumes/NIFE/EFI/BOOT/BOOTX64.EFI && diskutil eject /Volumes/NIFE
```

The commit message names the step 1 log, and the log is committed with it under
`bench/xenon-<date>/`, as the flag's own comment requires. QEMU's NIC is an 82574L, not a PCH part,
so `install-boot` passing says nothing about the I219; step 3 is the I219's test at the prompt.

## Step 3: milestone 801, install, boot from the disk, fetch

This destroys what is on the NVMe, which is the 2026-10-04 install. The steps are the October
card's step 2 (`notes/xenon-bench-2026-10.md`), with the serial capture in `screen`, because
`INSTALL` has to be typed:

```sh
cd <checkout>/target && screen -L /dev/cu.usbserial-<id> 115200     # ctrl-a k ends; log is screenlog.0
```

1. Boot the stick. Check that the offer's `TARGET` names the disk by size, then type `INSTALL` in
   `screen` within 30 seconds. Wait for `install : DONE`.
2. Remove the stick, power off, power on with the Ethernet cable in. Note whether the firmware found
   the disk itself or needed F12.
3. At `$`, type:

   ```text
   jig update
   jig install ioping
   cd packages
   ioping -v
   ```

| line | means |
|---|---|
| `index updated: N packages for x86_64, from basalt.nifeos.org over TLS` | the index came by name over pinned TLS |
| `fetched ioping-1.3-x86_64 from ... over TLS`, then `fetched and installed; generation N is live` | milestone 801's exit criterion is met |
| `ioping 1.3` | the installed program runs |
| `index unreachable at basalt.nifeos.org: NetworkUnreachable (<step>)` | the resolver was refused a network step, and the parenthesis names which |
| `index unreachable at basalt.nifeos.org: <other>` | the name resolved and the connection or TLS failed; the text says which |
| `passed over https://...: <why>` | the Release location was refused; `jig` falls back to Pages' `targets/` |
| `no source sent the package; nothing was installed` or `refused: the index does not vouch for the bytes` | a fetch or digest failure; stop |

## Step 4: ioping on nife, then on Linux

calef ruled on 2026-10-18 (UTC) that ioping is the benchmark that explains or corrects risk 6's 2.16x
read: unmodified, on nife and on Linux on xenon, with real logs and repeated passes.

### nife, three boots

ioping holds only its own installed file, read-only, so that file is the target. In `screen`, after
each boot from the disk:

```text
cd packages
wc ioping/1.3/ioping
ioping -c 1000 -i 0 -s 4k ioping/1.3/ioping
```

Type the `ioping` line five times. The `wc` line's third number is the file's size in bytes, `F`
(208440 in the rehearsal), which the Linux run uses. Power off fully between boots. ioping's first
line, `non-cached I/O not supported by this platform`, is expected and goes in the log.

### Linux, three boots

Fedora stick in, F12, "Try Fedora", a terminal. Nothing here writes to the NVMe: ioping refuses to
write a device without `-WWW`, and no line passes `-W`.

```sh
sudo dnf install -y ioping nvme-cli || sudo rpm -i /run/media/liveuser/NIFE/ioping-1.3-10.fc44.x86_64.rpm
sudo stty -F /dev/ttyS0 115200 cs8 -cstopb -parenb -crtscts onlcr
```

With `screen` capturing on patagonia, one command per boot, `B` the boot and `F` from nife's `wc`:

```sh
B=1 F=<bytes>; {
  date -u +%FT%TZ; uname -r; ioping -v
  cat /sys/block/nvme0n1/queue/scheduler /sys/module/nvme/parameters/poll_queues
  sudo nvme smart-log /dev/nvme0n1 | grep -i temp
  for p in 1 2 3 4 5; do echo "=== boot $B pass $p as-nife"; sudo ioping -c 1000 -i 0 -s 4k -S $F -o 1m /dev/nvme0n1; done
  for p in 1 2 3 4 5; do echo "=== boot $B pass $p direct"; sudo ioping -D -c 1000 -i 0 -s 4k -S $F -o 1m /dev/nvme0n1; done
  for p in 1 2 3 4 5; do echo "=== boot $B pass $p risk6-window"; sudo ioping -c 1000 -i 0 -s 4k -S 64m -o 1m /dev/nvme0n1; done
} 2>&1 | sudo tee /dev/ttyS0
```

`-c 1000 -i 0 -s 4k` is the same on both systems: 1000 requests of 4 KiB, no pause, one at a time,
each line a latency. The `as-nife` set reads a working set of nife's size. Without `-D`, ioping
drops the page cache before each read, the mode nife can match. The `direct` set adds `O_DIRECT` as
Linux's reference. The `risk6-window` set reads the 64 MiB at 1 MiB that risk 6's figure and the
Oct 4 `fio` run used.

## Step 5: the soak, boot 1 of three

radon crossed at 44 to 150 a second, so ten million on xenon is something like 20 to 60 hours over
three boots. A plain boot soaks until the watcher stops; it is not the `reboot_soak_test` build. The
procedure is `notes/soak.md`, "On radon at a bench". Quit `screen`, check `lsof /dev/cu.usbserial-*`,
then:

```sh
script/board-console --board xenon --port /dev/cu.usbserial-<id> --for 12h --until none \
    --log target/xenon-soak-$(date -u +%Y%m%dT%H%M%SZ).log
```

Boot the soak image. Before walking away, the first beat must show `on 4 online core(s)`,
`wakerate` near 400/s, and `crossings` rising between beats; if not, stop.

## Recording

Under `bench/xenon-<date>/`, each log whole, stripped with `LC_ALL=C tr -cd '\11\12\15\40-\176'`
and nothing else (`notes/bench-runbook.md`'s BUGS):

- `i219.log`, with `notes/e1000e.md`'s Results row; it lands with step 2's commit.
- `install-and-fetch.log` and `ioping-nife.log` from `screenlog.0`, and the line for milestone 801's
  block.
- `ioping-linux.log`, and risk 6's comparison, every pass, through whoever holds `design/`.
- `soak-boot1.log`, with its last beat in `notes/soak.md`'s table and milestone 225 (run the soak on
  radon, argon and xenon)'s block.

## BUGS

- The nife and Linux ioping rows are not apples-to-apples. nife reads a file through RedoxFS and the
  file server, and Linux reads the raw device. What direct I/O means on nife is milestone 834
  (ioping on nife and Linux)'s open question.
- Step 2 builds the evening's installer from a branch that has not merged, because the flag may
  flip only after silicon proves it. The branch's pull request follows the evening.
- The soak image is a debug build, as radon's were.
- Every xenon-side line above is a prediction. xenon has booted four cores (2026-09-17) and has
  never run the I219 path, `jig` or the soak.
