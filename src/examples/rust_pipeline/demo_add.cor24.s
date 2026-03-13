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

; --- function: demo_add ---
demo_add:
    la      r0, 0x000156
    pop     r2
    jmp     (r2)
.Lfunc_end1:

; --- function: mmio_write ---
mmio_write:
    sb      r1, 0(r0)
    pop     r2
    jmp     (r2)
.Lfunc_end2:

; --- function: start ---
start:
    la      r0, 0xFF0000
    la      r1, 0x000156
    ; call mmio_write
    la      r2, .Lret_6
    push    r2
    la      r2, mmio_write
    jmp     (r2)
    .Lret_6:
.LBB3_1:
    bra     .LBB3_1
.Lfunc_end3:

; --- function: uart_putc ---
uart_putc:
    mov     r1, r0
    la      r0, 0xFF0100
    ; tail call mmio_write
    la      r2, mmio_write
    jmp     (r2)
.Lfunc_end4:

