// Seeded from relibc (MIT, vendor/relibc/LICENSE), musl's code (README.md here), at 893a3b9133ac, 2026-10-12 (UTC), for milestone 836; nife owns it from here, and its edits say `nife:` where they are (vendor/README.md).
.global _longjmp
.global longjmp
.type _longjmp,%function
.type longjmp,%function
_longjmp:
longjmp:
	// IHI0055B_aapcs64.pdf 5.1.1, 5.1.2 callee saved registers
	ldp x19, x20, [x0,#0]
	ldp x21, x22, [x0,#16]
	ldp x23, x24, [x0,#32]
	ldp x25, x26, [x0,#48]
	ldp x27, x28, [x0,#64]
	ldp x29, x30, [x0,#80]
	ldr x2, [x0,#104]
	mov sp, x2
	// nife: d8 to d15 are not saved. A nife process is soft-float (`-mgeneral-regs-only`, and
	// the Rust targets' `softfloat` ABI), so nothing keeps a value in them across a call.

	mov x0, x1
	cbnz x1, 1f
	mov x0, #1
1:	br x30
