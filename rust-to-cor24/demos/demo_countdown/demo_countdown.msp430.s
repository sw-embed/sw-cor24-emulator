	.file	"demo_countdown.b078f427bbae8929-cgu.0"
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

	.section	.text.delay,"ax",@progbits
	.globl	delay
	.p2align	1
	.type	delay,@function
delay:
	sub	#2, r1
	tst	r12
	jeq	.LBB1_3
	add	#-1, r12
.LBB1_2:
	mov	r12, 0(r1)
	add	#-1, r12
	cmp	#-1, r12
	jne	.LBB1_2
.LBB1_3:
	add	#2, r1
	ret
.Lfunc_end1:
	.size	delay, .Lfunc_end1-delay

	.section	.text.demo_countdown,"ax",@progbits
	.globl	demo_countdown
	.p2align	1
	.type	demo_countdown,@function
demo_countdown:
	push	r10
	mov	#10, r10
.LBB2_1:
	mov	#-256, r12
	mov	r10, r13
	call	#mmio_write
	mov	#1000, r12
	call	#delay
	add	#-1, r10
	tst	r10
	jne	.LBB2_1
	mov	#-256, r12
	clr	r13
	call	#mmio_write
.LBB2_3:
	jmp	.LBB2_3
.Lfunc_end2:
	.size	demo_countdown, .Lfunc_end2-demo_countdown

	.section	.text.mmio_write,"ax",@progbits
	.globl	mmio_write
	.p2align	1
	.type	mmio_write,@function
mmio_write:
	mov.b	r13, 0(r12)
	ret
.Lfunc_end3:
	.size	mmio_write, .Lfunc_end3-mmio_write

	.section	.text.start,"ax",@progbits
	.globl	start
	.p2align	1
	.type	start,@function
start:
	call	#demo_countdown
.Lfunc_end4:
	.size	start, .Lfunc_end4-start

	.section	.text.uart_putc,"ax",@progbits
	.globl	uart_putc
	.p2align	1
	.type	uart_putc,@function
uart_putc:
	mov	r12, r13
	mov	#-255, r12
	call	#mmio_write
	ret
.Lfunc_end5:
	.size	uart_putc, .Lfunc_end5-uart_putc

	.ident	"rustc version 1.93.0-nightly (c871d09d1 2025-11-24)"
	.section	".note.GNU-stack","",@progbits
