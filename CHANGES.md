# Changes

## 2026-03-28

- **BREAKING**: LED active-low polarity matches hardware (0=ON, 1=OFF)
  - `IoState.leds` defaults to 1 (OFF at reset, matching Verilog `led <= 1'b1`)
  - Display code interprets bit 0 = 0 as ON
  - `EmulatorCore::is_led_on()` helper added
  - All assembler and Rust demos updated
  - Button Echo uses direct copy (no XOR), matching hardware blinky.c
- `--switch on|off` flag to set button S2 state before execution
- Improved `--dump` I/O section: LED and switch state shown separately with labels
- `--base-addr <addr>` for `--assemble` mode: labels resolve at specified base address
- `assemble_at(source, base_address)` public API on Assembler
- Fixed hello_uart.s and hello_world.s to poll UART TX busy before writes

## 2026-03-27

- `--patch <addr>=<value>` flag: write 24-bit LE values to memory after loading (repeatable)
- Binary-only mode: `--load-binary` + `--entry <addr>` without `--run` skips assembly
- `.p24` auto-detection: `--load-binary` strips 18-byte header when P24 magic detected
- `--entry` accepts numeric addresses (0x prefix, h suffix, decimal) in addition to labels
- 11 new unit tests for address parsing, patching, .p24 detection, binary loading
- Fix clippy `abs_diff` warning in isa/src/branch.rs
- Update docs/cli-tools.md with new flags, examples, and guest binary loading section
- Update [[COR24RS]] wiki page with recent changes and API docs

## 2026-03-24

- `--load-binary <file>@<addr>` flag for loading guest binaries into emulator memory
- `.word` directive supports label references (forward and backward)
- UART TX busy simulation disabled in WASM mode (prevents browser freeze)
- `-h` (short help), `--help` (extended help), `-V`/`--version` flags for cor24-run
- `--terminal` mode bridging stdin/stdout to UART
- `--echo` flag for local echo in terminal mode
- `--stack-kilobytes <3|8>` for configurable EBR stack size

## 2026-03-21

- Realistic UART TX busy: 10 instruction cycles per character (was 1)
- Characters written while TX busy are dropped (not silently buffered)
- `--uart-never-ready` CLI flag: TX never clears, polling programs hang
- 3 new UART discipline tests (no-poll drops, poll works, never-ready hangs)
- Assembler rejects r3-r9 register names; only r0-r2, fp, sp, z, iv, ir
- Remove r3-r7 aliases from all docs, ISA Reference, Tutorial
- Fix `mov r6,r0` → `mov iv,r0` in echo/echo_v2 demos
- Fix UART status address mapping: MSP430 0xFF02 → COR24 0xFF0101
- TX busy poll added to all 4 Rust demos (blinky, echo, panic, uart_hello)
- Replace all "Luther Johnson" references with "MakerLisp"
- VHS tapes re-recorded with paginated display and MP4 output

## 2026-03-20

- Feedback checklist with meeting notes and assembly idiom items
- CLI tools and pipeline documentation (docs/cli-tools.md, docs/pipeline.md)

## 2026-03-19

- Apply MakerLisp's assembly idiom feedback: add r0,-1 replaces lc+sub
  in countdown, fibonacci, multiply (saves 1-3 instructions each)
- Echo example: add TX busy poll (cls/brt) before all UART writes
- Feedback checklist: docs/feedback-checklist.md tracks all items
- ?showme-asm, ?showme-c, ?showme-rust animated tour modes
- Fix stale run loop: generation counter kills old setTimeout chains
- Stop emulator on example load and tab switch

## 2026-03-18

- Self-test mode: ?selftest runs all 15 examples, ?selftestbad uses wrong
  expected values for every test to verify the test system itself
- Self-test panel: green/red per-test rows, "expected X, actual Y" on failure
- Stop running emulator on example load and tab switch (was continuing)
- Fix stale EXAMPLE_PROGRAM: replace hardcoded copy with include_str!
- Fix Rust Button Echo: add XOR inversion for S2 active-low switch
- LED duty cycle tooltip restored (accumulates via run loop parameter)
- LED label "_ON"/"OFF" fixed-width (non-collapsible underscore)
- Blink LED: balanced 50% duty cycle with editable nop padding
- ID-based listing auto-scroll: instant jump to current PC during Run
- Remove scroll-behavior:smooth that caused slow listing scroll lag
- Log-scale speed slider: 10/s left, 100/s center, 1000/s right
- Show example name in Program Editor heading
- Remove delay loops from Countdown and Blink LED assembler examples
- Reduce Rust pipeline delay loops (5000→10 iterations)
- Per-instruction run loop replaces 500-batch loop (no browser freeze)
- Extract shared run_one_instruction and callback factories (~540 lines removed)

## 2026-03-17

- Fix nop: encode as 0xFF (matching reference as24), not add r0,r0
- Executor treats 0xFF as true no-op (advance PC, no register changes)
- Add Intel hex literal support (NNh suffix) to match reference as24
- Add Literals example showing decimal, negative, and Intel hex formats
- Simplify UART TX busy poll: cls r2,z / brt (1 instruction vs 3)
- Apply cls/brt TX busy optimization to C pipeline examples (fib, sieve)
- Rewrite Rust fibonacci_iter demo using @cor24 passthrough (16 instructions,
  zero spills, was ~35 with 14 spill ops per loop iteration)
- Add translator optimization design doc (docs/translator-optimization.md)
- Document register spill behavior in Stack Variables demo
- Cell headers (Source, Assembly, Execution) styled yellow for contrast

## 2026-03-16

- Comprehensive tutorial: docs/cor24-tutorial.md and web UI Tutorial dialog
- ISA Reference restyled with tables, CPU State, and Interrupts sections
- Move Examples/Challenges to context-specific locations above editor/wizard
- Program Editor fills available height
- Fix wizard scroll with scrollIntoView for reliable header visibility
- Add Changes link in web UI footer (links to GitHub CHANGES.md)
- Update README screenshots: assembler, C, and Rust tabs
- Backfill CHANGES.md with full project history
- Add collapsible Instruction Trace panel to Web UI (last 100 entries)
- Record halt instruction (bra-to-self) in trace before halting
- Add `trace` command to cor24-dbg CLI debugger
- Convert all assembler examples to reference as24-compatible syntax
- Translator emits decimal immediates instead of hex (as24 compat)
- Add Assert example with deliberate bug for debugging demo
- Add Loop Trace example demonstrating Run/Stop/Trace workflow
- Update Multiply example: native mul + loop, with assertions
- Add assembler range check for lc (0..127) and lcu (0..255)
- Document assembler compatibility analysis (docs/extended-assembler.md)

## 2026-03-15

- Add Comments example showing comment syntax and editability
- Fix Rust pipeline Compile step not scrolling to MSP430 assembly
- UART hex dump wraps at 8 bytes/line instead of one long line
- Remove max-height cap on code blocks in C/Rust pipeline tabs
- Fix stale load generation counter causing empty source panel
- Fix Step dropdown misalignment in notebook debug cells
- Reorder tabs alphabetically: Assembler, C, Rust

## 2026-03-14

- Add C tab to web UI with CPipeline component and wizard
- Add Compile step to C pipeline wizard (Source → Compile → Assemble)
- Add Tutorial button to C pipeline sidebar
- Add C pipeline examples: fib, sieve (with printf runtime stubs)
- Add .comm directive and .byte fix to assembler
- Migrate assembler examples to jal calling convention
- Add instruction trace ring buffer and CLI --trace/--step modes
- Document MakerLisp's COR24 calling convention feedback

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
