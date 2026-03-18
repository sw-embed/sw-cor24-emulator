# Changes

## 2026-03-16

- Add collapsible Instruction Trace panel to Web UI (last 100 entries)
- Record halt instruction (bra-to-self) in trace before halting
- Add `trace` command to cor24-dbg CLI debugger
- Convert all 14 assembler examples to reference as24-compatible syntax
- Translator emits decimal immediates instead of hex (as24 compat)
- Regenerate all 13 pipeline demos with decimal immediates
- Add Assert example with deliberate bug for debugging demo
- Add Loop Trace example demonstrating Run/Stop/Trace workflow
- Update Multiply example: native mul + loop, with assertions
- Add assembler range check for lc (0..127) and lcu (0..255)
- Document assembler compatibility analysis (docs/extended-assembler.md)
- Add Changes link in web UI footer (links to GitHub CHANGES.md)
- Update README screenshots: assembler, C, and Rust tabs
- Backfill CHANGES.md with full project history (Feb 25 - Mar 16)
- Comprehensive tutorial: docs/cor24-tutorial.md and web UI Tutorial dialog
  (registers, addressing modes, all instruction groups, I/O, interrupts, calling convention)
- ISA Reference restyled with tables, CPU State, and Interrupts sections
- Move Examples/Challenges to context-specific locations above editor/wizard
- Program Editor fills available height
- Fix wizard scroll with scrollIntoView for reliable header visibility

## 2026-03-17

- Fix nop: encode as 0xFF (matching reference as24), not add r0,r0
  (old encoding had unwanted side effect of doubling r0)
- Executor treats 0xFF as true no-op (advance PC, no register changes)
- Add Intel hex literal support (NNh suffix) to match reference as24
- Add Literals example showing decimal, negative, and Intel hex formats
- Simplify UART TX busy poll: cls r2,z / brt (3 instructions → 1)
  per Luther Johnson's feedback — lb sign-extends bit 7, so negative = busy
- Apply cls/brt TX busy optimization to C pipeline examples (fib, sieve)
- Rewrite Rust fibonacci_iter demo using @cor24 passthrough — 16 instructions
  with zero spills (was ~35 with 14 spill ops per loop iteration)
- Add translator optimization design doc (docs/translator-optimization.md)
- Per-instruction run loop with speed slider (1ms-200ms per instruction)
  replaces 500-batch loop that froze the browser on slower machines
- Extract shared run_one_instruction(), eliminating ~300 lines of
  duplicated run loop code across the three tabs
- Extract shared callback factories (make_step/run/switch/uart_send/
  uart_clear_callback), eliminating another ~225 lines of duplication
- Cell headers (Source, Assembly, Execution) styled yellow for contrast
- Document register spill behavior in Stack Variables demo

## 2026-03-18

- Per-instruction run loop with log-scale speed slider (10-1000/s)
- Refactored callbacks: shared factories eliminate ~540 lines duplication
- Remove delay loops from Countdown and Blink LED assembler examples
- Reduce Rust pipeline delay loops (5000→10 iterations)
- Show example name in Program Editor heading
- LED label " ON"/"OFF" fixed-width to prevent layout shift
- ID-based listing auto-scroll: instant jump to current PC during Run
- Yellow cell headers for contrast
- Log-scale speed slider: 10/s left, 100/s center, 1000/s right

## 2026-03-15

- Add Comments example showing comment syntax and editability
- Fix Rust pipeline Compile step not scrolling to MSP430 assembly
- UART hex dump wraps at 8 bytes/line instead of one long line
- Remove max-height cap on code blocks in C/Rust pipeline tabs
- Fix stale load generation counter causing empty source panel
- Add load_generation prop to ProgramArea for re-select same example
- Fix Step dropdown misalignment in notebook debug cells
- Reorder tabs alphabetically: Assembler, C, Rust

## 2026-03-14

- Add C tab to web UI with CPipeline component and wizard
- Add Compile step to C pipeline wizard (Source → Compile → Assemble)
- Add Tutorial button to C pipeline sidebar
- Add C pipeline examples: fib, sieve (with printf runtime stubs)
- Add .comm directive and .byte fix to assembler
- Migrate assembler examples to jal calling convention
- Regenerate pipeline demos with jal calling convention
- Add instruction trace ring buffer and CLI --trace/--step modes
- Document Luther Johnson's COR24 calling convention feedback

## 2026-03-13

- Add @cor24 asm passthrough in translator for ISR code
- Echo example: letters→uppercase, !→halt with interrupt-driven UART
- Fix MMIO byte width bug (must use byte ops, not word ops)
- Fix cmp+equality branch translation in translator
- Add --max-instructions and --uart-input options to cor24-run
- Add per-demo run scripts and master demo runner
- Add demo_drop: Rust Drop trait (RAII) on stack
- Fix translator sp base register bug
- Compact UI: tighter registers, consistent sidebars, larger Rust fonts
- Register tooltips show decimal values (signed+unsigned when bit 23 set)
- Fix tooltips clipped at right edge of screen
- Add MakerLisp sidebar link to both tabs
- Add panic demo, fix spill register clobbering, tail call optimization

## 2026-03-12

- Show complete, self-contained Rust source in Web UI pipeline examples
- Add entry point prologue to MSP430→COR24 translator
- Add pipeline tests and fail-fast on bad entry point

## 2026-03-11

- Stop running CPU when loading example or challenge

## 2026-03-10

- Sparse memory display with zero-row collapsing
- Add OOM and stack overflow example programs
- Implement UART RX interrupt support matching COR24 hardware
- Add UART input panel and interrupt-driven echo example
- Fix UART input race condition with shared queue during animated run
- Add UART clear button and echo prompt character

## 2026-03-09

- Rust→MSP430→COR24 pipeline: translator, demos, emulator dump
- Shared DebugPanel with unified register/memory display and I/O regions
- Shared ExamplePicker component for both Assembler and Rust tabs
- Auto-load Blink LED example on Rust tab startup
- LED duty cycle tracking for accurate blink visualization
- CSS tooltips via data-tooltip attribute
- UART TX display, Multiply and Fibonacci examples with UART output
- Integration tests for Fibonacci and other examples

## 2026-03-07

- Fix core CPU: memory model, UART addresses, SP init, I/O registers
- Fix branch displacement calculation
- Add LGO loader and CLI debugger (cor24-dbg)
- Add EmulatorCore abstraction layer
- Web UI: multi-region memory display and EmulatorCore integration
- Remove halt pseudo-instruction; use label+branch pattern
- Detect self-branch as halt condition
- UI overhaul: larger fonts, compact I/O, improved contrast
- Add Hello World UART example and demo scripts

## 2026-03-03

- Add feedback documents, test source, analysis comparisons
- FPGA soft CPU notes

## 2026-03-02

- Add animated run with stop button for infinite loops
- Use Rc<Cell> for stop flag and switch state (race condition fixes)
- Add Stop button and button/LED support for Rust pipeline

## 2026-03-01

- Add COR24-specific examples and challenges
- Add working wasm2cor24 translator prototype
- Add memory-mapped I/O with LED and switch visualization
- Add CLI LED demo runner (cor24-run)
- Complete Rust→WASM→COR24 compilation pipeline
- Add Assembler/Rust tabs with pipeline visualization
- Add step-through debugging with assembly listing highlighting
- Wizard-driven 3-column layout for Rust pipeline
- Heat map highlighting for register/memory changes
- Switch to pre-built pages deployment

## 2026-02-28

- Extract decode ROM from Verilog and update assembler encodings
- Add executor tests, fix assembler comments
- Setup GitHub Pages deployment

## 2026-02-25

- Initial COR24 assembly emulator implementation
