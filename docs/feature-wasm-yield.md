# Feature: WASM Batch Yield for Tight Loops

**Status: Option C implemented** — `EmulatorCore::set_uart_tx_busy_cycles(0)` added;
`WasmCpu` sets instant TX on new/reset/hard_reset. External consumers (e.g. web-tml24c)
should call `emulator.set_uart_tx_busy_cycles(0)` after creating `EmulatorCore`.

## Problem

When running tml24c in the web UI, programs that call `putc_uart` rapidly (e.g., bottles demo printing many characters) can hang the browser tab. The cause:

1. `putc_uart` busy-waits on TX busy flag (`while (UART_STATUS & 0x80) {}`)
2. The emulator sets TX busy for 10 cycles after each write
3. In WASM batch mode, if `uart_tick()` isn't called between instructions within a batch, the busy flag never clears
4. The busy loop spins the entire batch budget without progress
5. Browser tab locks up (single-threaded JS)

## Mitigations Applied

tml24c now uses a bounded busy wait (100 iterations max) in `putc_uart`, so it won't spin forever. But this means characters may be written while TX is still "busy", which the emulator drops.

## Proposed Fix in Emulator

### Option A: Always tick UART in execute loop

Ensure `uart_tick()` is called after every instruction, including within `run_batch()`. The WASM `step()` path may already do this, but the batch path might skip it.

### Option B: Yield from WASM batch on tight loop detection

If the emulator detects the same PC executing repeatedly (self-branch or small loop), yield back to the JS event loop after N iterations. This lets the browser paint, handle input, and avoid "page unresponsive" warnings.

```rust
// In WASM run loop:
if last_pc == current_pc {
    tight_loop_count += 1;
    if tight_loop_count > 1000 {
        return BatchResult { reason: StopReason::Yield, .. };
    }
} else {
    tight_loop_count = 0;
}
```

### Option C: Set uart_tx_busy_cycles to 0 in WASM mode

Skip TX busy simulation entirely in the web UI. Characters go through instantly. The busy flag is a hardware fidelity feature that's unnecessary for interactive web use.

```rust
#[cfg(target_arch = "wasm32")]
fn new() -> Self {
    let mut cpu = CpuState::new();
    cpu.io.uart_tx_busy_cycles = 0; // instant TX in web mode
    cpu
}
```

## Recommended

Option C is the simplest and most robust. TX busy simulation serves no purpose in a web REPL — it only exists to test hardware polling discipline.
