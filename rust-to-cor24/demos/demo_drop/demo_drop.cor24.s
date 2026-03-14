; COR24 Assembly - Generated from MSP430 via msp430-to-cor24
; Pipeline: Rust -> rustc (msp430-none-elf) -> MSP430 ASM -> COR24 ASM

; Reset vector -> start
    mov     fp, sp
    la      r0, start
    jmp     (r0)

; --- function: _RNvCsgMG9zBUy57e_7___rustc17rust_begin_unwind ---
_RNvCsgMG9zBUy57e_7___rustc17rust_begin_unwind:
.LBB0_1:
    bra     .LBB0_1
.Lfunc_end0:

; --- function: _RNvXs_Csdm5oPmm48S1_9demo_dropNtB4_5GuardNtNtNtCshbXD54rZpVC_4core3ops4drop4Drop4drop ---
_RNvXs_Csdm5oPmm48S1_9demo_dropNtB4_5GuardNtNtNtCshbXD54rZpVC_4core3ops4drop4Drop4drop:
    lw      r0, 0(r0)
    push    r0
    lc      r0, 0
    sw      r0, 24(fp)
    pop     r0
    ; tail call mem_write
    la      r2, mem_write
    jmp     (r2)
.Lfunc_end1:

; --- function: guard_new ---
guard_new:
    sw      r0, 30(fp)
    lw      r0, 18(fp)
    push    r0
    lw      r0, 30(fp)
    sw      r0, 18(fp)
    push    r0
    lc      r0, 1
    sw      r0, 24(fp)
    pop     r0
    ; call mem_write
    push    r1
    la      r2, mem_write
    jal     r1, (r2)
    pop     r1
    lw      r0, 18(fp)
    sw      r0, 30(fp)
    pop     r0
    sw      r0, 18(fp)
    lw      r0, 30(fp)
    jmp     (r1)
.Lfunc_end2:

; --- function: mem_write ---
mem_write:
    lw      r2, 24(fp)
    sb      r2, 0(r0)
    jmp     (r1)
.Lfunc_end3:

; --- function: start ---
start:
    sub     sp, 3
    la      r0, 0x000100
    ; call guard_new
    push    r1
    la      r2, guard_new
    jal     r1, (r2)
    pop     r1
    la      r0, 0x000100
    mov     r2, sp
    sw      r0, 0(r2)
    mov     r0, sp
    ; call _RNvXs_Csdm5oPmm48S1_9demo_dropNtB4_5GuardNtNtNtCshbXD54rZpVC_4core3ops4drop4Drop4drop
    push    r1
    la      r2, _RNvXs_Csdm5oPmm48S1_9demo_dropNtB4_5GuardNtNtNtCshbXD54rZpVC_4core3ops4drop4Drop4drop
    jal     r1, (r2)
    pop     r1
    la      r0, 0x000100
    push    r0
    la      r0, 0x0000FF
    sw      r0, 24(fp)
    pop     r0
    ; call mem_write
    push    r1
    la      r2, mem_write
    jal     r1, (r2)
    pop     r1
.LBB4_1:
    bra     .LBB4_1
.Lfunc_end4:


