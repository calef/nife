# Turning Secure Boot off, before booting the nife stick on a PC

This is the first instruction of the trivial-install page (milestone 802 (the trivial install that
makes a second customer possible), rung 4), written so the page can carry it as it stands. Milestone
500 (a stick that boots with Secure Boot on, or a page that says how to turn it off) holds the
ruling behind it: nife does not sign its boot file, so a stranger turns Secure Boot off. Everything
a vendor's menu is said to contain below cites the vendor page it was read from, and the
[sources](#sources) list gives the date each was read. What was not confirmed is marked where a
reader meets it.

**Why this step exists.** Secure Boot is a firmware setting that lets a PC start only boot files
signed by a key the firmware trusts. Most PCs sold with Windows ship with it on (HP states this for
every HP computer made with Windows 11 [hp]). nife's boot file, `\EFI\BOOT\BOOTX64.EFI` on the
stick, is not signed, so with Secure Boot on the PC refuses it, often with a message about a
security violation (`notes/x86-uefi-boot.md`, as seen on a Dell).

## 1. Before you change anything, get your BitLocker recovery key

**If Windows on this PC uses BitLocker or Device Encryption, changing a firmware setting can make
Windows ask for a 48-digit recovery key the next time it starts. Without that key, Windows and
every file on its disk stay locked. Microsoft cannot give it to you.** In Microsoft's own words, "Microsoft Support doesn't have the ability to retrieve, provide, or recreate a lost
BitLocker recovery key" [ms-key].

What Microsoft documents, and why it applies here:

- Device Encryption is turned on when you first sign in or set up a PC with a Microsoft account, or
  a work or school account. It is not turned on automatically for a local account [ms-de]. So a
  PC can be encrypted without its owner having chosen it.
- BitLocker's default unlock is "TPM-only". In that mode, "if BitLocker detects changes to the BIOS
  or UEFI configuration ... BitLocker enters recovery mode", and BitLocker protects Secure Boot's
  integrity through the TPM's `PCR[7]` measurement [ms-cm]. Turning Secure Boot off is such a change.
  ASUS states the consequence plainly: "modifying BIOS settings may require you to enter the
  BitLocker key to unlock the drive" [asus-nb].

What to do, in Windows, before restarting:

1. Open **Settings → Privacy & security → Device encryption** [ms-de]. If the switch is on, or
   your PC shows BitLocker as on, continue. If neither page exists, the disk is likely not
   encrypted, and you can go to step 2.
2. Find the recovery key and write it down somewhere other than this PC. For a personal PC it is
   usually in your Microsoft account at `aka.ms/myrecoverykey`; on a work or school PC it is held
   by that organization, at `aka.ms/aadrecoverykey` [ms-key]. It may also be on a printout or a
   USB stick from when encryption was set up [ms-key]. On a work or school PC, ask the people who
   run it before changing its firmware at all.
3. Optionally, suspend BitLocker protection first. Microsoft documents suspending for planned
   firmware changes: the drive stays encrypted, and protection resumes on its own at the next
   restart [ms-ro]. *Not confirmed*: whether Windows Home shows a suspend control for Device
   Encryption was not read. Having the key written down is the step that matters.

If Windows does ask for the key after you return to it, type it in. Windows then starts normally.

## 2. Open the firmware settings

Either way works:

- **From Windows 11**: **Settings → System → Recovery → Advanced startup → Restart now**, then
  **Troubleshoot → Advanced options → UEFI Firmware Settings → Restart** [hp]. Microsoft's older
  instructions reach the same menu by holding Shift while choosing **Restart** [ms-sb].
- **With a key at power-on**, from the table in section 3. Press it repeatedly from the moment you
  press the power button until the setup screen appears.

## 3. Turn Secure Boot off

Find your PC's maker below. "Read" means the vendor's page was read in full; "excerpt" means only
a search engine's excerpt of the vendor's page was read, because the page itself could not be
fetched.

| Maker | Setup key | Where the setting is, and what to choose | Boot menu key | Source |
|---|---|---|---|---|
| Dell | F2 at the Dell logo | **Secure Boot → Secure Boot Enable**: clear it, or set Secure Boot to **Disabled** | F12 | Read [dell-sb], [dell-f12] |
| HP consumer laptop | Esc, then F10 | **System Configuration → Boot Options → Secure Boot → Disabled**, then F10 to save | Esc, then F9 | Read [hp] |
| HP business laptop or workstation | Esc, then F10 | **Security → Secure Boot Configuration**: clear **Secure Boot**, then **Main → Save Changes and Exit** | Esc | Read [hp] |
| HP desktop | F10 | **Security → Secure Boot Configuration**, press F10 at the message, set Secure Boot to **Disable** | Esc | Read [hp] |
| Lenovo ThinkPad, ThinkCentre, ThinkStation | F1 at the logo | **Security → Secure Boot → Secure Boot: Disabled**, then F10 | not read | Read [lenovo-think] |
| Lenovo IdeaPad and other consumer models | F2 or Fn+F2, or the Novo button and **BIOS Setup** | not confirmed | F12 | Excerpt [lenovo-idea] |
| ASUS laptop | hold F2, then press power | F7 for **Advanced Mode**, then **Security → Secure Boot → Secure Boot Control: Disabled**, then F10 | not read | Read [asus-nb] |
| Acer laptop | F2 at the logo | **Security**: set a supervisor password; then **Boot → Secure Boot: Disabled**, then F10. Acer advises removing the supervisor password afterward | not read | Excerpt [acer] |
| Anything else | often F1, F2, F12 or Esc | "usually in either the **Security** tab, the **Boot** tab, or the **Authentication** tab" | varies | Read [ms-sb] |

Two warnings that hold on every PC:

- **Leave the boot mode on UEFI.** Do not turn on *Legacy Support*, *Legacy Boot* or *CSM*. HP's
  instructions go on to enable Legacy Support after Secure Boot is off [hp], and Microsoft's
  mention a Compatibility Support Module [ms-sb]; both are for older systems. nife's stick boots
  only through UEFI, so enabling either can stop the stick from being offered at all.
- **HP asks for a code.** After saving, an HP PC shows a code, on desktops in an "Operating
  System Boot Mode Change" message. Type it and press Enter. On desktops nothing appears as you
  type, which HP says is expected [hp].

## 4. Check, then boot the stick

To confirm from Windows, run `msinfo32`; **Secure Boot State** reads **Off** [dell-sb], [asus-nb].
Then put the stick in, restart, press the boot menu key from the table, and choose the USB stick.

## What turning it off costs

- With Secure Boot off, software that installs itself before Windows starts is no longer blocked.
  HP also says running Windows 11 without Secure Boot "might cause instability and prevent Windows
  from making updates" [hp].
- Microsoft warns that after installing other software, Secure Boot "may be difficult to
  re-activate ... without restoring your PC to the factory state" [ms-sb].
- You can turn it back on once you stop booting nife from the stick, by the same menu. A disk nife
  is installed on needs it off for as long as nife boots from it.

## BUGS

- **No stranger has followed this page, and only one PC has been checked against it**: xenon, a
  Dell OptiPlex 7050, whose menu reads *Secure Boot → Secure Boot Enable* with Disabled and Enabled
  (`notes/xenon-firmware.md`). Every other row is the vendor's own page, not a machine.
- **Dell's page describes turning Secure Boot on, not off**; the off steps are that page's steps
  reversed. Dell's page names two forms, a checkbox on OptiPlex, Precision, Wyse and XPS and an
  Enabled setting on Alienware, Inspiron and Vostro [dell-sb].
- **Four sources were excerpts, not read**: Acer's knowledge base sits behind a bot check, and
  Lenovo's IdeaPad page, ASUS's motherboard page and HP's Esc menu outside its Secure Boot page
  were not opened. Microsoft Surface, MSI and Gigabyte were not looked up at all.
- **That turning Secure Boot off triggers BitLocker recovery is an inference** from Microsoft's
  `PCR[7]` and "changes to the BIOS or UEFI configuration" sentences [ms-cm], and ASUS's general
  statement [asus-nb]. No Microsoft page read says it of Secure Boot in those words.

## Sources

Each was read on 2026-10-18 (UTC), in full unless the table above says excerpt.

- [ms-key] Microsoft Support, "Find your BitLocker recovery key",
  `support.microsoft.com/en-us/windows/find-your-bitlocker-recovery-key-6b71ad27-0b89-ea08-f143-056f5ab347d6`.
- [ms-de] Microsoft Support, "Device encryption in Windows",
  `support.microsoft.com/en-us/windows/device-encryption-in-windows-cf7e2b6f-3e70-4882-9532-18633605b7df`.
- [ms-cm] Microsoft Learn, "BitLocker countermeasures",
  `learn.microsoft.com/en-us/windows/security/operating-system-security/data-protection/bitlocker/countermeasures`.
- [ms-ro] Microsoft Learn, "BitLocker recovery overview",
  `learn.microsoft.com/en-us/windows/security/operating-system-security/data-protection/bitlocker/recovery-overview`.
- [ms-sb] Microsoft Learn (archived, Windows 8.1), "Disabling Secure Boot",
  `learn.microsoft.com/en-us/previous-versions/windows/it-pro/windows-8.1-and-8/dn481258(v=win.10)`.
- [dell-sb] Dell, "How to Enable Secure Boot on Your Dell Device", KB 000190116,
  `dell.com/support/kbdoc/en-us/000190116`.
- [dell-f12] Dell, "Flashing the BIOS from the F12 One-Time Boot Menu", KB 000128928 (excerpt;
  cited only for the F12 menu's name), `dell.com/support/kbdoc/en-us/000128928`.
- [hp] HP, "HP PCs - Secure Boot (Windows 11)", `support.hp.com/us-en/document/ish_6930187-6931079-16`.
- [lenovo-think] Lenovo, "How to enable Secure Boot on Think branded systems", HT509044,
  `support.lenovo.com/us/en/solutions/ht509044`.
- [lenovo-idea] Lenovo, "Recommended way to enter BIOS - ideapad", HT500216 (excerpt),
  `support.lenovo.com/us/en/solutions/ht500216`.
- [asus-nb] ASUS, "How to Enable/Disable Secure Boot", FAQ 1050047,
  `asus.com/us/support/FAQ/1050047/`.
- [acer] Acer Community knowledge base, "Enable or disable Secure Boot on an Acer notebook",
  article 88 (excerpt), `community.acer.com/en/kb/articles/88-enable-or-disable-secure-boot-on-an-acer-notebook`.
