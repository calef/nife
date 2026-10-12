# Seeded from relibc (MIT, vendor/relibc/LICENSE), musl's code (README.md here), at 893a3b9133ac, 2026-10-12 (UTC), for milestone 836; nife owns it from here, and its edits say `nife:` where they are (vendor/README.md).
# nife: relibc sets `.attribute arch, "rv64gc"` here (its rust issue #80608); nife's target is
# rv64imac, and with the float registers gone below nothing here needs more.
.global __setjmp
.global _setjmp
.global setjmp
.type __setjmp, %function
.type _setjmp,  %function
.type setjmp,   %function
__setjmp:
_setjmp:
setjmp:
	sd s0,    0(a0)
	sd s1,    8(a0)
	sd s2,    16(a0)
	sd s3,    24(a0)
	sd s4,    32(a0)
	sd s5,    40(a0)
	sd s6,    48(a0)
	sd s7,    56(a0)
	sd s8,    64(a0)
	sd s9,    72(a0)
	sd s10,   80(a0)
	sd s11,   88(a0)
	sd sp,    96(a0)
	sd ra,    104(a0)

	# nife: fs0 to fs11 are not saved. A nife process is soft-float (rv64imac, lp64), so the
	# registers do not exist and nothing keeps a value in them across a call.

	li a0, 0
	ret
