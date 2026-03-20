; COR24 Assembly - Generated from MSP430 via msp430-to-cor24
; Pipeline: Rust -> rustc (msp430-none-elf) -> MSP430 ASM -> COR24 ASM

; Reset vector -> start
    mov     fp, sp
    la      r0, start
    jmp     (r0)

; --- function: isr_handler ---
isr_handler:
    push r0
    push r1
    push r2
    mov r2, c
    push r2
    la r1, -65280
    lb r0, 0(r1)
    mov r2, r0
    lc r0, 33
    ceq r0, r2
    brt do_halt
    lc r0, 97
    clu r2, r0
    brt not_lower
    lc r0, 123
    clu r2, r0
    brf not_lower
    mov r0, r2
    lcu r1, 223
    and r0, r1
    la r1, -65280
    sb r0, 0(r1)
    bra isr_done
not_lower:
    la r1, -65280
    sb r2, 0(r1)
isr_done:
    pop r2
    clu z, r2
    pop r2
    pop r1
    pop r0
    jmp (ir)
do_halt:
    bra do_halt
.Lfunc_end0:

; --- function: mmio_read ---
mmio_read:
    lbu      r0, 0(r0)
    jmp     (r1)
.Lfunc_end1:

; --- function: mmio_write ---
mmio_write:
    lw      r2, 24(fp)
    sb      r2, 0(r0)
    jmp     (r1)
.Lfunc_end2:

; --- function: start ---
start:
    lc      r0, 63
    ; call uart_putc
    push    r1
    la      r2, uart_putc
    jal     r1, (r2)
    pop     r1
    la r0, isr_handler
    mov iv, r0
    lc r0, 1
    la r1, -65520
    sb r0, 0(r1)
.LBB3_1:
    nop

    bra     .LBB3_1
.Lfunc_end3:

; --- function: uart_putc ---
uart_putc:
    sw      r0, 30(fp)
    lw      r0, 18(fp)
    push    r0
    lw      r0, 30(fp)
    sw      r0, 18(fp)
.LBB4_1:
    la      r0, -65279
    ; call mmio_read
    push    r1
    la      r2, mmio_read
    jal     r1, (r2)
    pop     r1
    ceq     r0, z
    brt     .LBB4_1
    la      r0, -65280
    push    r0
    lw      r0, 18(fp)
    sw      r0, 24(fp)
    pop     r0
    ; call mmio_write
    push    r1
    la      r2, mmio_write
    jal     r1, (r2)
    pop     r1
    sw      r0, 30(fp)
    pop     r0
    sw      r0, 18(fp)
    lw      r0, 30(fp)
    jmp     (r1)
.Lfunc_end4:

