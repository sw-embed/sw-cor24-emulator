; Multiply: 6 x 7 = 42, two ways
; Native mul vs repeated addition loop
; Asserts both methods produce 42

        ; --- Method 1: native mul instruction ---
        lc      r0,6
        lc      r1,7
        mul     r0,r1           ; r0 = 42

        ; Assert r0 == 42
        lc      r1,42
        ceq     r0,r1
        brf     assert_fail

        ; --- Method 2: repeated addition loop ---
        lc      r0,0            ; sum = 0
        lc      r1,7            ; counter = 7
loop:
        add     r0,6            ; sum += 6
        push    r0
        lc      r0,1
        sub     r1,r0           ; counter--
        pop     r0
        ceq     r1,z
        brf     loop

        ; Assert r0 == 42
        lc      r1,42
        ceq     r0,r1
        brf     assert_fail

        ; Both methods agree — print "42 42\n"
        push    r0              ; save result
        la      r2,print2
        jal     r1,(r2)

        lc      r0,32           ; ' '
        la      r2,putc
        jal     r1,(r2)

        pop     r0              ; restore result
        la      r2,print2
        jal     r1,(r2)

        lc      r0,10           ; '\n'
        la      r2,putc
        jal     r1,(r2)

halt:
        bra     halt

; Assertion failed — spins here so you can
; inspect registers to see what went wrong
assert_fail:
        bra     assert_fail

; print2: print r0 as 1-2 digit decimal
print2:
        push    r1
        lc      r1,0
.div10:
        lc      r2,10
        clu     r0,r2
        brt     .ones
        sub     r0,r2
        add     r1,1
        bra     .div10
.ones:
        push    r0
        ceq     r1,z
        brt     .notens
        push    r1
        lc      r0,48
        add     r0,r1
        la      r2,putc
        jal     r1,(r2)
        pop     r1
.notens:
        pop     r0
        lc      r1,48
        add     r0,r1
        la      r2,putc
        jal     r1,(r2)
        pop     r1
        jmp     (r1)

; putc: send byte in r0, polling TX busy
putc:
        push    r1
        push    r0
        la      r1,-65280
.wait:
        lb      r2,1(r1)
        lcu     r0,128
        and     r2,r0
        ceq     r2,z
        brf     .wait
        pop     r0
        sb      r0,0(r1)
        pop     r1
        jmp     (r1)
