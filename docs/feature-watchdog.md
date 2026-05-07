# Feature Request: Watchdog and Exit Code Differentiation

## Problem

When a COR24 program halts, `cor24-run` prints `[CPU halted]` and exits with code 0 regardless of whether the halt was:
- Intentional: `(exit)` → self-branch by design
- PANIC: OOM/STR-OOM → self-branch after error message
- Bug: infinite loop that happens to self-branch
- Timeout: instruction limit or time limit reached

The user can't distinguish these programmatically (e.g., in CI scripts).

## Proposed: Exit Code Differentiation

| Condition | Exit Code | Message |
|-----------|-----------|---------|
| Clean halt (self-branch) | 0 | `[CPU halted]` |
| Instruction limit reached | 1 | `[instruction limit reached]` |
| Time limit reached | 2 | `[time limit reached]` |
| UART output contains "PANIC:" | 3 | `[PANIC detected in UART output]` |

### PANIC Detection

Scan the accumulated UART output for the string `PANIC:`. If found, exit with code 3 instead of 0 on halt. This requires no protocol changes — tml24c already prints `PANIC:` prefixed messages before halting.

```bash
cor24-asm prog.s -o /tmp/p.lgo && cor24-emu --lgo /tmp/p.lgo --terminal --speed 0 -n 500000000
echo $?  # 0 = clean, 1 = instruction limit, 2 = time, 3 = PANIC
```

## Proposed: Watchdog Mode

```
cor24-asm prog.s -o /tmp/p.lgo && cor24-emu --lgo /tmp/p.lgo --watchdog 5
```

If the CPU produces no new UART output for N seconds, assume it's stuck in an infinite loop (not a halt — halts are detected immediately). Print `[watchdog timeout: no output for 5s]` and exit with code 4.

This catches:
- Spin loops that aren't self-branches (e.g., polling a register that never changes)
- Prelude loading that takes too long
- Programs that hang waiting for UART input when no input is available

### Implementation

In the batch loop, track the last time UART output changed. If `Instant::now() - last_uart_change > watchdog_duration`, break and exit.

```rust
let mut last_output_time = Instant::now();
// ... in loop:
if output.len() > prev_uart_len {
    last_output_time = Instant::now();
}
if watchdog > 0 && last_output_time.elapsed() > Duration::from_secs(watchdog) {
    println!("[watchdog timeout]");
    break;
}
```
