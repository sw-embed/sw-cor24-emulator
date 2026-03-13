	.file	"demo_uart_hello.3bf15f557303b3a-cgu.0"
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

	.section	.text.demo_uart_hello,"ax",@progbits
	.globl	demo_uart_hello
	.p2align	1
	.type	demo_uart_hello,@function
demo_uart_hello:
	mov	#72, r12
	call	#uart_putc
	mov	#101, r12
	call	#uart_putc
	mov	#108, r12
	call	#uart_putc
	mov	#108, r12
	call	#uart_putc
	mov	#111, r12
	call	#uart_putc
	mov	#10, r12
	call	#uart_putc
.LBB1_1:
	jmp	.LBB1_1
.Lfunc_end1:
	.size	demo_uart_hello, .Lfunc_end1-demo_uart_hello

	.section	.text.mmio_write,"ax",@progbits
	.globl	mmio_write
	.p2align	1
	.type	mmio_write,@function
mmio_write:
	mov.b	r13, 0(r12)
	ret
.Lfunc_end2:
	.size	mmio_write, .Lfunc_end2-mmio_write

	.section	.text.start,"ax",@progbits
	.globl	start
	.p2align	1
	.type	start,@function
start:
	call	#demo_uart_hello
.Lfunc_end3:
	.size	start, .Lfunc_end3-start

	.section	.text.uart_putc,"ax",@progbits
	.globl	uart_putc
	.p2align	1
	.type	uart_putc,@function
uart_putc:
	mov	r12, r13
	mov	#-255, r12
	call	#mmio_write
	ret
.Lfunc_end4:
	.size	uart_putc, .Lfunc_end4-uart_putc

	.ident	"rustc version 1.93.0-nightly (c871d09d1 2025-11-24)"
	.section	".note.GNU-stack","",@progbits
