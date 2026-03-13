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

; --- function: demo_fibonacci_iter ---
demo_fibonacci_iter:
    lc      r0, 10
    ; call fibonacci_iter
    la      r2, .Lret_6
    push    r2
    la      r2, fibonacci_iter
    jmp     (r2)
    .Lret_6:
    mov     r1, r0
    la      r0, 0xFF0000
    ; call mmio_write
    la      r2, .Lret_7
    push    r2
    la      r2, mmio_write
    jmp     (r2)
    .Lret_7:
.LBB1_1:
    bra     .LBB1_1
.Lfunc_end1:

; --- function: fibonacci_iter ---
fibonacci_iter:
    ceq     r0, z
    brt     .LBB2_3
    push    r0
    lc      r0, 1
    sw      r0, 24(fp)
    pop     r0
    lc      r2, 1
.LBB2_2:
    push    r0
    lw      r0, 24(fp)
    sw      r0, 21(fp)
    pop     r0
    push    r0
    lw      r0, 21(fp)
    add     r0, r2
    sw      r0, 21(fp)
    pop     r0
    add     r0, -1
    ceq     r0, z
    mov     r1, r2
    sw      r2, 24(fp)
    lw      r2, 21(fp)
    brf     .LBB2_2
    bra     .LBB2_4
.LBB2_3:
    lc      r1, 1
.LBB2_4:
    mov     r0, r1
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
    ; call demo_fibonacci_iter
    la      r2, .Lret_8
    push    r2
    la      r2, demo_fibonacci_iter
    jmp     (r2)
    .Lret_8:
.Lfunc_end4:

; --- function: uart_putc ---
uart_putc:
    mov     r1, r0
    la      r0, 0xFF0100
    ; tail call mmio_write
    la      r2, mmio_write
    jmp     (r2)
.Lfunc_end5:

