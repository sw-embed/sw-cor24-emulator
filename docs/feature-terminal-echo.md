# Feature: `--echo` flag for `--terminal` mode

**Status: Implemented** (see `rust-to-cor24/src/run.rs`)

## Summary

The `--echo` flag for `cor24-run --terminal` echoes stdin bytes to stdout as they are sent to UART RX. This provides local echo at the terminal level, so COR24 programs don't need to implement their own character echo.

## Motivation

The tml24c REPL reads lines from UART but doesn't echo characters back — it just silently collects input and prints the result. In `--terminal` mode, the user types blind:

```
> 42        ← user sees only "42" (the result), not "(+ 40 2)" (what they typed)
```

Echo is a terminal-level concern, not an application concern. Real serial terminals (minicom, screen, picocom) all have local echo as a terminal setting. The COR24 program shouldn't need to know about it.

## Specification

```
cor24-asm <file.s> -o /tmp/p.lgo && cor24-emu --lgo /tmp/p.lgo --terminal --echo
```

When `--echo` is enabled:
- Each byte read from stdin is written to stdout **before** being sent to UART RX
- Special handling:
  - `\r` or `\n` → write `\n` to stdout (normalize line endings)
  - Backspace (0x08) or DEL (0x7F) → write `\b \b` sequence to erase previous character on screen
  - Ctrl characters (0x00-0x1F except \r, \n, \b) → don't echo (control signals)
  - Printable characters (0x20-0x7E) → echo as-is
- Echo happens at the point bytes enter the stdin buffer, not when they're drained to UART RX. This ensures immediate visual feedback even if the emulator is busy.

## Example

Without `--echo`:
```
> 42
>
```

With `--echo`:
```
> (+ 40 2)
42
> (exit)
Bye.
```

## CLI change

Add to `CliArgs`:
```rust
echo: bool,  // echo stdin to stdout in terminal mode
```

Parse `--echo` flag. Error if used without `--terminal`.

## Default behavior

`--echo` should be **off** by default. Some COR24 programs (echo servers, editors) do their own echo and doubling would be wrong. The user opts in when they know the program doesn't echo.

For convenience, the tml24c Makefile and scripts would use `--echo`:
```makefile
run: $(REPL_ASM)
	$(COR24_RUN) --run $(REPL_ASM) --terminal --echo --speed 0
```
