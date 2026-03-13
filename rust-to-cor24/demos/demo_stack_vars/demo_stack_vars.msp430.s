	.file	"demo_stack_vars.de87e2681959ec26-cgu.0"
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

	.section	.text.accumulate,"ax",@progbits
	.globl	accumulate
	.p2align	1
	.type	accumulate,@function
accumulate:
	push	r6
	push	r7
	push	r8
	push	r9
	push	r10
	mov	r12, r10
	mov	r10, r6
	inc	r6
	add	r6, r10
	mov	r10, r9
	add	r6, r9
	mov	r10, r13
	xor	r6, r13
	xor	r9, r13
	mov	r9, r8
	add	r10, r8
	xor	r8, r13
	mov	r8, r7
	add	r9, r7
	xor	r7, r13
	mov	#-256, r12
	call	#mmio_write
	mov	r6, r12
	call	#uart_putc
	mov	r10, r12
	call	#uart_putc
	mov	r9, r12
	call	#uart_putc
	mov	r8, r12
	call	#uart_putc
	mov	r7, r12
	call	#uart_putc
.LBB1_1:
	jmp	.LBB1_1
.Lfunc_end1:
	.size	accumulate, .Lfunc_end1-accumulate

	.section	.text.demo_stack_vars,"ax",@progbits
	.globl	demo_stack_vars
	.p2align	1
	.type	demo_stack_vars,@function
demo_stack_vars:
	mov	#-256, r12
	call	#mmio_read
	inc	r12
	call	#accumulate
.Lfunc_end2:
	.size	demo_stack_vars, .Lfunc_end2-demo_stack_vars

	.section	.text.mmio_read,"ax",@progbits
	.globl	mmio_read
	.p2align	1
	.type	mmio_read,@function
mmio_read:
	mov.b	0(r12), r12
	ret
.Lfunc_end3:
	.size	mmio_read, .Lfunc_end3-mmio_read

	.section	.text.mmio_write,"ax",@progbits
	.globl	mmio_write
	.p2align	1
	.type	mmio_write,@function
mmio_write:
	mov.b	r13, 0(r12)
	ret
.Lfunc_end4:
	.size	mmio_write, .Lfunc_end4-mmio_write

	.section	.text.start,"ax",@progbits
	.globl	start
	.p2align	1
	.type	start,@function
start:
	call	#demo_stack_vars
.Lfunc_end5:
	.size	start, .Lfunc_end5-start

	.section	.text.uart_putc,"ax",@progbits
	.globl	uart_putc
	.p2align	1
	.type	uart_putc,@function
uart_putc:
	mov	r12, r13
	mov	#-255, r12
	call	#mmio_write
	ret
.Lfunc_end6:
	.size	uart_putc, .Lfunc_end6-uart_putc

	.ident	"rustc version 1.93.0-nightly (c871d09d1 2025-11-24)"
	.section	".note.GNU-stack","",@progbits
