# Seeded from relibc (MIT, vendor/relibc/LICENSE), musl's code (README.md here), at 893a3b9133ac, 2026-10-12 (UTC), for milestone 836; nife owns it from here, and its edits say `nife:` where they are (vendor/README.md).
# nife: relibc sets `.attribute arch, "rv64gc"` here (its rust issue #80608); nife's target is
# rv64imac, and with the float registers gone below nothing here needs more.
.global __longjmp
.global _longjmp
.global longjmp
.type __longjmp, %function
.type _longjmp,  %function
.type longjmp,   %function
__longjmp:
_longjmp:
longjmp:
	ld s0,    0(a0)
	ld s1,    8(a0)
	ld s2,    16(a0)
	ld s3,    24(a0)
	ld s4,    32(a0)
	ld s5,    40(a0)
	ld s6,    48(a0)
	ld s7,    56(a0)
	ld s8,    64(a0)
	ld s9,    72(a0)
	ld s10,   80(a0)
	ld s11,   88(a0)
	ld sp,    96(a0)
	ld ra,    104(a0)

	# nife: fs0 to fs11 are not saved. A nife process is soft-float (rv64imac, lp64), so the
	# registers do not exist and nothing keeps a value in them across a call.

	seqz a0, a1
	add a0, a0, a1
	ret
