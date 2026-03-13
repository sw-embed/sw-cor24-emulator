	.file	"demo_nested.f40f3a1c3ff23541-cgu.0"
	.section	.text._RNvCsgMG9zBUy57e_7___rustc17rust_begin_unwind,"ax",@progbits
	.hidden	_RNvCsgMG9zBUy57e_7___rustc17rust_begin_unwind
	.globl	_RNvCsgMG9zBUy57e_7___rustc17rust_begin_unwind
	.p2align	1
	.type	_RNvCsgMG9zBUy57e_7___rustc17rust_begin_unwind,@function
_RNvCsgMG9zBUy57e_7___rustc17rust_begin_unwind:
	mov	#80, r12
	call	#uart_putc
	mov	#65, r12
	call	#uart_putc
	mov	#78, r12
	call	#uart_putc
	mov	#73, r12
	call	#uart_putc
	mov	#67, r12
	call	#uart_putc
	mov	#10, r12
	call	#uart_putc
.LBB0_1:
	jmp	.LBB0_1
.Lfunc_end0:
	.size	_RNvCsgMG9zBUy57e_7___rustc17rust_begin_unwind, .Lfunc_end0-_RNvCsgMG9zBUy57e_7___rustc17rust_begin_unwind

	.section	.text.demo_nested,"ax",@progbits
	.globl	demo_nested
	.p2align	1
	.type	demo_nested,@function
demo_nested:
	mov	#-256, r12
	call	#mmio_read
	add	#5, r12
	call	#level_a
.Lfunc_end1:
	.size	demo_nested, .Lfunc_end1-demo_nested

	.section	.text.level_a,"ax",@progbits
	.globl	level_a
	.p2align	1
	.type	level_a,@function
level_a:
	add	#10, r12
	call	#level_b
.Lfunc_end2:
	.size	level_a, .Lfunc_end2-level_a

	.section	.text.level_b,"ax",@progbits
	.globl	level_b
	.p2align	1
	.type	level_b,@function
level_b:
	mov	r12, r13
	add	r12, r12
	add	#3, r12
	call	#level_c
.Lfunc_end3:
	.size	level_b, .Lfunc_end3-level_b

	.section	.text.level_c,"ax",@progbits
	.globl	level_c
	.p2align	1
	.type	level_c,@function
level_c:
	push	r10
	mov	r13, r10
	mov	r12, r13
	mov	#-256, r12
	call	#mmio_write
	mov	r10, r12
	call	#uart_putc
.LBB4_1:
	jmp	.LBB4_1
.Lfunc_end4:
	.size	level_c, .Lfunc_end4-level_c

	.section	.text.mmio_read,"ax",@progbits
	.globl	mmio_read
	.p2align	1
	.type	mmio_read,@function
mmio_read:
	mov.b	0(r12), r12
	ret
.Lfunc_end5:
	.size	mmio_read, .Lfunc_end5-mmio_read

	.section	.text.mmio_write,"ax",@progbits
	.globl	mmio_write
	.p2align	1
	.type	mmio_write,@function
mmio_write:
	mov.b	r13, 0(r12)
	ret
.Lfunc_end6:
	.size	mmio_write, .Lfunc_end6-mmio_write

	.section	.text.start,"ax",@progbits
	.globl	start
	.p2align	1
	.type	start,@function
start:
	call	#demo_nested
.Lfunc_end7:
	.size	start, .Lfunc_end7-start

	.section	.text.uart_putc,"ax",@progbits
	.globl	uart_putc
	.p2align	1
	.type	uart_putc,@function
uart_putc:
	mov	r12, r13
	mov	#-255, r12
	call	#mmio_write
	ret
.Lfunc_end8:
	.size	uart_putc, .Lfunc_end8-uart_putc

	.ident	"rustc version 1.93.0-nightly (c871d09d1 2025-11-24)"
	.section	".note.GNU-stack","",@progbits
