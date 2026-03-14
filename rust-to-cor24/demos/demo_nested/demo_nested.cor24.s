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

; --- function: demo_nested ---
demo_nested:
    la      r0, 0xFF0000
    ; call mmio_read
    push    r1
    la      r2, mmio_read
    jal     r1, (r2)
    pop     r1
    add     r0, 5
    ; call level_a
    push    r1
    la      r2, level_a
    jal     r1, (r2)
    pop     r1
.Lfunc_end1:

; --- function: level_a ---
level_a:
    add     r0, 10
    ; call level_b
    push    r1
    la      r2, level_b
    jal     r1, (r2)
    pop     r1
.Lfunc_end2:

; --- function: level_b ---
level_b:
    sw      r0, 24(fp)
    add     r0, r0
    add     r0, 3
    ; call level_c
    push    r1
    la      r2, level_c
    jal     r1, (r2)
    pop     r1
.Lfunc_end3:

; --- function: level_c ---
level_c:
    sw      r0, 30(fp)
    lw      r0, 18(fp)
    push    r0
    lw      r0, 30(fp)
    push    r0
    lw      r0, 24(fp)
    sw      r0, 18(fp)
    pop     r0
    sw      r0, 24(fp)
    la      r0, 0xFF0000
    ; call mmio_write
    push    r1
    la      r2, mmio_write
    jal     r1, (r2)
    pop     r1
    lw      r0, 18(fp)
    ; call uart_putc
    push    r1
    la      r2, uart_putc
    jal     r1, (r2)
    pop     r1
.LBB4_1:
    bra     .LBB4_1
.Lfunc_end4:

; --- function: mmio_read ---
mmio_read:
    lbu      r0, 0(r0)
    jmp     (r1)
.Lfunc_end5:

; --- function: mmio_write ---
mmio_write:
    lw      r2, 24(fp)
    sb      r2, 0(r0)
    jmp     (r1)
.Lfunc_end6:

; --- function: start ---
start:
    ; call demo_nested
    push    r1
    la      r2, demo_nested
    jal     r1, (r2)
    pop     r1
.Lfunc_end7:

; --- function: uart_putc ---
uart_putc:
    sw      r0, 24(fp)
    la      r0, 0xFF0100
    ; tail call mmio_write
    la      r2, mmio_write
    jmp     (r2)
.Lfunc_end8:


