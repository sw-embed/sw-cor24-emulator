; Literals: Number formats in COR24
;
; Decimal, negative, and Intel hex (NNh)
; are supported by both assemblers.

        ; Decimal literals
        lc  r0, 42         ; r0 = 42
        lcu r1, 200         ; r1 = 200

        ; Negative decimal
        lc  r0, -1          ; r0 = 0xFFFFFF
        la  r1, -65536      ; r1 = 0xFF0000

        ; Intel hex (NNh suffix)
        lcu r0, 0C8h        ; r0 = 200 (0xC8)
        lc  r1, 2Ah         ; r1 = 42  (0x2A)
        la  r2, 0FF0100h    ; r2 = 0xFF0100

        ; Assert: all three r2 loads are the
        ; same value (-65280 = 0FF0100h)
        la  r0, -65280
        ceq r0, r2
        brf assert_fail

        ; Assert: 0C8h == 200
        lcu r0, 0C8h
        lcu r1, 200
        ceq r0, r1
        brf assert_fail

all_pass:
        bra all_pass

assert_fail:
        bra assert_fail
