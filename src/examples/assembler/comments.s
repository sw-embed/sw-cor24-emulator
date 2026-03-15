; ============================================================
; COR24 Assembly — Comments Example
; ============================================================
;
; You can edit this (or any Assembler tab example) to write
; and assemble COR24 code right in your browser, then run
; or single-step through it.
;
; The C and Rust tab examples are read-only — they were
; compiled offline and are shown here as fixed pipeline demos.
;
; ------------------------------------------------------------
; Comment syntax
; ------------------------------------------------------------
;
; Semicolons start a comment (to end of line):

    lc r0, 1        ; load the constant 1 into r0

; Hash marks also start a comment:

    lc r1, 2        # load the constant 2 into r1

; A full line starting with ; or # is a comment:
# This entire line is a comment too.

; ------------------------------------------------------------
; Try it: edit, assemble, step!
; ------------------------------------------------------------

    add r0, r1      ; r0 = 1 + 2 = 3

; COR24 has no halt instruction — stop by branching to self:
done:   bra     done
