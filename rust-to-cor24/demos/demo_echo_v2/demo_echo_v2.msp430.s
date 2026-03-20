	.file	"demo_echo_v2.1a366a591b9a8cc1-cgu.0"
	.section	.text.handle_rx,"ax",@progbits
	.globl	handle_rx
	.p2align	1
	.type	handle_rx,@function
handle_rx:
	mov	#-255, r12
	call	#mmio_read
	cmp	#33, r12
	jne	.LBB0_2
	mov	#256, r12
	mov	#1, r13
	call	#mmio_write
	ret
.LBB0_2:
	call	#to_upper
	call	#uart_putc
	ret
.Lfunc_end0:
	.size	handle_rx, .Lfunc_end0-handle_rx

	.section	.text.isr_handler,"ax",@progbits
	.globl	isr_handler
	.p2align	1
	.type	isr_handler,@function
isr_handler:
	;APP
	; @cor24: push r0
	; @cor24: push r1
	; @cor24: push r2
	; @cor24: mov r2, c
	; @cor24: push r2
	;NO_APP
	call	#handle_rx
	;APP
	; @cor24: pop r2
	; @cor24: clu z, r2
	; @cor24: pop r2
	; @cor24: pop r1
	; @cor24: pop r0
	; @cor24: jmp (ir)
	;NO_APP
.Lfunc_end1:
	.size	isr_handler, .Lfunc_end1-isr_handler

	.section	.text.mmio_read,"ax",@progbits
	.globl	mmio_read
	.p2align	1
	.type	mmio_read,@function
mmio_read:
	mov.b	0(r12), r12
	ret
.Lfunc_end2:
	.size	mmio_read, .Lfunc_end2-mmio_read

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
	mov	#63, r12
	call	#uart_putc
	;APP
	; @cor24: la r0, isr_handler
	; @cor24: mov iv, r0
	; @cor24: lc r0, 1
	; @cor24: la r1, -65520
	; @cor24: sb r0, 0(r1)
	;NO_APP
.LBB4_1:
	mov	#256, r12
	call	#mmio_read
	tst	r12
	jne	.LBB4_3
	;APP
	nop

	;NO_APP
	jmp	.LBB4_1
.LBB4_3:
	;APP
	; @cor24: halted:
	; @cor24: bra halted
	;NO_APP
.Lfunc_end4:
	.size	start, .Lfunc_end4-start

	.section	.text.to_upper,"ax",@progbits
	.globl	to_upper
	.p2align	1
	.type	to_upper,@function
to_upper:
	mov	r12, r13
	add	#-97, r13
	cmp	#26, r13
	jhs	.LBB5_2
	and	#95, r12
.LBB5_2:
	ret
.Lfunc_end5:
	.size	to_upper, .Lfunc_end5-to_upper

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
	.ident	"rustc version 1.93.0-nightly (c871d09d1 2025-11-24)"
	.section	".note.GNU-stack","",@progbits
