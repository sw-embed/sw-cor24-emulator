# Feature: `--terminal` mode for cor24-run

**Status: Implemented** (see `rust-to-cor24/src/run.rs`)

## Summary

The `--terminal` flag for `cor24-run` bridges the host terminal's stdin/stdout directly to the emulated UART, enabling interactive programs (REPLs, shells, monitors) to run on the COR24 emulator as if connected to a real serial terminal.

## Motivation

The tml24c project implements a Lisp REPL that reads from UART RX and writes to UART TX. Today there is no way to interact with it in real time:

- `--uart-input` pre-loads a static byte buffer. No way to respond to program output.
- The current workaround replays an entire session file from scratch on every new expression, which is slow and doesn't scale.
- The `[UART TX @ N] 'c'` output format is meant for debugging, not for end-user interaction.

Any COR24 program that does interactive I/O (REPL, monitor, game, echo server) needs this.

## Specification

### CLI interface

```
cor24-asm <file.s> -o /tmp/p.lgo && cor24-emu --lgo /tmp/p.lgo --terminal [options]
```

Existing flags that should compose with `--terminal`:
- `--speed, -s` — instruction rate (default 100,000; 0 = max speed)
- `--time, -t` — time limit (default: unlimited in terminal mode, or keep 10s and let user override)
- `--max-instructions, -n` — instruction limit (default: unlimited in terminal mode)
- `--entry, -e` — entry point label
- `--dump` — dump state on exit
- `--trace N` — dump last N instructions on exit

Flags that are **incompatible** with `--terminal` (error if combined):
- `--uart-input` / `-u` — conflicts with stdin-driven input
- `--step` — conflicts with raw terminal mode

### Behavior

#### Startup
1. Assemble and load program as normal
2. Put the host terminal into **raw mode** (disable line buffering, disable echo, disable signal processing for Ctrl-C if possible — Ctrl-C should send 0x03 to the UART, not kill the emulator)
3. Print a one-line banner: `[cor24-run terminal mode — Ctrl-] to exit]`
4. Begin execution

#### Main loop

The main loop runs the emulator in batches (same as `run_with_timing`), but between batches:

1. **UART TX → stdout**: Check for new bytes in `emu.get_uart_output()`. Write them directly to stdout as raw bytes. No `[UART TX]` decoration. Flush after each batch that produces output.

2. **stdin → UART RX**: Non-blocking read from stdin. If bytes are available, feed them one at a time via `emu.send_uart_byte()`, respecting the FIFO-drain protocol (only feed when `read_byte(0xFF0101) & 0x01 == 0`). Buffer any excess stdin bytes until the emulator is ready.

3. **Exit signal**: The escape sequence **Ctrl-]** (0x1D, same as telnet escape) signals the user wants to quit. On receiving 0x1D from stdin, break out of the loop. Do NOT forward 0x1D to the UART.

4. **CPU halt**: If `run_batch` returns 0 instructions (CPU halted via self-branch), print `\n[CPU halted]` and exit.

#### Shutdown
1. Restore the terminal to its original mode (cooked mode, echo on)
2. Print `\n[cor24-run exited]`
3. If `--dump` was specified, print the state dump
4. If `--trace N` was specified, print the trace

### Raw terminal mode details

Use the `termios` crate or raw `libc::tcsetattr` to:
- Disable canonical mode (`ICANON` off) — character-at-a-time input
- Disable echo (`ECHO` off) — let the COR24 program decide what to echo
- Disable signal generation (`ISIG` off) — Ctrl-C sends 0x03 byte, not SIGINT
- Set `VMIN=0, VTIME=0` — non-blocking reads
- Restore original termios on exit (use a drop guard or `atexit`)

### Stdin buffering

Since the emulator can only accept one UART RX byte at a time (single-register, no FIFO), and the user may type faster than the emulator consumes:

- Maintain a `VecDeque<u8>` input buffer
- On each batch tick: read all available stdin bytes into the buffer
- After each batch: if UART RX ready bit is clear AND buffer is non-empty, feed the next byte

This is the same FIFO-drain pattern already used for `--uart-input`, but with stdin as the source instead of a static buffer.

### Speed considerations

- Default speed in terminal mode should probably be higher than 100K IPS (maybe 500K or 1M) since the user expects responsive interaction. Or default to `--speed 0` (max speed) in terminal mode.
- Batch size should be tuned so UART output appears promptly. With speed=0, batch_size=10000 is fine. With speed=500K, batch_size=5000 gives 100 batches/sec which is responsive enough.
- Time limit should default to unlimited (or very large, like 3600s) in terminal mode, since the session is interactive.

### Example usage

```bash
# Interactive Lisp REPL
cor24-asm build/repl.s -o build/repl.lgo && cor24-emu --lgo build/repl.lgo --terminal

# With explicit speed
cor24-asm build/repl.s -o build/repl.lgo && cor24-emu --lgo build/repl.lgo --terminal --speed 500000

# Dump state after exit
cor24-asm build/repl.s -o build/repl.lgo && cor24-emu --lgo build/repl.lgo --terminal --dump

# With entry point
cor24-asm build/repl.s -o build/repl.lgo && cor24-emu --lgo build/repl.lgo --terminal --entry main
```

What the user sees:
```
[cor24-run terminal mode — Ctrl-] to exit]
> (+ 1 2)
3
> (define square (lambda (n) (* n n)))
#<obj>
> (square 7)
49
> ^]
[cor24-run exited]
```

### Implementation sketch

```rust
"terminal" => {
    // ... assemble, load (same as "run" mode) ...

    // Raw terminal setup
    let original_termios = set_raw_mode()?;
    let _guard = TermiosGuard(original_termios); // restores on drop

    println!("[cor24-run terminal mode — Ctrl-] to exit]");

    let mut stdin_buf: VecDeque<u8> = VecDeque::new();
    let mut prev_uart_len = 0usize;
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    let mut read_buf = [0u8; 64];

    emu.resume();

    loop {
        // Time/instruction limit checks ...

        let result = emu.run_batch(batch_size);

        // TX: flush new UART output to stdout
        let output = emu.get_uart_output();
        if output.len() > prev_uart_len {
            stdout.write_all(output[prev_uart_len..].as_bytes())?;
            stdout.flush()?;
            prev_uart_len = output.len();
        }

        // RX: non-blocking read from stdin into buffer
        // (with raw mode + VMIN=0 VTIME=0, read returns immediately)
        if let Ok(n) = stdin.lock().read(&mut read_buf) {
            for &b in &read_buf[..n] {
                if b == 0x1D { /* Ctrl-] */
                    goto exit;
                }
                stdin_buf.push_back(b);
            }
        }

        // Feed buffered input to UART when ready
        if !stdin_buf.is_empty() {
            let status = emu.read_byte(0xFF0101);
            if status & 0x01 == 0 {
                let ch = stdin_buf.pop_front().unwrap();
                emu.send_uart_byte(ch);
            }
        }

        if result.instructions_run == 0 {
            println!("\n[CPU halted]");
            break;
        }

        // Timing synchronization (same as run_with_timing)
    }
}
```

### Edge cases to handle

1. **Program doesn't read UART**: If the program never polls UART RX, stdin bytes accumulate in the buffer. This is fine — they just sit there. Consider a max buffer size (e.g., 4KB) and silently drop overflow.

2. **Binary output**: UART TX bytes go directly to stdout. If the program outputs binary/control chars, they'll go to the terminal. This is intentional — same as connecting a real serial terminal.

3. **Ctrl-C handling**: With `ISIG` off, Ctrl-C (0x03) goes to the UART as a byte, not as a signal. The user uses Ctrl-] to exit instead. Document this clearly in the banner.

4. **Pipe detection**: If stdin is not a TTY (e.g., piped input), skip raw mode setup and just read stdin as-is. When stdin reaches EOF, let the emulator continue running (it may still produce output) until the time/instruction limit. This makes `echo "(+ 1 2)" | cor24-asm repl.s -o /tmp/repl.lgo && cor24-emu --lgo /tmp/repl.lgo --terminal` work too.

5. **Terminal restoration on panic**: The `TermiosGuard` drop impl must restore the terminal. Also register a panic hook that restores termios, or the user's terminal will be stuck in raw mode.

### Testing

1. **Smoke test**: assembling echo.s with `cor24-asm` then `cor24-emu --lgo echo.lgo --terminal` where echo.s reads a byte and writes it back. Type characters, verify they echo.

2. **REPL test**: `cor24-asm repl.s -o /tmp/repl.lgo && cor24-emu --lgo /tmp/repl.lgo --terminal` — define a function, call it, verify output.

3. **Ctrl-] exit**: Verify clean exit and terminal restoration.

4. **Pipe mode**: `echo "(+ 1 2)" | cor24-asm repl.s -o /tmp/repl.lgo && cor24-emu --lgo /tmp/repl.lgo --terminal` — verify output, clean exit on EOF.

5. **CPU halt**: Run a program that halts. Verify `[CPU halted]` message and clean exit.

6. **Speed/timing**: Verify responsive interaction at default speed.
