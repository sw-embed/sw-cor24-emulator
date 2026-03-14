; COR24 Assembly - Generated from MSP430 via msp430-to-cor24
; Pipeline: Rust -> rustc (msp430-none-elf) -> MSP430 ASM -> COR24 ASM

; Reset vector -> start
    mov     fp, sp
    la      r0, start
    jmp     (r0)

; --- function: handle_rx ---
handle_rx:
    la      r0, 0xFF0100
    ; call mmio_read
    push    r1
    la      r2, mmio_read
    jal     r1, (r2)
    pop     r1
    push    r2
    lc      r2, 33
    ceq     r0, r2
    pop     r2
    brf     .LBB0_2
    la      r0, 0x000100
    push    r0
    lc      r0, 1
    sw      r0, 24(fp)
    pop     r0
    ; tail call mmio_write
    la      r2, mmio_write
    jmp     (r2)
.LBB0_2:
    ; call to_upper
    push    r1
    la      r2, to_upper
    jal     r1, (r2)
    pop     r1
    ; tail call uart_putc
    la      r2, uart_putc
    jmp     (r2)
.Lfunc_end0:

; --- function: isr_handler ---
isr_handler:
    push r0
    push r1
    push r2
    mov r2, c
    push r2
    ; call handle_rx
    push    r1
    la      r2, handle_rx
    jal     r1, (r2)
    pop     r1
    pop r2
    clu z, r2
    pop r2
    pop r1
    pop r0
    jmp (ir)
.Lfunc_end1:

; --- function: mmio_read ---
mmio_read:
    lbu      r0, 0(r0)
    jmp     (r1)
.Lfunc_end2:

; --- function: mmio_write ---
mmio_write:
    lw      r2, 24(fp)
    sb      r2, 0(r0)
    jmp     (r1)
.Lfunc_end3:

; --- function: start ---
start:
    lc      r0, 63
    ; call uart_putc
    push    r1
    la      r2, uart_putc
    jal     r1, (r2)
    pop     r1
    la r0, isr_handler
    mov r6, r0
    lc r0, 1
    la r1, 0xFF0010
    sb r0, 0(r1)
.LBB4_1:
    la      r0, 0x000100
    ; call mmio_read
    push    r1
    la      r2, mmio_read
    jal     r1, (r2)
    pop     r1
    ceq     r0, z
    brf     .LBB4_3
    nop

    bra     .LBB4_1
.LBB4_3:
halted:
    bra halted
.Lfunc_end4:

; --- function: to_upper ---
to_upper:
    sw      r0, 24(fp)
    push    r0
    lw      r0, 24(fp)
    add     r0, -97
    sw      r0, 24(fp)
    pop     r0
    push    r0
    lw      r0, 24(fp)
    push    r2
    lc      r2, 26
    clu     r0, r2
    pop     r2
    pop     r0
    brf     .LBB5_2
    lc      r2, 95
    and     r0, r2
.LBB5_2:
    jmp     (r1)
.Lfunc_end5:

; --- function: uart_putc ---
uart_putc:
    sw      r0, 24(fp)
    la      r0, 0xFF0100
    ; tail call mmio_write
    la      r2, mmio_write
    jmp     (r2)
.Lfunc_end6:


