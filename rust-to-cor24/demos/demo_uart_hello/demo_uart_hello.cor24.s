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

; --- function: demo_uart_hello ---
demo_uart_hello:
    lc      r0, 72
    ; call uart_putc
    push    r1
    la      r2, uart_putc
    jal     r1, (r2)
    pop     r1
    lc      r0, 101
    ; call uart_putc
    push    r1
    la      r2, uart_putc
    jal     r1, (r2)
    pop     r1
    lc      r0, 108
    ; call uart_putc
    push    r1
    la      r2, uart_putc
    jal     r1, (r2)
    pop     r1
    lc      r0, 108
    ; call uart_putc
    push    r1
    la      r2, uart_putc
    jal     r1, (r2)
    pop     r1
    lc      r0, 111
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
.LBB1_1:
    bra     .LBB1_1
.Lfunc_end1:

; --- function: mmio_write ---
mmio_write:
    lw      r2, 24(fp)
    sb      r2, 0(r0)
    jmp     (r1)
.Lfunc_end2:

; --- function: start ---
start:
    ; call demo_uart_hello
    push    r1
    la      r2, demo_uart_hello
    jal     r1, (r2)
    pop     r1
.Lfunc_end3:

; --- function: uart_putc ---
uart_putc:
    sw      r0, 24(fp)
    la      r0, 0xFF0100
    ; tail call mmio_write
    la      r2, mmio_write
    jmp     (r2)
.Lfunc_end4:


