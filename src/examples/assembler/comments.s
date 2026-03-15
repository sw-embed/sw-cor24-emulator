; -- Comments Example --
;
; Edit this or any Assembler example
; to write COR24 code in your browser,
; assemble it, then run or step through.
;
; C and Rust examples are read-only
; (compiled offline, shown as demos).
;
; -- Comment syntax --
;
; Semicolons start a comment:

    lcu r0, 100     ; load 100 into r0

; Hash marks work too:

    lcu r1, 200     # load 200 into r1

# Full-line comments use ; or #

; -- Try it: edit, assemble, step! --

    add r0, r1      ; r0 = 300

; Stop by branching to self:
done:   bra     done
