; Loop Trace: Watch a loop run via trace
;
; How to use:
;   1. Assemble this code
;   2. Click Run — the loop runs forever
;   3. Click Stop after a moment
;   4. Expand "Instruction Trace" in the
;      debug panel to see recent iterations
;
; The trace shows each instruction with
; its PC, disassembly, and any register
; or flag changes in [brackets].

        lc      r0, 0           ; counter = 0
        lc      r1, 1           ; increment

        ; Infinite loop: count up forever
loop:
        add     r0, r1          ; counter++
        bra     loop
