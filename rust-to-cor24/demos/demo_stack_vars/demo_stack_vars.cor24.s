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
    la      r2, .Lret_0
    push    r2
    la      r2, uart_putc
    jmp     (r2)
    .Lret_0:
    lc      r0, 65
    ; call uart_putc
    la      r2, .Lret_1
    push    r2
    la      r2, uart_putc
    jmp     (r2)
    .Lret_1:
    lc      r0, 78
    ; call uart_putc
    la      r2, .Lret_2
    push    r2
    la      r2, uart_putc
    jmp     (r2)
    .Lret_2:
    lc      r0, 73
    ; call uart_putc
    la      r2, .Lret_3
    push    r2
    la      r2, uart_putc
    jmp     (r2)
    .Lret_3:
    lc      r0, 67
    ; call uart_putc
    la      r2, .Lret_4
    push    r2
    la      r2, uart_putc
    jmp     (r2)
    .Lret_4:
    lc      r0, 10
    ; call uart_putc
    la      r2, .Lret_5
    push    r2
    la      r2, uart_putc
    jmp     (r2)
    .Lret_5:
.LBB0_1:
    bra     .LBB0_1
.Lfunc_end0:

; --- function: accumulate ---
accumulate:
    lw      r1, 6(fp)
    push    r1
    lw      r1, 9(fp)
    push    r1
    lw      r1, 12(fp)
    push    r1
    lw      r1, 15(fp)
    push    r1
    lw      r1, 18(fp)
    push    r1
    sw      r0, 18(fp)
    push    r0
    lw      r0, 18(fp)
    sw      r0, 6(fp)
    pop     r0
    push    r0
    lw      r0, 6(fp)
    add     r0, 1
    sw      r0, 6(fp)
    pop     r0
    push    r0
    lw      r0, 18(fp)
    push    r1
    lw      r1, 6(fp)
    add     r0, r1
    pop     r1
    sw      r0, 18(fp)
    pop     r0
    push    r0
    lw      r0, 18(fp)
    sw      r0, 15(fp)
    pop     r0
    push    r0
    lw      r0, 15(fp)
    push    r1
    lw      r1, 6(fp)
    add     r0, r1
    pop     r1
    sw      r0, 15(fp)
    pop     r0
    lw      r1, 18(fp)
    push    r0
    lw      r0, 6(fp)
    xor     r1, r0
    pop     r0
    push    r0
    lw      r0, 15(fp)
    xor     r1, r0
    pop     r0
    push    r0
    lw      r0, 15(fp)
    sw      r0, 12(fp)
    pop     r0
    push    r0
    lw      r0, 12(fp)
    push    r1
    lw      r1, 18(fp)
    add     r0, r1
    pop     r1
    sw      r0, 12(fp)
    pop     r0
    push    r0
    lw      r0, 12(fp)
    xor     r1, r0
    pop     r0
    push    r0
    lw      r0, 12(fp)
    sw      r0, 9(fp)
    pop     r0
    push    r0
    lw      r0, 9(fp)
    push    r1
    lw      r1, 15(fp)
    add     r0, r1
    pop     r1
    sw      r0, 9(fp)
    pop     r0
    push    r0
    lw      r0, 9(fp)
    xor     r1, r0
    pop     r0
    la      r0, 0xFF0000
    ; call mmio_write
    la      r2, .Lret_6
    push    r2
    la      r2, mmio_write
    jmp     (r2)
    .Lret_6:
    lw      r0, 6(fp)
    ; call uart_putc
    la      r2, .Lret_7
    push    r2
    la      r2, uart_putc
    jmp     (r2)
    .Lret_7:
    lw      r0, 18(fp)
    ; call uart_putc
    la      r2, .Lret_8
    push    r2
    la      r2, uart_putc
    jmp     (r2)
    .Lret_8:
    lw      r0, 15(fp)
    ; call uart_putc
    la      r2, .Lret_9
    push    r2
    la      r2, uart_putc
    jmp     (r2)
    .Lret_9:
    lw      r0, 12(fp)
    ; call uart_putc
    la      r2, .Lret_10
    push    r2
    la      r2, uart_putc
    jmp     (r2)
    .Lret_10:
    lw      r0, 9(fp)
    ; call uart_putc
    la      r2, .Lret_11
    push    r2
    la      r2, uart_putc
    jmp     (r2)
    .Lret_11:
.LBB1_1:
    bra     .LBB1_1
.Lfunc_end1:

; --- function: demo_stack_vars ---
demo_stack_vars:
    la      r0, 0xFF0000
    ; call mmio_read
    la      r2, .Lret_12
    push    r2
    la      r2, mmio_read
    jmp     (r2)
    .Lret_12:
    add     r0, 1
    ; call accumulate
    la      r2, .Lret_13
    push    r2
    la      r2, accumulate
    jmp     (r2)
    .Lret_13:
.Lfunc_end2:

; --- function: mmio_read ---
mmio_read:
    lbu      r0, 0(r0)
    pop     r2
    jmp     (r2)
.Lfunc_end3:

; --- function: mmio_write ---
mmio_write:
    sb      r1, 0(r0)
    pop     r2
    jmp     (r2)
.Lfunc_end4:

; --- function: start ---
start:
    ; call demo_stack_vars
    la      r2, .Lret_14
    push    r2
    la      r2, demo_stack_vars
    jmp     (r2)
    .Lret_14:
.Lfunc_end5:

; --- function: uart_putc ---
uart_putc:
    mov     r1, r0
    la      r0, 0xFF0100
    ; tail call mmio_write
    la      r2, mmio_write
    jmp     (r2)
.Lfunc_end6:

