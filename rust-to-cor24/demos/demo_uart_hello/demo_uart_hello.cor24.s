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

; --- function: demo_uart_hello ---
demo_uart_hello:
    lc      r0, 72
    ; call uart_putc
    la      r2, .Lret_6
    push    r2
    la      r2, uart_putc
    jmp     (r2)
    .Lret_6:
    lc      r0, 101
    ; call uart_putc
    la      r2, .Lret_7
    push    r2
    la      r2, uart_putc
    jmp     (r2)
    .Lret_7:
    lc      r0, 108
    ; call uart_putc
    la      r2, .Lret_8
    push    r2
    la      r2, uart_putc
    jmp     (r2)
    .Lret_8:
    lc      r0, 108
    ; call uart_putc
    la      r2, .Lret_9
    push    r2
    la      r2, uart_putc
    jmp     (r2)
    .Lret_9:
    lc      r0, 111
    ; call uart_putc
    la      r2, .Lret_10
    push    r2
    la      r2, uart_putc
    jmp     (r2)
    .Lret_10:
    lc      r0, 10
    ; call uart_putc
    la      r2, .Lret_11
    push    r2
    la      r2, uart_putc
    jmp     (r2)
    .Lret_11:
.LBB1_1:
    bra     .LBB1_1
.Lfunc_end1:

; --- function: mmio_write ---
mmio_write:
    sb      r1, 0(r0)
    pop     r2
    jmp     (r2)
.Lfunc_end2:

; --- function: start ---
start:
    ; call demo_uart_hello
    la      r2, .Lret_12
    push    r2
    la      r2, demo_uart_hello
    jmp     (r2)
    .Lret_12:
.Lfunc_end3:

; --- function: uart_putc ---
uart_putc:
    mov     r1, r0
    la      r0, 0xFF0100
    ; tail call mmio_write
    la      r2, mmio_write
    jmp     (r2)
.Lfunc_end4:

