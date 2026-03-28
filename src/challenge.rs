//! Challenge system and self-test for COR24 emulator

use crate::assembler::Assembler;
use crate::cpu::CpuState;
use crate::cpu::executor::Executor;

/// A challenge for the user to complete
#[derive(Clone)]
pub struct Challenge {
    pub id: usize,
    pub name: String,
    pub description: String,
    pub initial_code: String,
    pub hint: String,
    pub validator: fn(&CpuState) -> bool,
}

/// Get all available challenges
pub fn get_challenges() -> Vec<Challenge> {
    vec![
        Challenge {
            id: 1,
            name: "Load and Add".to_string(),
            description: "Load the value 10 into r0, then add 5 to it. Result should be 15 in r0."
                .to_string(),
            initial_code: "; Load 10 into r0, add 5\n; Result: r0 = 15\n\n".to_string(),
            hint: "Use 'lc r0,10' to load 10, then 'add r0,5' to add 5".to_string(),
            validator: |cpu| cpu.get_reg(0) == 15,
        },
        Challenge {
            id: 2,
            name: "Compare and Branch".to_string(),
            description: "Set r0 to 1 if 5 < 10 (signed), otherwise 0. Use cls and brt/brf."
                .to_string(),
            initial_code: "; Compare 5 < 10 and set r0 accordingly\n; Result: r0 = 1\n\n"
                .to_string(),
            hint: "Load values, use cls to compare, then mov r0,c to get the result".to_string(),
            validator: |cpu| cpu.get_reg(0) == 1,
        },
        Challenge {
            id: 3,
            name: "Stack Operations".to_string(),
            description: "Push values 1, 2, 3 onto the stack, then pop them into r0, r1, r2."
                .to_string(),
            initial_code: "; Push 1, 2, 3 then pop into r0, r1, r2\n; Result: r0=3, r1=2, r2=1\n\n"
                .to_string(),
            hint: "Remember LIFO order - last pushed is first popped".to_string(),
            validator: |cpu| cpu.get_reg(0) == 3 && cpu.get_reg(1) == 2 && cpu.get_reg(2) == 1,
        },
        Challenge {
            id: 4,
            name: "Max of Two".to_string(),
            description: "Set r0 to the maximum of r0=7 and r1=12 (without branching). Use mov ra,c!"
                .to_string(),
            initial_code: "; Find max of r0=7 and r1=12, store result in r0\n; Hint: Use COR24's mov ra,c feature\n; Result: r0 = 12\n\n        lc      r0,7\n        lc      r1,12\n\n        ; Your code here\n\nhalt:   bra     halt\n"
                .to_string(),
            hint: "cls sets C if r0 < r1. If true, you want r1. Use sub/add with C flag.".to_string(),
            validator: |cpu| cpu.get_reg(0) == 12,
        },
        Challenge {
            id: 5,
            name: "Byte Sign Extension".to_string(),
            description: "Load -50 (0xCE) as unsigned into r0, then sign-extend it. Result should be 0xFFFFCE."
                .to_string(),
            initial_code: "; Load 0xCE unsigned, then sign extend\n; Result: r0 = 0xFFFFCE (-50)\n\n"
                .to_string(),
            hint: "Use lcu to load unsigned, then sxt to sign extend".to_string(),
            validator: |cpu| cpu.get_reg(0) == 0xFFFFCE,
        },
    ]
}

/// Get example programs
pub fn get_examples() -> Vec<(String, String, String)> {
    vec![
        (
            "Add".into(),
            "Compute 100 + 200 + 42 = 342, store to memory".into(),
            include_str!("examples/assembler/add.s").into(),
        ),
        (
            "Assert".into(),
            "Validate results with assertions (has a deliberate bug!)".into(),
            include_str!("examples/assembler/assert.s").into(),
        ),
        (
            "Blink LED".into(),
            "Toggle LED with delay loop".into(),
            include_str!("examples/assembler/blink_led.s").into(),
        ),
        (
            "Button Echo".into(),
            "LED D2 follows button S2".into(),
            include_str!("examples/assembler/button_echo.s").into(),
        ),
        (
            "Comments".into(),
            "Comment syntax and how to edit examples".into(),
            include_str!("examples/assembler/comments.s").into(),
        ),
        (
            "Countdown".into(),
            "Count 10→0, store to memory".into(),
            include_str!("examples/assembler/countdown.s").into(),
        ),
        (
            "Echo".into(),
            "Interrupt-driven UART echo (letters→uppercase, !→halt)".into(),
            include_str!("examples/assembler/echo.s").into(),
        ),
        (
            "Fibonacci".into(),
            "Print fib(1)..fib(10) to UART".into(),
            include_str!("examples/assembler/fibonacci.s").into(),
        ),
        (
            "Literals".into(),
            "Decimal, negative, and Intel hex (NNh) number formats".into(),
            include_str!("examples/assembler/literals.s").into(),
        ),
        (
            "Loop Trace".into(),
            "Run, Stop, then view Instruction Trace".into(),
            include_str!("examples/assembler/loop_trace.s").into(),
        ),
        (
            "Memory Access".into(),
            "Store to non-adjacent memory blocks".into(),
            include_str!("examples/assembler/memory_access.s").into(),
        ),
        (
            "Multiply".into(),
            "6 × 7 = 42 two ways: native mul and loop".into(),
            include_str!("examples/assembler/multiply.s").into(),
        ),
        (
            "Nested Calls".into(),
            "Function call chain showing stack frames".into(),
            include_str!("examples/assembler/nested_calls.s").into(),
        ),
        (
            "Stack Variables".into(),
            "Local variables and register spilling, result to memory".into(),
            include_str!("examples/assembler/stack_variables.s").into(),
        ),
        (
            "UART Hello".into(),
            "Write \"Hello\\n\" to UART output".into(),
            include_str!("examples/assembler/uart_hello.s").into(),
        ),
        (
            "Variables".into(),
            "Copy \"Hello\" to SRAM, stack, and code — hover memory for ASCII".into(),
            include_str!("examples/assembler/variables.s").into(),
        ),
    ]
}

/// Self-test result for one example
#[derive(Clone, Debug)]
pub struct SelfTestResult {
    pub name: String,
    pub pass: bool,
    pub detail: String,
}

/// Run self-tests on all assembler examples.
/// If inject_bad is true, all expected values are wrong (test the tests).
pub fn run_self_tests(inject_bad: bool) -> Vec<SelfTestResult> {
    let examples = get_examples();
    let executor = Executor::new();
    let mut results = Vec::new();

    for (name, _, source) in &examples {
        let result = run_one_test(name, source, &executor, inject_bad);
        results.push(result);
    }
    results
}

fn run_one_test(name: &str, source: &str, executor: &Executor, inject_bad: bool) -> SelfTestResult {
    // Assemble
    let mut asm = Assembler::new();
    let asm_result = asm.assemble(source);
    if !asm_result.errors.is_empty() {
        return SelfTestResult {
            name: name.to_string(),
            pass: false,
            detail: format!("Assembly error: {}", asm_result.errors.join(", ")),
        };
    }

    // Load into CPU
    let mut cpu = CpuState::new();
    for (addr, byte) in asm_result.bytes.iter().enumerate() {
        cpu.memory[addr] = *byte;
    }
    cpu.pc = 0;

    // Run (non-halting examples get fewer cycles)
    let max_cycles = match name {
        "Blink LED" | "Button Echo" | "Loop Trace" => 200,
        "Echo" => 500,
        _ => 500_000,
    };
    executor.run(&mut cpu, max_cycles);

    // For interactive examples, inject input and run more
    match name {
        "Button Echo" => {
            // Press S2 (set switches bit 0 low)
            cpu.io.switches = 0x00;
            executor.run(&mut cpu, 100);
        }
        "Echo" => {
            // Send 'a' via UART RX
            cpu.uart_send_rx(b'a');
            executor.run(&mut cpu, 500);
        }
        _ => {}
    }

    // Check expected state
    check_expected(name, &cpu, inject_bad)
}

fn check_expected(name: &str, cpu: &CpuState, inject_bad: bool) -> SelfTestResult {
    let mut checks: Vec<(&str, String, String, bool)> = Vec::new(); // (label, expected, actual, pass)

    // When inject_bad, flip all boolean expectations and use wrong numeric values
    let expect_halt = !inject_bad;
    let expect_run = inject_bad; // inverted: expect halted when should be running

    match name {
        "Add" => {
            let val = cpu.read_byte(256);
            let expect_val: u8 = if inject_bad { 0xFF } else { 0x56 };
            checks.push((
                "halted",
                format!("{}", expect_halt),
                format!("{}", cpu.halted),
                cpu.halted == expect_halt,
            ));
            checks.push((
                "mem[256]",
                format!("0x{:02X}", expect_val),
                format!("0x{:02X}", val),
                val == expect_val,
            ));
        }
        "Assert" => {
            checks.push((
                "halted",
                format!("{}", expect_halt),
                format!("{}", cpu.halted),
                cpu.halted == expect_halt,
            ));
        }
        "Blink LED" => {
            let expect_running = !expect_run;
            checks.push((
                "running",
                format!("{}", expect_running),
                format!("{}", !cpu.halted),
                cpu.halted != expect_running,
            ));
            let min = if inject_bad { 99999 } else { 10 };
            checks.push((
                "instructions",
                format!(">{}", min),
                format!("{}", cpu.instructions),
                cpu.instructions > min as u64,
            ));
        }
        "Button Echo" => {
            let led = cpu.io.leds & 1;
            let expect_running = !expect_run;
            // Active-low: S2 pressed=0 → write 0 → LED ON (bit 0 = 0)
            let expect_led: u8 = if inject_bad { 1 } else { 0 };
            checks.push((
                "running",
                format!("{}", expect_running),
                format!("{}", !cpu.halted),
                cpu.halted != expect_running,
            ));
            checks.push((
                "LED (S2 pressed)",
                format!("{}", expect_led),
                format!("{}", led),
                led == expect_led,
            ));
        }
        "Comments" => {
            let r0 = cpu.get_reg(0);
            let expect_r0: u32 = if inject_bad { 999 } else { 300 };
            checks.push((
                "halted",
                format!("{}", expect_halt),
                format!("{}", cpu.halted),
                cpu.halted == expect_halt,
            ));
            checks.push((
                "r0",
                format!("{}", expect_r0),
                format!("{}", r0),
                r0 == expect_r0,
            ));
        }
        "Countdown" => {
            let val = cpu.read_byte(256);
            let expect_val: u8 = if inject_bad { 99 } else { 0 };
            checks.push((
                "halted",
                format!("{}", expect_halt),
                format!("{}", cpu.halted),
                cpu.halted == expect_halt,
            ));
            checks.push((
                "mem[256]",
                format!("{}", expect_val),
                format!("{}", val),
                val == expect_val,
            ));
        }
        "Echo" => {
            let expect_char = if inject_bad { 'Z' } else { 'A' };
            let has_char = cpu.io.uart_output.contains(expect_char);
            let expect_running = !expect_run;
            let label = if inject_bad {
                "UART has 'Z'"
            } else {
                "UART has 'A'"
            };
            checks.push((
                "running",
                format!("{}", expect_running),
                format!("{}", !cpu.halted),
                cpu.halted != expect_running,
            ));
            checks.push((
                label,
                "true".into(),
                format!("{} ({:?})", has_char, &cpu.io.uart_output),
                has_char,
            ));
        }
        "Fibonacci" => {
            let expected = if inject_bad {
                "WRONG\n"
            } else {
                "1 1 2 3 5 8 13 21 34 55\n"
            };
            checks.push((
                "halted",
                format!("{}", expect_halt),
                format!("{}", cpu.halted),
                cpu.halted == expect_halt,
            ));
            checks.push((
                "UART",
                format!("{:?}", expected),
                format!("{:?}", &cpu.io.uart_output),
                cpu.io.uart_output == expected,
            ));
        }
        "Literals" => {
            checks.push((
                "halted",
                format!("{}", expect_halt),
                format!("{}", cpu.halted),
                cpu.halted == expect_halt,
            ));
        }
        "Loop Trace" => {
            let expect_running = !expect_run;
            let min = if inject_bad { 99999 } else { 10 };
            checks.push((
                "running",
                format!("{}", expect_running),
                format!("{}", !cpu.halted),
                cpu.halted != expect_running,
            ));
            checks.push((
                "instructions",
                format!(">{}", min),
                format!("{}", cpu.instructions),
                cpu.instructions > min as u64,
            ));
        }
        "Memory Access" => {
            let v1 = cpu.read_byte(256);
            let v2 = cpu.read_byte(512);
            let (e1, e2): (u8, u8) = if inject_bad { (99, 99) } else { (42, 200) };
            checks.push((
                "halted",
                format!("{}", expect_halt),
                format!("{}", cpu.halted),
                cpu.halted == expect_halt,
            ));
            checks.push(("mem[256]", format!("{}", e1), format!("{}", v1), v1 == e1));
            checks.push(("mem[512]", format!("{}", e2), format!("{}", v2), v2 == e2));
        }
        "Multiply" => {
            let expected = if inject_bad { "WRONG\n" } else { "42 42\n" };
            checks.push((
                "halted",
                format!("{}", expect_halt),
                format!("{}", cpu.halted),
                cpu.halted == expect_halt,
            ));
            checks.push((
                "UART",
                format!("{:?}", expected),
                format!("{:?}", &cpu.io.uart_output),
                cpu.io.uart_output == expected,
            ));
        }
        "Nested Calls" => {
            let r0 = cpu.get_reg(0);
            let expect_r0: u32 = if inject_bad { 999 } else { 33 };
            checks.push((
                "halted",
                format!("{}", expect_halt),
                format!("{}", cpu.halted),
                cpu.halted == expect_halt,
            ));
            checks.push((
                "r0",
                format!("{}", expect_r0),
                format!("{}", r0),
                r0 == expect_r0,
            ));
        }
        "Stack Variables" => {
            let val = cpu.read_byte(256);
            let expect_val: u8 = if inject_bad { 99 } else { 16 };
            checks.push((
                "halted",
                format!("{}", expect_halt),
                format!("{}", cpu.halted),
                cpu.halted == expect_halt,
            ));
            checks.push((
                "mem[256]",
                format!("{}", expect_val),
                format!("{}", val),
                val == expect_val,
            ));
        }
        "UART Hello" => {
            let expected = if inject_bad { "WRONG\n" } else { "Hello\n" };
            checks.push((
                "halted",
                format!("{}", expect_halt),
                format!("{}", cpu.halted),
                cpu.halted == expect_halt,
            ));
            checks.push((
                "UART",
                format!("{:?}", expected),
                format!("{:?}", &cpu.io.uart_output),
                cpu.io.uart_output == expected,
            ));
        }
        "Variables" => {
            // "Hello World!" copied to SRAM at addresses 256 and 512
            let expected = if inject_bad { "WRONG" } else { "Hello World!" };
            let copy1: Vec<u8> = (0..12).map(|i| cpu.read_byte(256 + i)).collect();
            let copy2: Vec<u8> = (0..12).map(|i| cpu.read_byte(512 + i)).collect();
            let actual1 = String::from_utf8_lossy(&copy1).to_string();
            let actual2 = String::from_utf8_lossy(&copy2).to_string();
            checks.push((
                "halted",
                format!("{}", expect_halt),
                format!("{}", cpu.halted),
                cpu.halted == expect_halt,
            ));
            checks.push((
                "mem[256]",
                expected.to_string(),
                actual1.clone(),
                actual1 == expected,
            ));
            checks.push((
                "mem[512]",
                expected.to_string(),
                actual2.clone(),
                actual2 == expected,
            ));
        }
        _ => {
            checks.push(("defined", "true".into(), "false".into(), false));
        }
    }

    let pass = checks.iter().all(|c| c.3);
    let detail = checks
        .iter()
        .map(|(label, expected, actual, ok)| {
            if *ok {
                format!("{}: {}", label, actual)
            } else {
                format!("{}: expected {}, actual {}", label, expected, actual)
            }
        })
        .collect::<Vec<_>>()
        .join("; ");

    SelfTestResult {
        name: name.to_string(),
        pass,
        detail,
    }
}
