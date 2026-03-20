	.file	"demo_echo.ba9e214e17d3d37e-cgu.0"
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
	; @cor24: la r1, -65280
	; @cor24: lb r0, 0(r1)
	; @cor24: mov r2, r0
	; @cor24: lc r0, 33
	; @cor24: ceq r0, r2
	; @cor24: brt do_halt
	; @cor24: lc r0, 97
	; @cor24: clu r2, r0
	; @cor24: brt not_lower
	; @cor24: lc r0, 123
	; @cor24: clu r2, r0
	; @cor24: brf not_lower
	; @cor24: mov r0, r2
	; @cor24: lcu r1, 223
	; @cor24: and r0, r1
	; @cor24: la r1, -65280
	; @cor24: sb r0, 0(r1)
	; @cor24: bra isr_done
	; @cor24: not_lower:
	; @cor24: la r1, -65280
	; @cor24: sb r2, 0(r1)
	; @cor24: isr_done:
	; @cor24: pop r2
	; @cor24: clu z, r2
	; @cor24: pop r2
	; @cor24: pop r1
	; @cor24: pop r0
	; @cor24: jmp (ir)
	; @cor24: do_halt:
	; @cor24: bra do_halt
	;NO_APP
.Lfunc_end0:
	.size	isr_handler, .Lfunc_end0-isr_handler

	.section	.text.mmio_read,"ax",@progbits
	.globl	mmio_read
	.p2align	1
	.type	mmio_read,@function
mmio_read:
	mov.b	0(r12), r12
	ret
.Lfunc_end1:
	.size	mmio_read, .Lfunc_end1-mmio_read

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
	mov	#63, r12
	call	#uart_putc
	;APP
	; @cor24: la r0, isr_handler
	; @cor24: mov iv, r0
	;NO_APP
	;APP
	; @cor24: lc r0, 1
	; @cor24: la r1, -65520
	; @cor24: sb r0, 0(r1)
	;NO_APP
.LBB3_1:
	;APP
	nop

	;NO_APP
	jmp	.LBB3_1
.Lfunc_end3:
	.size	start, .Lfunc_end3-start

	.section	.text.uart_putc,"ax",@progbits
	.globl	uart_putc
	.p2align	1
	.type	uart_putc,@function
uart_putc:
	push	r10
	mov	r12, r10
.LBB4_1:
	mov	#-254, r12
	call	#mmio_read
	tst.b	r12
	jl	.LBB4_1
	mov	#-255, r12
	mov	r10, r13
	call	#mmio_write
	pop	r10
	ret
.Lfunc_end4:
	.size	uart_putc, .Lfunc_end4-uart_putc

	.ident	"rustc version 1.93.0-nightly (c871d09d1 2025-11-24)"
	.ident	"rustc version 1.93.0-nightly (c871d09d1 2025-11-24)"
	.section	".note.GNU-stack","",@progbits
