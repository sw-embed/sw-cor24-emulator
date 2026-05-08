; SPI data exchange
        .text

        .globl _spixchg

_spixchg:
        push    fp
        push    r2
        push    r1
        mov     fp,sp

        la      r2,0ff0030h     ; SPI base address

; Write and read 8 bits
        lc      r0,0
        sb      r0,1(r2)        ; 0 to SCLK
        lb      r1,9(fp)        ; data to write
        cls     r1,z
        add     r1,r1
        sb      r1,9(fp)
        mov     r1,c
        sb      r1,0(r2)        ; MSB to MOSI
        lc      r1,1
        sb      r1,1(r2)        ; 1 to SCLK
        lb      r1,0(r2)        ; LSB from MISO
        or      r0,r1

        add     r0,r0
        sb      r0,1(r2)        ; 0 to SCLK
        lb      r1,9(fp)        ; data to write
        cls     r1,z
        add     r1,r1
        sb      r1,9(fp)
        mov     r1,c
        sb      r1,0(r2)        ; MSB to MOSI
        lc      r1,1
        sb      r1,1(r2)        ; 1 to SCLK
        lb      r1,0(r2)        ; LSB from MISO
        or      r0,r1

        add     r0,r0
        sb      r0,1(r2)        ; 0 to SCLK
        lb      r1,9(fp)        ; data to write
        cls     r1,z
        add     r1,r1
        sb      r1,9(fp)
        mov     r1,c
        sb      r1,0(r2)        ; MSB to MOSI
        lc      r1,1
        sb      r1,1(r2)        ; 1 to SCLK
        lb      r1,0(r2)        ; LSB from MISO
        or      r0,r1

        add     r0,r0
        sb      r0,1(r2)        ; 0 to SCLK
        lb      r1,9(fp)        ; data to write
        cls     r1,z
        add     r1,r1
        sb      r1,9(fp)
        mov     r1,c
        sb      r1,0(r2)        ; MSB to MOSI
        lc      r1,1
        sb      r1,1(r2)        ; 1 to SCLK
        lb      r1,0(r2)        ; LSB from MISO
        or      r0,r1

        add     r0,r0
        sb      r0,1(r2)        ; 0 to SCLK
        lb      r1,9(fp)        ; data to write
        cls     r1,z
        add     r1,r1
        sb      r1,9(fp)
        mov     r1,c
        sb      r1,0(r2)        ; MSB to MOSI
        lc      r1,1
        sb      r1,1(r2)        ; 1 to SCLK
        lb      r1,0(r2)        ; LSB from MISO
        or      r0,r1

        add     r0,r0
        sb      r0,1(r2)        ; 0 to SCLK
        lb      r1,9(fp)        ; data to write
        cls     r1,z
        add     r1,r1
        sb      r1,9(fp)
        mov     r1,c
        sb      r1,0(r2)        ; MSB to MOSI
        lc      r1,1
        sb      r1,1(r2)        ; 1 to SCLK
        lb      r1,0(r2)        ; LSB from MISO
        or      r0,r1

        add     r0,r0
        sb      r0,1(r2)        ; 0 to SCLK
        lb      r1,9(fp)        ; data to write
        cls     r1,z
        add     r1,r1
        sb      r1,9(fp)
        mov     r1,c
        sb      r1,0(r2)        ; MSB to MOSI
        lc      r1,1
        sb      r1,1(r2)        ; 1 to SCLK
        lb      r1,0(r2)        ; LSB from MISO
        or      r0,r1

        add     r0,r0
        sb      r0,1(r2)        ; 0 to SCLK
        lb      r1,9(fp)        ; data to write
        cls     r1,z
        mov     r1,c
        sb      r1,0(r2)        ; MSB to MOSI
        lc      r1,1
        sb      r1,1(r2)        ; 1 to SCLK
        lb      r1,0(r2)        ; LSB from MISO
        or      r0,r1

        pop     r1
        pop     r2
        pop     fp
        jmp      (r1)
