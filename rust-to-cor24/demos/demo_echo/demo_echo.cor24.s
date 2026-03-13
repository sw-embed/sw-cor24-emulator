; COR24 Assembly - Generated from MSP430 via msp430-to-cor24
; Pipeline: Rust -> rustc (msp430-none-elf) -> MSP430 ASM -> COR24 ASM

; Reset vector -> start
    mov     fp, sp
    la      r0, start
    jmp     (r0)

; --- function: isr_handler ---
isr_handler:
    push r0
    push r1
    push r2
    mov r2, c
    push r2
    la r1, 0xFF0100
    lb r0, 0(r1)
    mov r2, r0
    lc r0, 0x21
    ceq r0, r2
    brt do_halt
    lc r0, 0x61
    clu r2, r0
    brt not_lower
    lc r0, 0x7B
    clu r2, r0
    brf not_lower
    mov r0, r2
    lc r1, 0xDF
    and r0, r1
    la r1, 0xFF0100
    sb r0, 0(r1)
    bra isr_done
not_lower:
    la r1, 0xFF0100
    sb r2, 0(r1)
isr_done:
    pop r2
    clu z, r2
    pop r2
    pop r1
    pop r0
    jmp (ir)
do_halt:
    bra do_halt
.Lfunc_end0:

; --- function: mmio_write ---
mmio_write:
    sb      r1, 0(r0)
    pop     r2
    jmp     (r2)
.Lfunc_end1:

; --- function: start ---
start:
    lc      r0, 63
    ; call uart_putc
    la      r2, .Lret_0
    push    r2
    la      r2, uart_putc
    jmp     (r2)
    .Lret_0:
    la r0, isr_handler
    mov r6, r0
    lc r0, 1
    la r1, 0xFF0010
    sb r0, 0(r1)
.LBB2_1:
    nop

    bra     .LBB2_1
.Lfunc_end2:

; --- function: uart_putc ---
uart_putc:
    mov     r1, r0
    la      r0, 0xFF0100
    ; tail call mmio_write
    la      r2, mmio_write
    jmp     (r2)
.Lfunc_end3:

