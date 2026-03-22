; Fibonacci — COR24 C compiler output + runtime stubs
; Source: fib.c compiled by MakerLisp's cc24/as24 toolchain
; Calling convention: push fp,r2,r1; mov fp,sp; args on stack at 9(fp)
;
; _fib and _main are authentic compiler output (fib(10) variant).
; _printf, _printn, _putchr are injected runtime stubs that output
; via the COR24 UART so the program runs on the emulator.

; --- Reset vector ---
    mov     fp, sp
    la      r0, _main
    jal     r1, (r0)
_halt:
    bra     _halt

; =============================================
; _fib: recursive Fibonacci (compiler output)
; int fib(int n) { if (n<2) return 1; return fib(n-1)+fib(n-2); }
; =============================================
_fib:
    push    fp
    push    r2
    push    r1
    mov     fp, sp
    add     sp, -3
    lw      r2, 9(fp)
; line 7, file "fib.c"
    lc      r0, 2
    cls     r2, r0
    brf     L17
; line 8, file "fib.c"
    lc      r0, 1
    bra     L16
L17:
; line 11, file "fib.c"
    mov     r0, r2
    add     r0, -1
    push    r0
    la      r0, _fib
    jal     r1, (r0)
    add     sp, 3
    sw      r0, -3(fp)
    mov     r0, r2
    add     r0, -2
    push    r0
    la      r0, _fib
    jal     r1, (r0)
    add     sp, 3
    lw      r1, -3(fp)
    add     r0, r1
L16:
    mov     sp, fp
    pop     r1
    pop     r2
    pop     fp
    jmp     (r1)

; =============================================
; _main: entry point (compiler output, modified for fib(10))
; =============================================
_main:
    push    fp
    push    r2
    push    r1
    mov     fp, sp
    add     sp, -3
; printf("Fibonacci 10\n")
    la      r0, L20
    push    r0
    la      r0, _printf
    jal     r1, (r0)
    add     sp, 3
; result = fib(10)
    lc      r0, 10
    push    r0
    la      r0, _fib
    jal     r1, (r0)
    add     sp, 3
    sw      r0, -3(fp)
; printf("%d\n", result)
    lw      r0, -3(fp)
    push    r0
    la      r0, L21
    push    r0
    la      r0, _printf
    jal     r1, (r0)
    add     sp, 6
; return 0
    lc      r0, 0
    mov     sp, fp
    pop     r1
    pop     r2
    pop     fp
    jmp     (r1)

; =============================================
; Runtime stubs (injected for emulator compatibility)
; =============================================

; _printf: minimal printf supporting %d and literal chars
; Args: 9(fp) = format string pointer, 12(fp)+ = varargs
_printf:
    push    fp
    push    r2
    push    r1
    mov     fp, sp
    add     sp, -3
    lw      r2, 9(fp)
    lc      r0, 12
    sw      r0, -3(fp)
.pf_loop:
    lb      r0, (r2)
    clu     z, r0
    brf     .pf_done
    lc      r1, 37
    ceq     r0, r1
    brt     .pf_percent
    push    r2
    push    r0
    la      r0, _putchr
    jal     r1, (r0)
    add     sp, 3
    pop     r2
    add     r2, 1
    bra     .pf_loop
.pf_percent:
    add     r2, 1
    lb      r0, (r2)
    lc      r1, 100
    ceq     r0, r1
    brt     .pf_d
    push    r2
    push    r0
    la      r0, _putchr
    jal     r1, (r0)
    add     sp, 3
    pop     r2
    add     r2, 1
    bra     .pf_loop
.pf_d:
    push    r2
    lw      r0, -3(fp)
    add     r0, fp
    lw      r0, (r0)
    push    r0
    la      r0, _printn
    jal     r1, (r0)
    add     sp, 3
    lw      r0, -3(fp)
    add     r0, 3
    sw      r0, -3(fp)
    pop     r2
    add     r2, 1
    bra     .pf_loop
.pf_done:
    mov     sp, fp
    pop     r1
    pop     r2
    pop     fp
    jmp     (r1)

; _printn: print integer as decimal (recursive)
; Args: 9(fp) = integer to print
_printn:
    push    fp
    push    r2
    push    r1
    mov     fp, sp
    lw      r2, 9(fp)
    lc      r0, 10
    cls     r2, r0
    brt     .pn_single
    lc      r0, 0
.pn_div:
    lc      r1, 10
    cls     r2, r1
    brt     .pn_divd
    sub     r2, r1
    add     r0, 1
    bra     .pn_div
.pn_divd:
    push    r2
    push    r0
    la      r0, _printn
    jal     r1, (r0)
    add     sp, 3
    pop     r2
.pn_single:
    mov     r0, r2
    add     r0, 48
    push    r0
    la      r0, _putchr
    jal     r1, (r0)
    add     sp, 3
    mov     sp, fp
    pop     r1
    pop     r2
    pop     fp
    jmp     (r1)

; _putchr: write one byte to UART
; Args: 9(fp) = character to output
_putchr:
    push    fp
    push    r2
    push    r1
    mov     fp, sp
    la      r2, -65280
.pc_wait:
    lb      r0, 1(r2)       ; read status (sign-extended)
    cls     r0, z
    brt     .pc_wait        ; spin while TX busy (bit 7 = negative)
    lb      r0, 9(fp)
    sb      r0, (r2)
    mov     sp, fp
    pop     r1
    pop     r2
    pop     fp
    jmp     (r1)

; =============================================
; Data section (format strings)
; =============================================
L20:
; "Fibonacci 10\n\0"
    .byte   70, 105, 98, 111, 110, 97, 99, 99
    .byte   105, 32, 49, 48, 10, 0
L21:
; "%d\n\0"
    .byte   37, 100, 10, 0
