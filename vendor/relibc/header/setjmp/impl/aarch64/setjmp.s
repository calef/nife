// Seeded from relibc (MIT, vendor/relibc/LICENSE), musl's code (README.md here), at 893a3b9133ac, 2026-10-12 (UTC), for milestone 836; nife owns it from here, and its edits say `nife:` where they are (vendor/README.md).
.global __setjmp
.global _setjmp
.global setjmp
.type __setjmp,@function
.type _setjmp,@function
.type setjmp,@function
__setjmp:
_setjmp:
setjmp:
	// IHI0055B_aapcs64.pdf 5.1.1, 5.1.2 callee saved registers
	stp x19, x20, [x0,#0]
	stp x21, x22, [x0,#16]
	stp x23, x24, [x0,#32]
	stp x25, x26, [x0,#48]
	stp x27, x28, [x0,#64]
	stp x29, x30, [x0,#80]
	mov x2, sp
	str x2, [x0,#104]
	// nife: d8 to d15 are not saved. A nife process is soft-float (`-mgeneral-regs-only`, and
	// the Rust targets' `softfloat` ABI), so nothing keeps a value in them across a call.
	mov x0, #0
	ret
