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

; --- function: delay ---
delay:
    sub     sp, 3
    ceq     r0, z
    brt     .LBB1_3
    add     r0, -1
.LBB1_2:
    mov     r2, sp
    sw      r0, 0(r2)
    add     r0, -1
    push    r2
    lc      r2, -1
    ceq     r0, r2
    pop     r2
    brf     .LBB1_2
.LBB1_3:
    add     sp, 3
    jmp     (r1)
.Lfunc_end1:

; --- function: demo_blinky ---
demo_blinky:
.LBB2_1:
    la      r0, 0xFF0000
    push    r0
    lc      r0, 1
    sw      r0, 24(fp)
    pop     r0
    ; call mmio_write
    push    r1
    la      r2, mmio_write
    jal     r1, (r2)
    pop     r1
    la      r0, 0x001388
    ; call delay
    push    r1
    la      r2, delay
    jal     r1, (r2)
    pop     r1
    la      r0, 0xFF0000
    push    r0
    lc      r0, 0
    sw      r0, 24(fp)
    pop     r0
    ; call mmio_write
    push    r1
    la      r2, mmio_write
    jal     r1, (r2)
    pop     r1
    la      r0, 0x001388
    ; call delay
    push    r1
    la      r2, delay
    jal     r1, (r2)
    pop     r1
    bra     .LBB2_1
.Lfunc_end2:

; --- function: mmio_write ---
mmio_write:
    lw      r2, 24(fp)
    sb      r2, 0(r0)
    jmp     (r1)
.Lfunc_end3:

; --- function: start ---
start:
    ; call demo_blinky
    push    r1
    la      r2, demo_blinky
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


