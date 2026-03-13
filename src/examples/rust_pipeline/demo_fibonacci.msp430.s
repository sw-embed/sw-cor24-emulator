	.file	"demo_fibonacci.bee7b57dc10298c8-cgu.0"
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

	.section	.text.demo_fibonacci,"ax",@progbits
	.globl	demo_fibonacci
	.p2align	1
	.type	demo_fibonacci,@function
demo_fibonacci:
	mov	#10, r12
	call	#fibonacci
	mov	r12, r13
	mov	#-256, r12
	call	#mmio_write
.LBB1_1:
	jmp	.LBB1_1
.Lfunc_end1:
	.size	demo_fibonacci, .Lfunc_end1-demo_fibonacci

	.section	.text.fibonacci,"ax",@progbits
	.globl	fibonacci
	.p2align	1
	.type	fibonacci,@function
fibonacci:
	push	r9
	push	r10
	mov	r12, r9
	mov	#1, r10
	cmp	#2, r9
	jlo	.LBB2_4
	clr	r10
.LBB2_2:
	mov	r9, r12
	add	#-1, r12
	call	#fibonacci
	add	r12, r10
	add	#-2, r9
	cmp	#2, r9
	jhs	.LBB2_2
	inc	r10
.LBB2_4:
	mov	r10, r12
	pop	r10
	pop	r9
	ret
.Lfunc_end2:
	.size	fibonacci, .Lfunc_end2-fibonacci

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
	call	#demo_fibonacci
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
