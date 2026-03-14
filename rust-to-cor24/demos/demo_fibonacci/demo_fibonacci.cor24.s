; COR24 Assembly - Generated from MSP430 via msp430-to-cor24
; Pipeline: Rust -> rustc (msp430-none-elf) -> MSP430 ASM -> COR24 ASM

; Reset vector -> start
    mov     fp, sp
    la      r0, start
    jmp     (r0)

; --- function: _RNvCsgMG9zBUy57e_7___rustc17rust_begin_unwind ---
_RNvCsgMG9zBUy57e_7___rustc17rust_begin_unwind:
    lc      r0, 80
    ; call uart_putc
    push    r1
    la      r2, uart_putc
    jal     r1, (r2)
    pop     r1
    lc      r0, 65
    ; call uart_putc
    push    r1
    la      r2, uart_putc
    jal     r1, (r2)
    pop     r1
    lc      r0, 78
    ; call uart_putc
    push    r1
    la      r2, uart_putc
    jal     r1, (r2)
    pop     r1
    lc      r0, 73
    ; call uart_putc
    push    r1
    la      r2, uart_putc
    jal     r1, (r2)
    pop     r1
    lc      r0, 67
    ; call uart_putc
    push    r1
    la      r2, uart_putc
    jal     r1, (r2)
    pop     r1
    lc      r0, 10
    ; call uart_putc
    push    r1
    la      r2, uart_putc
    jal     r1, (r2)
    pop     r1
.LBB0_1:
    bra     .LBB0_1
.Lfunc_end0:

; --- function: demo_fibonacci ---
demo_fibonacci:
    lc      r0, 10
    ; call fibonacci
    push    r1
    la      r2, fibonacci
    jal     r1, (r2)
    pop     r1
    sw      r0, 24(fp)
    la      r0, 0xFF0000
    ; call mmio_write
    push    r1
    la      r2, mmio_write
    jal     r1, (r2)
    pop     r1
.LBB1_1:
    bra     .LBB1_1
.Lfunc_end1:

; --- function: fibonacci ---
fibonacci:
    sw      r0, 30(fp)
    lw      r0, 15(fp)
    push    r0
    lw      r0, 30(fp)
    sw      r0, 30(fp)
    lw      r0, 18(fp)
    push    r0
    lw      r0, 30(fp)
    sw      r0, 15(fp)
    push    r0
    lc      r0, 1
    sw      r0, 18(fp)
    pop     r0
    push    r0
    lw      r0, 15(fp)
    push    r2
    lc      r2, 2
    clu     r0, r2
    pop     r2
    pop     r0
    brt     .LBB2_4
    push    r0
    lc      r0, 0
    sw      r0, 18(fp)
    pop     r0
.LBB2_2:
    lw      r0, 15(fp)
    add     r0, -1
    ; call fibonacci
    push    r1
    la      r2, fibonacci
    jal     r1, (r2)
    pop     r1
    push    r2
    lw      r2, 18(fp)
    add     r2, r0
    sw      r2, 18(fp)
    pop     r2
    push    r0
    lw      r0, 15(fp)
    add     r0, -2
    sw      r0, 15(fp)
    pop     r0
    push    r0
    lw      r0, 15(fp)
    push    r2
    lc      r2, 2
    clu     r0, r2
    pop     r2
    pop     r0
    brf     .LBB2_2
    push    r0
    lw      r0, 18(fp)
    add     r0, 1
    sw      r0, 18(fp)
    pop     r0
.LBB2_4:
    lw      r0, 18(fp)
    sw      r0, 30(fp)
    pop     r0
    sw      r0, 18(fp)
    lw      r0, 30(fp)
    sw      r0, 30(fp)
    pop     r0
    sw      r0, 15(fp)
    lw      r0, 30(fp)
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
    ; call demo_fibonacci
    push    r1
    la      r2, demo_fibonacci
    jal     r1, (r2)
    pop     r1
.Lfunc_end4:

; --- function: uart_putc ---
uart_putc:
    sw      r0, 24(fp)
    la      r0, 0xFF0100
    ; tail call mmio_write
    la      r2, mmio_write
    jmp     (r2)
.Lfunc_end5:


