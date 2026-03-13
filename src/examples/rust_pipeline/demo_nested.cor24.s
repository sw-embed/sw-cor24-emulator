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

; --- function: demo_nested ---
demo_nested:
    la      r0, 0xFF0000
    ; call mmio_read
    la      r2, .Lret_6
    push    r2
    la      r2, mmio_read
    jmp     (r2)
    .Lret_6:
    add     r0, 5
    ; call level_a
    la      r2, .Lret_7
    push    r2
    la      r2, level_a
    jmp     (r2)
    .Lret_7:
.Lfunc_end1:

; --- function: level_a ---
level_a:
    add     r0, 10
    ; call level_b
    la      r2, .Lret_8
    push    r2
    la      r2, level_b
    jmp     (r2)
    .Lret_8:
.Lfunc_end2:

; --- function: level_b ---
level_b:
    mov     r1, r0
    add     r0, r0
    add     r0, 3
    ; call level_c
    la      r2, .Lret_9
    push    r2
    la      r2, level_c
    jmp     (r2)
    .Lret_9:
.Lfunc_end3:

; --- function: level_c ---
level_c:
    lw      r1, 18(fp)
    push    r1
    sw      r1, 18(fp)
    mov     r1, r0
    la      r0, 0xFF0000
    ; call mmio_write
    la      r2, .Lret_10
    push    r2
    la      r2, mmio_write
    jmp     (r2)
    .Lret_10:
    lw      r0, 18(fp)
    ; call uart_putc
    la      r2, .Lret_11
    push    r2
    la      r2, uart_putc
    jmp     (r2)
    .Lret_11:
.LBB4_1:
    bra     .LBB4_1
.Lfunc_end4:

; --- function: mmio_read ---
mmio_read:
    lbu      r0, 0(r0)
    pop     r2
    jmp     (r2)
.Lfunc_end5:

; --- function: mmio_write ---
mmio_write:
    sb      r1, 0(r0)
    pop     r2
    jmp     (r2)
.Lfunc_end6:

; --- function: start ---
start:
    ; call demo_nested
    la      r2, .Lret_12
    push    r2
    la      r2, demo_nested
    jmp     (r2)
    .Lret_12:
.Lfunc_end7:

; --- function: uart_putc ---
uart_putc:
    mov     r1, r0
    la      r0, 0xFF0100
    ; tail call mmio_write
    la      r2, mmio_write
    jmp     (r2)
.Lfunc_end8:

