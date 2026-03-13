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
    la      r2, .Lret_0
    push    r2
    la      r2, mmio_read
    jmp     (r2)
    .Lret_0:
    push    r1
    lc      r1, 33
    ceq     r0, r1
    pop     r1
    brf     .LBB0_2
    la      r0, 0x000100
    lc      r1, 1
    ; tail call mmio_write
    la      r2, mmio_write
    jmp     (r2)
.LBB0_2:
    ; call to_upper
    la      r2, .Lret_1
    push    r2
    la      r2, to_upper
    jmp     (r2)
    .Lret_1:
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
    la      r2, .Lret_2
    push    r2
    la      r2, handle_rx
    jmp     (r2)
    .Lret_2:
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
    pop     r2
    jmp     (r2)
.Lfunc_end2:

; --- function: mmio_write ---
mmio_write:
    sb      r1, 0(r0)
    pop     r2
    jmp     (r2)
.Lfunc_end3:

; --- function: start ---
start:
    lc      r0, 63
    ; call uart_putc
    la      r2, .Lret_3
    push    r2
    la      r2, uart_putc
    jmp     (r2)
    .Lret_3:
    la r0, isr_handler
    mov r6, r0
    lc r0, 1
    la r1, 0xFF0010
    sb r0, 0(r1)
.LBB4_1:
    la      r0, 0x000100
    ; call mmio_read
    la      r2, .Lret_4
    push    r2
    la      r2, mmio_read
    jmp     (r2)
    .Lret_4:
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
    mov     r1, r0
    add     r1, -97
    push    r0
    lc      r0, 26
    clu     r1, r0
    pop     r0
    brf     .LBB5_2
    lc      r1, 95
    and     r0, r1
.LBB5_2:
    pop     r2
    jmp     (r2)
.Lfunc_end5:

; --- function: uart_putc ---
uart_putc:
    mov     r1, r0
    la      r0, 0xFF0100
    ; tail call mmio_write
    la      r2, mmio_write
    jmp     (r2)
.Lfunc_end6:
