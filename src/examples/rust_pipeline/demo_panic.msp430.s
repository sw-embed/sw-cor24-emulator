	.file	"demo_panic.6df6f61352875665-cgu.0"
	.section	.text._RNvCsgMG9zBUy57e_7___rustc17rust_begin_unwind,"ax",@progbits
	.hidden	_RNvCsgMG9zBUy57e_7___rustc17rust_begin_unwind
	.globl	_RNvCsgMG9zBUy57e_7___rustc17rust_begin_unwind
	.p2align	1
	.type	_RNvCsgMG9zBUy57e_7___rustc17rust_begin_unwind,@function
_RNvCsgMG9zBUy57e_7___rustc17rust_begin_unwind:
	call	#emit_panic
.Lfunc_end0:
	.size	_RNvCsgMG9zBUy57e_7___rustc17rust_begin_unwind, .Lfunc_end0-_RNvCsgMG9zBUy57e_7___rustc17rust_begin_unwind

	.section	.text.demo_panic,"ax",@progbits
	.globl	demo_panic
	.p2align	1
	.type	demo_panic,@function
demo_panic:
	mov	#-256, r12
	mov	#1, r13
	call	#mmio_write
	call	#emit_panic
.Lfunc_end1:
	.size	demo_panic, .Lfunc_end1-demo_panic

	.section	.text.emit_panic,"ax",@progbits
	.globl	emit_panic
	.p2align	1
	.type	emit_panic,@function
emit_panic:
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
.LBB2_1:
	jmp	.LBB2_1
.Lfunc_end2:
	.size	emit_panic, .Lfunc_end2-emit_panic

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
	call	#demo_panic
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
