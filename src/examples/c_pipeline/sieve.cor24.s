; Sieve of Eratosthenes — COR24 C compiler output
; Source: sieve.c compiled by MakerLisp's cc24/as24 toolchain
; Calling convention: push fp,r2,r1; mov fp,sp; args on stack at 9(fp)
;
; All functions are authentic compiler output.
; UART address changed from -65280 to 0xFF0100 for emulator.
; Iterations reduced from 1000 to 1 for web demo speed.

; --- Reset vector ---
    mov     fp, sp
    la      r0, _main
    jal     r1, (r0)
_halt:
    bra     _halt

; =============================================
; _putchr: write one byte to UART (compiler output, UART addr adapted)
; =============================================
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
; _printn: print integer as decimal (compiler output)
; Uses divdec[] table for division
; =============================================
_printn:
    push    fp
    push    r2
    push    r1
    mov     fp, sp
    add     sp, -9
    lw      r2, 9(fp)
; line 23
    lc      r0, 0
    sw      r0, -6(fp)
L20:
; line 24 — for (i = 0; i < 3; i++)
    lw      r0, -6(fp)
    lc      r1, 3
    cls     r0, r1
    brf     L21
; line 25
    lc      r0, 0
    sw      r0, -9(fp)
; line 26 — d = divdec[i]
    lw      r0, -6(fp)
    mov     r1, r0
    add     r1, r1
    add     r0, r1
    la      r1, _divdec
    add     r0, r1
    lw      r0, (r0)
    sw      r0, -3(fp)
L22:
; line 27 — while (n >= d)
    lw      r0, -3(fp)
    cls     r2, r0
    brt     L23
; line 28 — n -= d
    lw      r0, -3(fp)
    sub     r2, r0
; line 29 — q++
    lw      r0, -9(fp)
    add     r0, 1
    sw      r0, -9(fp)
    bra     L22
L23:
; line 31 — putchr(q + '0')
    lw      r0, -9(fp)
    add     r0, 48
    push    r0
    la      r0, _putchr
    jal     r1, (r0)
    add     sp, 3
; line 32
    lw      r0, -6(fp)
    add     r0, 1
    sw      r0, -6(fp)
    bra     L20
L21:
; line 34 — putchr(n + '0')
    mov     r0, r2
    add     r0, 48
    push    r0
    la      r0, _putchr
    jal     r1, (r0)
    mov     sp, fp
    pop     r1
    pop     r2
    pop     fp
    jmp     (r1)

; =============================================
; _putstr: print null-terminated string (compiler output)
; =============================================
_putstr:
    push    fp
    push    r2
    push    r1
    mov     fp, sp
    lw      r2, 9(fp)
L26:
; line 39
    lb      r0, (r2)
    clu     z, r0
    brf     L27
; line 40
    lb      r0, (r2)
    push    r0
    la      r0, _putchr
    jal     r1, (r0)
    add     sp, 3
; line 41
    add     r2, 1
    bra     L26
L27:
    mov     sp, fp
    pop     r1
    pop     r2
    pop     fp
    jmp     (r1)

; =============================================
; _main: sieve entry point (compiler output, 1 iteration for demo)
; =============================================
    .globl  _main
_main:
    push    fp
    push    r2
    push    r1
    mov     fp, sp
    add     sp, -12
; line 51 — putstr("1 iteration\n")
    la      r0, L31
    push    r0
    la      r0, _putstr
    jal     r1, (r0)
    add     sp, 3
; line 53 — for (iter = 1; iter <= 1; iter++)
    lc      r0, 1
    sw      r0, -12(fp)
L34:
    lw      r0, -12(fp)
    lc      r1, 1
    cls     r1, r0
    brt     L33
; line 54 — count = 0
    lc      r0, 0
    sw      r0, -9(fp)
; line 55 — for (i = 0; i <= 8190; i++) flags[i] = 1
    lc      r2, 0
L37:
    la      r0, 8190
    cls     r0, r2
    brt     L36
    la      r0, _flags
    add     r0, r2
    lc      r1, 1
    sb      r1, (r0)
    add     r2, 1
    bra     L37
L36:
; line 57 — for (i = 0; i <= 8190; i++)
    lc      r2, 0
L40:
    la      r0, 8190
    cls     r0, r2
    brt     L39
; line 58 — if (flags[i])
    la      r0, _flags
    add     r0, r2
    lb      r0, (r0)
    clu     z, r0
    brf     L41
; line 59 — prime = i + i + 3
    mov     r0, r2
    add     r0, r2
    add     r0, 3
    sw      r0, -3(fp)
; line 60 — k = i + prime
    lw      r0, -3(fp)
    add     r0, r2
    sw      r0, -6(fp)
L44:
; line 60 — while (k <= 8190)
    lw      r0, -6(fp)
    la      r1, 8190
    cls     r1, r0
    brt     L43
; line 61 — flags[k] = 0
    lw      r0, -6(fp)
    la      r1, _flags
    add     r0, r1
    lc      r1, 0
    sb      r1, (r0)
; line 61 — k += prime
    lw      r0, -6(fp)
    lw      r1, -3(fp)
    add     r0, r1
    sw      r0, -6(fp)
    bra     L44
L43:
; line 62 — count++
    lw      r0, -9(fp)
    add     r0, 1
    sw      r0, -9(fp)
L41:
; line 64
    add     r2, 1
    bra     L40
L39:
; line 64
    lw      r0, -12(fp)
    add     r0, 1
    sw      r0, -12(fp)
    bra     L34
L33:
; line 66 — printn(count)
    lw      r0, -9(fp)
    push    r0
    la      r0, _printn
    jal     r1, (r0)
    add     sp, 3
; line 67 — putstr(" primes.\n")
    la      r0, L45
    push    r0
    la      r0, _putstr
    jal     r1, (r0)
    add     sp, 3
; line 69 — return 0
    lc      r0, 0
    mov     sp, fp
    pop     r1
    pop     r2
    pop     fp
    jmp     (r1)

; =============================================
; Data section
; =============================================
_divdec:
    .word   1000
    .word   100
    .word   10
    .comm   _flags, 8191
L31:
; "1 iteration\n\0"
    .byte   49, 32, 105, 116, 101, 114, 97, 116
    .byte   105, 111, 110, 10, 0
L45:
; " primes.\n\0"
    .byte   32, 112, 114, 105, 109, 101, 115, 46
    .byte   10, 0
