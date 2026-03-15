//! Challenge system for COR24 emulator

use crate::cpu::CpuState;

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
            "Memory Access".into(),
            "Store to non-adjacent memory blocks".into(),
            include_str!("examples/assembler/memory_access.s").into(),
        ),
        (
            "Multiply".into(),
            "6 × 7 = 42 via loop, print to UART".into(),
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
    ]
}
