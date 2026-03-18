; Countdown: Store 10 down to 0 to memory at 256
; Watch r0 count down and mem[256] update each iteration
; Use Step or Run with the speed slider to watch

        la      r1,256      ; Result address
        lc      r0,10       ; Start at 10

loop:
        sb      r0,0(r1)    ; Write count to memory

        lc      r2,1
        sub     r0,r2       ; count--
        ceq     r0,z        ; count == 0?
        brf     loop        ; Continue if not zero

        ; Clear result and halt
        lc      r0,0
        sb      r0,0(r1)
halt:
        bra     halt
