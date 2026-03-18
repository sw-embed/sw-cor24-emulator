//! Yew application for COR24 Assembly Emulator

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::Rc;

use components::{
    CExample, CPipeline, DebugPanel, ExampleItem, ExamplePicker, Header, Modal, ProgramArea,
    EmulatorState, RustExample, RustPipeline, Sidebar, SidebarButton, Tab, TabBar, Tooltip,
};
use yew::prelude::*;

use crate::challenge::{get_challenges, get_examples};
use crate::wasm::{WasmCpu, validate_challenge};

#[function_component(App)]
pub fn app() -> Html {
    // Tab state
    let active_tab = use_state(|| "assembler".to_string());

    // Rust pipeline state - separate CPU for Rust tab execution
    let rust_cpu = use_state(WasmCpu::new);
    let rust_emu_state = use_state(EmulatorState::default);
    let rust_is_loaded = use_state(|| false);
    let rust_is_running = use_state(|| false);
    let rust_loaded_example = use_state(|| None::<RustExample>);
    let rust_load_gen = use_state(|| 0u32);
    // Rc<Cell> counter avoids stale closure reads of use_state in callbacks
    let rust_load_counter = use_mut_ref(|| Rc::new(Cell::new(0u32)));
    let rust_switch_value = use_state(|| 0u8);
    // Use Rc<Cell> for immediate stop flag visibility in Rust pipeline
    let rust_stop_requested = use_mut_ref(|| Rc::new(Cell::new(false)));
    // Use Rc<Cell> for switch state during Rust run - avoids race with cpu_handle updates
    let rust_shared_switches = use_mut_ref(|| Rc::new(Cell::new(0u8)));
    // Shared UART input queue for Rust tab - run loop drains this each tick
    let rust_uart_queue = use_mut_ref(|| Rc::new(RefCell::new(VecDeque::<u8>::new())));
    // Shared flag for UART clear during animated run
    let rust_uart_clear_flag = use_mut_ref(|| Rc::new(Cell::new(false)));

    // C pipeline state - separate CPU for C tab execution
    let c_cpu = use_state(WasmCpu::new);
    let c_emu_state = use_state(EmulatorState::default);
    let c_is_loaded = use_state(|| false);
    let c_is_running = use_state(|| false);
    let c_loaded_example = use_state(|| None::<CExample>);
    let c_load_gen = use_state(|| 0u32);
    let c_load_counter = use_mut_ref(|| Rc::new(Cell::new(0u32)));
    let c_switch_value = use_state(|| 0u8);
    let c_stop_requested = use_mut_ref(|| Rc::new(Cell::new(false)));
    let c_shared_switches = use_mut_ref(|| Rc::new(Cell::new(0u8)));
    let c_uart_queue = use_mut_ref(|| Rc::new(RefCell::new(VecDeque::<u8>::new())));
    let c_uart_clear_flag = use_mut_ref(|| Rc::new(Cell::new(false)));

    // State management
    let cpu = use_state(WasmCpu::new);
    let program_code = use_state(|| String::from(EXAMPLE_PROGRAM));
    let asm_example_name = use_state(|| Some("Blink LED".to_string()));
    let assembly_output = use_state(|| None::<Html>);
    let assembly_lines = use_state(Vec::<String>::new);
    let asm_emu_state = use_state(EmulatorState::default);
    let asm_switch_value = use_state(|| 0u8);
    let challenge_mode = use_state(|| false);
    let current_challenge_id = use_state(|| None::<usize>);
    let challenge_result = use_state(|| None::<Result<String, String>>);

    // Track whether assembly succeeded (enables Step/Run)
    let asm_assembled = use_state(|| false);
    let asm_load_gen = use_state(|| 0u32);
    let asm_load_counter = use_mut_ref(|| Rc::new(Cell::new(0u32)));

    // Animated run state for assembler tab
    let asm_is_running = use_state(|| false);
    // Use Rc<Cell> for stop flag - provides immediate visibility across closures
    let asm_stop_requested = use_mut_ref(|| Rc::new(Cell::new(false)));
    // Use Rc<Cell> for switch state during run - avoids race with cpu_handle updates
    let shared_switches = use_mut_ref(|| Rc::new(Cell::new(0u8)));
    // Shared UART input queue for assembler tab - run loop drains this each tick
    let asm_uart_queue = use_mut_ref(|| Rc::new(RefCell::new(VecDeque::<u8>::new())));
    // Shared flag for UART clear during animated run
    let asm_uart_clear_flag = use_mut_ref(|| Rc::new(Cell::new(false)));
    // Shared run speed: delay in ms between instructions (all tabs share this)
    // Default 10ms/instruction (~100 instr/sec)
    let run_speed_ms = use_mut_ref(|| Rc::new(Cell::new(10u32)));

    // Modal states
    let tutorial_open = use_state(|| false);
    let examples_open = use_state(|| false);
    let rust_examples_open = use_state(|| false);
    let c_examples_open = use_state(|| false);
    let challenges_open = use_state(|| false);
    let isa_ref_open = use_state(|| false);
    let help_open = use_state(|| false);

    // Callbacks for modals
    let close_tutorial = {
        let tutorial_open = tutorial_open.clone();
        Callback::from(move |_| tutorial_open.set(false))
    };
    let close_examples = {
        let examples_open = examples_open.clone();
        Callback::from(move |_| examples_open.set(false))
    };
    let close_rust_examples = {
        let rust_examples_open = rust_examples_open.clone();
        Callback::from(move |_| rust_examples_open.set(false))
    };
    let close_c_examples = {
        let c_examples_open = c_examples_open.clone();
        Callback::from(move |_| c_examples_open.set(false))
    };
    let close_challenges = {
        let challenges_open = challenges_open.clone();
        Callback::from(move |_| challenges_open.set(false))
    };
    let close_isa_ref = {
        let isa_ref_open = isa_ref_open.clone();
        Callback::from(move |_| isa_ref_open.set(false))
    };
    let close_help = {
        let help_open = help_open.clone();
        Callback::from(move |_| help_open.set(false))
    };

    // Sidebar buttons with inline callbacks
    let sidebar_buttons = vec![
        SidebarButton {
            label: "Tutorial".to_string(),
            onclick: {
                let tutorial_open = tutorial_open.clone();
                Callback::from(move |_| tutorial_open.set(true))
            },
            title: Some("Learn COR24 basics".to_string()),
            href: None,
        },
        SidebarButton {
            label: "ISA Ref".to_string(),
            onclick: {
                let isa_ref_open = isa_ref_open.clone();
                Callback::from(move |_| isa_ref_open.set(true))
            },
            title: Some("Instruction reference".to_string()),
            href: None,
        },
        SidebarButton {
            label: "Help".to_string(),
            onclick: {
                let help_open = help_open.clone();
                Callback::from(move |_| help_open.set(true))
            },
            title: Some("Usage help".to_string()),
            href: None,
        },
        SidebarButton {
            label: "Blog".to_string(),
            onclick: Callback::noop(),
            title: Some("SW-Lab Blog".to_string()),
            href: Some("https://software-wrighter-lab.github.io/".to_string()),
        },
        SidebarButton {
            label: "Discord".to_string(),
            onclick: Callback::noop(),
            title: Some("SW-Lab Discord".to_string()),
            href: Some("https://discord.com/invite/Ctzk5uHggZ".to_string()),
        },
        SidebarButton {
            label: "MakerLisp".to_string(),
            onclick: Callback::noop(),
            title: Some("MakerLisp".to_string()),
            href: Some("https://www.makerlisp.com/".to_string()),
        },
    ];

    // CPU operation callbacks
    let on_assemble = {
        let cpu = cpu.clone();
        let assembly_output = assembly_output.clone();
        let assembly_lines = assembly_lines.clone();
        let program_code = program_code.clone();
        let asm_assembled = asm_assembled.clone();
        let asm_emu_state = asm_emu_state.clone();

        Callback::from(move |code: String| {
            program_code.set(code.clone());

            // Assemble the source code
            let mut new_cpu = (*cpu).clone();
            match new_cpu.assemble(&code) {
                Ok(_output) => {
                    // Get assembled lines for display
                    let lines = new_cpu.get_assembled_lines();
                    assembly_lines.set(lines);
                    asm_emu_state.set(capture_cpu_state_initial(&new_cpu));
                    cpu.set(new_cpu);
                    asm_assembled.set(true);

                    assembly_output.set(Some(html! {
                        <div class="success-text">
                            {"✓ Program assembled successfully"}
                        </div>
                    }));
                }
                Err(e) => {
                    assembly_lines.set(Vec::new());
                    asm_assembled.set(false);
                    assembly_output.set(Some(html! {
                        <div class="error-text">
                            {format!("Assembly error: {:?}", e)}
                        </div>
                    }));
                }
            }
        })
    };

    let on_step = {
        let cpu = cpu.clone();
        let assembly_output = assembly_output.clone();
        let asm_emu_state = asm_emu_state.clone();

        Callback::from(move |count: u32| {
            let mut new_cpu = (*cpu).clone();
            let prev_state = (*asm_emu_state).clone();

            for _ in 0..count {
                if new_cpu.is_halted() { break; }
                if let Err(e) = new_cpu.step() {
                    assembly_output.set(Some(html! {
                        <div class="error-text">
                            {format!("Error: {:?}", e)}
                        </div>
                    }));
                    break;
                }
            }

            asm_emu_state.set(capture_cpu_state(&new_cpu, &prev_state));
            cpu.set(new_cpu);
        })
    };

    let on_run = make_run_callback(
        cpu.clone(), asm_is_running.clone(), asm_emu_state.clone(),
        asm_stop_requested.borrow().clone(), shared_switches.borrow().clone(),
        asm_uart_queue.borrow().clone(), asm_uart_clear_flag.borrow().clone(),
        run_speed_ms.borrow().clone(),
    );

    let on_stop = {
        let stop_flag = asm_stop_requested.borrow().clone();
        Callback::from(move |()| {
            stop_flag.set(true);
        })
    };

    let on_reset = {
        let cpu = cpu.clone();
        let assembly_lines = assembly_lines.clone();
        let asm_emu_state = asm_emu_state.clone();
        let program_code = program_code.clone();

        Callback::from(move |()| {
            // Reset CPU and re-assemble current program so Step/Run stay enabled
            let mut new_cpu = WasmCpu::new();
            let code = (*program_code).clone();
            if !code.is_empty()
                && new_cpu.assemble(&code).is_ok()
            {
                assembly_lines.set(new_cpu.get_assembled_lines());
                asm_emu_state.set(capture_cpu_state_initial(&new_cpu));
                cpu.set(new_cpu);
                return;
            }
            // Fallback: no code or assembly failed — full reset
            assembly_lines.set(Vec::new());
            asm_emu_state.set(EmulatorState::default());
            cpu.set(new_cpu);
        })
    };

    // Tab change callback
    let on_tab_change = {
        let active_tab = active_tab.clone();
        Callback::from(move |tab: String| {
            active_tab.set(tab);
        })
    };

    // Rust pipeline: Load example
    let on_rust_load = {
        let rust_cpu = rust_cpu.clone();
        let rust_emu_state = rust_emu_state.clone();
        let rust_is_loaded = rust_is_loaded.clone();
        let rust_loaded_example = rust_loaded_example.clone();
        let rust_load_gen = rust_load_gen.clone();
        let rust_load_counter = rust_load_counter.borrow().clone();

        Callback::from(move |example: RustExample| {
            let new_gen = rust_load_counter.get() + 1;
            rust_load_counter.set(new_gen);
            rust_load_gen.set(new_gen);
            rust_loaded_example.set(Some(example.clone()));
            let mut new_cpu = WasmCpu::new();
            // Assemble the COR24 assembly from the example
            if new_cpu.assemble(&example.cor24_assembly).is_ok() {
                // Get assembled lines for display
                let assembled_lines = new_cpu.get_assembled_lines();

                let regs = new_cpu.get_registers();
                let mut registers = [0u32; 8];
                for (i, &val) in regs.iter().enumerate().take(8) {
                    registers[i] = val;
                }
                let memory_low = new_cpu.get_sparse_sram();
                let mut memory_io_led = Vec::with_capacity(16);
                for addr in 0xFF0000..0xFF0010 {
                    memory_io_led.push(new_cpu.read_byte(addr));
                }
                let mut memory_io_uart = Vec::with_capacity(16);
                for addr in 0xFF0100..0xFF0110 {
                    memory_io_uart.push(new_cpu.read_byte(addr));
                }
                let memory_stack = new_cpu.get_sparse_ebr();
                let program_end = new_cpu.get_program_end();

                rust_emu_state.set(EmulatorState {
                    registers,
                    prev_registers: registers, // No changes on initial load
                    prev_prev_registers: registers,
                    pc: new_cpu.get_pc(),
                    condition_flag: new_cpu.get_condition_flag(),
                    is_halted: new_cpu.is_halted(),
                    led_value: new_cpu.get_led_value(),
                    led_duty_cycle: if (new_cpu.get_led_value() & 1) == 1 { 1.0 } else { 0.0 },
                    led_on_count: 0,
                    instruction_count: new_cpu.get_instruction_count(),
                    memory_low: memory_low.clone(),
                    memory_io_led: memory_io_led.clone(),
                    memory_io_uart: memory_io_uart.clone(),
                    memory_stack: memory_stack.clone(),
                    program_end,
                    prev_memory_low: memory_low.clone(),
                    prev_memory_io_led: memory_io_led.clone(),
                    prev_memory_io_uart: memory_io_uart.clone(),
                    prev_memory_stack: memory_stack.clone(),
                    prev_prev_memory_low: memory_low,
                    prev_prev_memory_io_led: memory_io_led,
                    prev_prev_memory_io_uart: memory_io_uart,
                    prev_prev_memory_stack: memory_stack,
                    current_instruction: new_cpu.get_current_instruction(),
                    assembled_lines,
                    uart_output: new_cpu.get_uart_output(),
                    trace_lines: new_cpu.get_trace_lines(100),
                });

                rust_cpu.set(new_cpu);
                rust_is_loaded.set(true);
            }
        })
    };

    // Rust pipeline: Step N instructions
    let on_rust_step = make_step_callback(rust_cpu.clone(), rust_emu_state.clone());

    // Rust pipeline: Run with stop button and switch support
    let on_rust_run = make_run_callback(
        rust_cpu.clone(), rust_is_running.clone(), rust_emu_state.clone(),
        rust_stop_requested.borrow().clone(), rust_shared_switches.borrow().clone(),
        rust_uart_queue.borrow().clone(), rust_uart_clear_flag.borrow().clone(),
        run_speed_ms.borrow().clone(),
    );

    // Rust pipeline: Stop execution
    let on_rust_stop = {
        let stop_flag = rust_stop_requested.clone();
        Callback::from(move |()| {
            stop_flag.borrow().set(true);
        })
    };

    // Rust pipeline: Toggle switch
    let on_rust_switch_toggle = make_switch_callback(
        rust_switch_value.clone(), rust_cpu.clone(), rust_shared_switches.borrow().clone(),
    );

    // Rust pipeline: UART send
    let on_rust_uart_send = make_uart_send_callback(
        rust_cpu.clone(), rust_emu_state.clone(), rust_is_running.clone(),
        rust_uart_queue.borrow().clone(),
    );

    // Rust pipeline: Reset
    let on_rust_reset = {
        let rust_cpu = rust_cpu.clone();
        let rust_emu_state = rust_emu_state.clone();
        let rust_loaded_example = rust_loaded_example.clone();

        Callback::from(move |()| {
            if let Some(example) = &*rust_loaded_example {
                let mut new_cpu = WasmCpu::new();
                if new_cpu.assemble(&example.cor24_assembly).is_ok() {
                    let assembled_lines = new_cpu.get_assembled_lines();

                    let regs = new_cpu.get_registers();
                    let mut registers = [0u32; 8];
                    for (i, &val) in regs.iter().enumerate().take(8) {
                        registers[i] = val;
                    }
                    let memory_low = new_cpu.get_sparse_sram();
                    let mut memory_io_led = Vec::with_capacity(16);
                    for addr in 0xFF0000..0xFF0010 {
                        memory_io_led.push(new_cpu.read_byte(addr));
                    }
                    let mut memory_io_uart = Vec::with_capacity(16);
                    for addr in 0xFF0100..0xFF0110 {
                        memory_io_uart.push(new_cpu.read_byte(addr));
                    }
                    let memory_stack = new_cpu.get_sparse_ebr();
                    let program_end = new_cpu.get_program_end();

                    rust_emu_state.set(EmulatorState {
                        registers,
                        prev_registers: registers, // No changes on reset
                        prev_prev_registers: registers,
                        pc: new_cpu.get_pc(),
                        condition_flag: new_cpu.get_condition_flag(),
                        is_halted: new_cpu.is_halted(),
                        led_value: new_cpu.get_led_value(),
                        led_duty_cycle: if (new_cpu.get_led_value() & 1) == 1 { 1.0 } else { 0.0 },
                        led_on_count: 0,
                        instruction_count: new_cpu.get_instruction_count(),
                        memory_low: memory_low.clone(),
                        memory_io_led: memory_io_led.clone(),
                        memory_io_uart: memory_io_uart.clone(),
                        memory_stack: memory_stack.clone(),
                        program_end,
                        prev_memory_low: memory_low.clone(),
                        prev_memory_io_led: memory_io_led.clone(),
                        prev_memory_io_uart: memory_io_uart.clone(),
                        prev_memory_stack: memory_stack.clone(),
                        prev_prev_memory_low: memory_low,
                        prev_prev_memory_io_led: memory_io_led,
                        prev_prev_memory_io_uart: memory_io_uart,
                        prev_prev_memory_stack: memory_stack,
                        current_instruction: new_cpu.get_current_instruction(),
                        assembled_lines,
                        uart_output: new_cpu.get_uart_output(),
                        trace_lines: new_cpu.get_trace_lines(100),
                    });

                    rust_cpu.set(new_cpu);
                }
            }
        })
    };

    // Rust pipeline: Unload (clear loaded state)
    // Tab definitions
    let tabs = vec![
        Tab { id: "assembler".to_string(), label: "Assembler".to_string(), tooltip: Some("Write and run COR24 assembly directly".to_string()) },
        Tab { id: "c".to_string(), label: "C".to_string(), tooltip: Some("C → COR24 compilation pipeline (Luther Johnson's cc24)".to_string()) },
        Tab { id: "rust".to_string(), label: "Rust".to_string(), tooltip: Some("Rust → MSP430 → COR24 compilation pipeline".to_string()) },
    ];

    // Get examples for the modal
    let examples = get_examples();

    // Pre-built Rust examples
    let rust_examples = crate::rust_examples::get_rust_examples();

    // Pre-built C examples
    let c_examples = crate::c_examples::get_c_examples();

    // Assembler switch toggle callback
    let on_asm_switch_toggle = make_switch_callback(
        asm_switch_value.clone(), cpu.clone(), shared_switches.borrow().clone(),
    );

    let on_asm_uart_send = make_uart_send_callback(
        cpu.clone(), asm_emu_state.clone(), asm_is_running.clone(),
        asm_uart_queue.borrow().clone(),
    );
    let on_asm_uart_clear = make_uart_clear_callback(
        cpu.clone(), asm_emu_state.clone(), asm_is_running.clone(),
        asm_uart_clear_flag.borrow().clone(),
    );
    let on_rust_uart_clear = make_uart_clear_callback(
        rust_cpu.clone(), rust_emu_state.clone(), rust_is_running.clone(),
        rust_uart_clear_flag.borrow().clone(),
    );

    // C pipeline: Load example
    let on_c_load = {
        let c_cpu = c_cpu.clone();
        let c_emu_state = c_emu_state.clone();
        let c_is_loaded = c_is_loaded.clone();
        let c_loaded_example = c_loaded_example.clone();
        let c_load_gen = c_load_gen.clone();
        let c_load_counter = c_load_counter.borrow().clone();

        Callback::from(move |example: CExample| {
            let new_gen = c_load_counter.get() + 1;
            c_load_counter.set(new_gen);
            c_load_gen.set(new_gen);
            c_loaded_example.set(Some(example.clone()));
            let mut new_cpu = WasmCpu::new();
            if new_cpu.assemble(&example.cor24_assembly).is_ok() {
                let assembled_lines = new_cpu.get_assembled_lines();
                let regs = new_cpu.get_registers();
                let mut registers = [0u32; 8];
                for (i, &val) in regs.iter().enumerate().take(8) {
                    registers[i] = val;
                }
                let memory_low = new_cpu.get_sparse_sram();
                let mut memory_io_led = Vec::with_capacity(16);
                for addr in 0xFF0000..0xFF0010 {
                    memory_io_led.push(new_cpu.read_byte(addr));
                }
                let mut memory_io_uart = Vec::with_capacity(16);
                for addr in 0xFF0100..0xFF0110 {
                    memory_io_uart.push(new_cpu.read_byte(addr));
                }
                let memory_stack = new_cpu.get_sparse_ebr();
                let program_end = new_cpu.get_program_end();

                c_emu_state.set(EmulatorState {
                    registers,
                    prev_registers: registers,
                    prev_prev_registers: registers,
                    pc: new_cpu.get_pc(),
                    condition_flag: new_cpu.get_condition_flag(),
                    is_halted: new_cpu.is_halted(),
                    led_value: new_cpu.get_led_value(),
                    led_duty_cycle: if (new_cpu.get_led_value() & 1) == 1 { 1.0 } else { 0.0 },
                    led_on_count: 0,
                    instruction_count: new_cpu.get_instruction_count(),
                    memory_low: memory_low.clone(),
                    memory_io_led: memory_io_led.clone(),
                    memory_io_uart: memory_io_uart.clone(),
                    memory_stack: memory_stack.clone(),
                    program_end,
                    prev_memory_low: memory_low.clone(),
                    prev_memory_io_led: memory_io_led.clone(),
                    prev_memory_io_uart: memory_io_uart.clone(),
                    prev_memory_stack: memory_stack.clone(),
                    prev_prev_memory_low: memory_low,
                    prev_prev_memory_io_led: memory_io_led,
                    prev_prev_memory_io_uart: memory_io_uart,
                    prev_prev_memory_stack: memory_stack,
                    current_instruction: new_cpu.get_current_instruction(),
                    assembled_lines,
                    uart_output: new_cpu.get_uart_output(),
                    trace_lines: new_cpu.get_trace_lines(100),
                });

                c_cpu.set(new_cpu);
                c_is_loaded.set(true);
            }
        })
    };

    let on_c_step = make_step_callback(c_cpu.clone(), c_emu_state.clone());

    let on_c_run = make_run_callback(
        c_cpu.clone(), c_is_running.clone(), c_emu_state.clone(),
        c_stop_requested.borrow().clone(), c_shared_switches.borrow().clone(),
        c_uart_queue.borrow().clone(), c_uart_clear_flag.borrow().clone(),
        run_speed_ms.borrow().clone(),
    );

    // C pipeline: Stop execution
    let on_c_stop = {
        let stop_flag = c_stop_requested.clone();
        Callback::from(move |()| {
            stop_flag.borrow().set(true);
        })
    };

    // C pipeline: Toggle switch
    let on_c_switch_toggle = make_switch_callback(
        c_switch_value.clone(), c_cpu.clone(), c_shared_switches.borrow().clone(),
    );
    let on_c_uart_send = make_uart_send_callback(
        c_cpu.clone(), c_emu_state.clone(), c_is_running.clone(),
        c_uart_queue.borrow().clone(),
    );
    let on_c_uart_clear = make_uart_clear_callback(
        c_cpu.clone(), c_emu_state.clone(), c_is_running.clone(),
        c_uart_clear_flag.borrow().clone(),
    );

    // C pipeline: Reset
    let on_c_reset = {
        let c_cpu = c_cpu.clone();
        let c_emu_state = c_emu_state.clone();
        let c_loaded_example = c_loaded_example.clone();

        Callback::from(move |()| {
            if let Some(example) = &*c_loaded_example {
                let mut new_cpu = WasmCpu::new();
                if new_cpu.assemble(&example.cor24_assembly).is_ok() {
                    let assembled_lines = new_cpu.get_assembled_lines();
                    let regs = new_cpu.get_registers();
                    let mut registers = [0u32; 8];
                    for (i, &val) in regs.iter().enumerate().take(8) {
                        registers[i] = val;
                    }
                    let memory_low = new_cpu.get_sparse_sram();
                    let mut memory_io_led = Vec::with_capacity(16);
                    for addr in 0xFF0000..0xFF0010 {
                        memory_io_led.push(new_cpu.read_byte(addr));
                    }
                    let mut memory_io_uart = Vec::with_capacity(16);
                    for addr in 0xFF0100..0xFF0110 {
                        memory_io_uart.push(new_cpu.read_byte(addr));
                    }
                    let memory_stack = new_cpu.get_sparse_ebr();
                    let program_end = new_cpu.get_program_end();

                    c_emu_state.set(EmulatorState {
                        registers,
                        prev_registers: registers,
                        prev_prev_registers: registers,
                        pc: new_cpu.get_pc(),
                        condition_flag: new_cpu.get_condition_flag(),
                        is_halted: new_cpu.is_halted(),
                        led_value: new_cpu.get_led_value(),
                        led_duty_cycle: if (new_cpu.get_led_value() & 1) == 1 { 1.0 } else { 0.0 },
                        led_on_count: 0,
                        instruction_count: new_cpu.get_instruction_count(),
                        memory_low: memory_low.clone(),
                        memory_io_led: memory_io_led.clone(),
                        memory_io_uart: memory_io_uart.clone(),
                        memory_stack: memory_stack.clone(),
                        program_end,
                        prev_memory_low: memory_low.clone(),
                        prev_memory_io_led: memory_io_led.clone(),
                        prev_memory_io_uart: memory_io_uart.clone(),
                        prev_memory_stack: memory_stack.clone(),
                        prev_prev_memory_low: memory_low,
                        prev_prev_memory_io_led: memory_io_led,
                        prev_prev_memory_io_uart: memory_io_uart,
                        prev_prev_memory_stack: memory_stack,
                        current_instruction: new_cpu.get_current_instruction(),
                        assembled_lines,
                        uart_output: new_cpu.get_uart_output(),
                        trace_lines: new_cpu.get_trace_lines(100),
                    });

                    c_cpu.set(new_cpu);
                }
            }
        })
    };

    html! {
        <div class="container">
            <Tooltip />
            <Header title="MakerLisp COR24 — Assembly Emulator">
                <TabBar tabs={tabs} active_tab={(*active_tab).clone()} on_tab_change={on_tab_change} />
            </Header>

            // Only show main sidebar on Assembler tab
            if *active_tab == "assembler" {
                <Sidebar buttons={sidebar_buttons} />
            }

            // Assembler Tab Content
            <div class={if *active_tab == "assembler" { "main-content" } else { "main-content hidden" }}>
                <div class="editor-column">
                <div class="editor-toolbar">
                    <button class="toolbar-btn"
                        data-tooltip="Load example programs"
                        onclick={{
                            let examples_open = examples_open.clone();
                            Callback::from(move |_: MouseEvent| examples_open.set(true))
                        }}>{"Examples"}</button>
                    <button class="toolbar-btn"
                        data-tooltip="Test your skills"
                        onclick={{
                            let challenges_open = challenges_open.clone();
                            Callback::from(move |_: MouseEvent| challenges_open.set(true))
                        }}>{"Challenges"}</button>
                </div>
                <ProgramArea
                    on_assemble={on_assemble}
                    on_step={{
                        let on_step = on_step.clone();
                        Callback::from(move |()| on_step.emit(1))
                    }}
                    on_run={on_run.clone()}
                    on_stop={{
                        let stop_flag = asm_stop_requested.borrow().clone();
                        Callback::from(move |()| stop_flag.set(true))
                    }}
                    on_reset={{
                        let on_reset = on_reset.clone();
                        Callback::from(move |()| on_reset.emit(()))
                    }}
                    is_running={*asm_is_running}
                    assembly_output={
                        if !assembly_lines.is_empty() {
                            // Show highlighted assembly lines
                            let pc = (*cpu).pc();
                            Some(html! {
                                <div>
                                    {for assembly_lines.iter().map(|line| {
                                        // Parse address from "ADDR: BYTES SOURCE" format
                                        let is_current = if line.len() > 4 && line.chars().nth(4) == Some(':') {
                                            if let Ok(addr) = u32::from_str_radix(&line[0..4], 16) {
                                                addr == pc
                                            } else {
                                                false
                                            }
                                        } else {
                                            false
                                        };

                                        let class = if is_current {
                                            "assembly-line current"
                                        } else {
                                            "assembly-line"
                                        };

                                        html! {
                                            <div class={class}>{line}</div>
                                        }
                                    })}
                                </div>
                            })
                        } else {
                            // Show success/error messages
                            (*assembly_output).clone()
                        }
                    }
                    initial_code={Some((*program_code).clone())}
                    load_generation={*asm_load_gen}
                    step_enabled={*asm_assembled && !(*cpu).is_halted()}
                    run_enabled={*asm_assembled && !(*cpu).is_halted()}
                    show_exec_buttons={false}
                    example_name={(*asm_example_name).clone()}
                />
                </div> // editor-column

                <div class="right-panels">
                    <DebugPanel
                        cpu_state={(*asm_emu_state).clone()}
                        is_loaded={*asm_assembled}
                        is_running={*asm_is_running}
                        on_step={on_step.clone()}
                        on_run={on_run.clone()}
                        on_stop={on_stop}
                        on_reset={on_reset}
                        switch_value={*asm_switch_value}
                        on_switch_toggle={on_asm_switch_toggle}
                        on_uart_send={on_asm_uart_send}
                        on_uart_clear={on_asm_uart_clear}
                        listing_scroll_id={"asm-debug-listing-scroll".to_string()}
                        show_listing={false}
                        run_speed_ms={Some(run_speed_ms.borrow().clone())}
                    />
                </div>
            </div>

            // Rust Pipeline Tab Content - Full width wizard layout
            <div class={if *active_tab == "rust" { "rust-tab-content full-width" } else { "rust-tab-content hidden" }}>
                <RustPipeline
                    examples={rust_examples.clone()}
                    loaded_example={(*rust_loaded_example).clone()}
                    load_generation={*rust_load_gen}
                    on_load={on_rust_load.clone()}
                    on_step={on_rust_step}
                    on_run={on_rust_run}
                    on_stop={on_rust_stop}
                    on_reset={on_rust_reset}
                    cpu_state={(*rust_emu_state).clone()}
                    is_loaded={*rust_is_loaded}
                    is_running={*rust_is_running}
                    switch_value={*rust_switch_value}
                    on_switch_toggle={on_rust_switch_toggle}
                    on_uart_send={on_rust_uart_send}
                    on_uart_clear={on_rust_uart_clear}
                    run_speed_ms={Some(run_speed_ms.borrow().clone())}
                    on_tutorial_open={
                        let tutorial_open = tutorial_open.clone();
                        Callback::from(move |_| tutorial_open.set(true))
                    }
                    on_examples_open={
                        let rust_examples_open = rust_examples_open.clone();
                        Callback::from(move |_| rust_examples_open.set(true))
                    }
                    on_isa_ref_open={
                        let isa_ref_open = isa_ref_open.clone();
                        Callback::from(move |_| isa_ref_open.set(true))
                    }
                    on_help_open={
                        let help_open = help_open.clone();
                        Callback::from(move |_| help_open.set(true))
                    }
                />
            </div>

            // C Pipeline Tab Content
            <div class={if *active_tab == "c" { "rust-tab-content full-width" } else { "rust-tab-content hidden" }}>
                <CPipeline
                    examples={c_examples.clone()}
                    loaded_example={(*c_loaded_example).clone()}
                    load_generation={*c_load_gen}
                    on_load={on_c_load.clone()}
                    on_step={on_c_step}
                    on_run={on_c_run}
                    on_stop={on_c_stop}
                    on_reset={on_c_reset}
                    cpu_state={(*c_emu_state).clone()}
                    is_loaded={*c_is_loaded}
                    is_running={*c_is_running}
                    switch_value={*c_switch_value}
                    on_switch_toggle={on_c_switch_toggle}
                    on_uart_send={on_c_uart_send}
                    on_uart_clear={on_c_uart_clear}
                    run_speed_ms={Some(run_speed_ms.borrow().clone())}
                    on_tutorial_open={
                        let tutorial_open = tutorial_open.clone();
                        Callback::from(move |_| tutorial_open.set(true))
                    }
                    on_examples_open={
                        let c_examples_open = c_examples_open.clone();
                        Callback::from(move |_| c_examples_open.set(true))
                    }
                    on_isa_ref_open={
                        let isa_ref_open = isa_ref_open.clone();
                        Callback::from(move |_| isa_ref_open.set(true))
                    }
                    on_help_open={
                        let help_open = help_open.clone();
                        Callback::from(move |_| help_open.set(true))
                    }
                />
            </div>

            // Challenge Mode Banner
            if *challenge_mode {
                if let Some(challenge_id) = *current_challenge_id {
                    <div class="challenge-banner">
                        <span class="challenge-indicator">{"⚡"}</span>
                        <span class="challenge-text">
                            {format!("Challenge Mode - Challenge {}", challenge_id)}
                        </span>
                        <button
                            class="check-button"
                            onclick={
                                let challenge_result = challenge_result.clone();
                                let program_code = program_code.clone();
                                Callback::from(move |_| {
                                    match validate_challenge(challenge_id, &program_code) {
                                        Ok(passed) => {
                                            if passed {
                                                challenge_result.set(Some(Ok(format!("✅ Challenge {} PASSED!", challenge_id))));
                                            } else {
                                                challenge_result.set(Some(Err(format!("❌ Challenge {} did not pass. Check your solution.", challenge_id))));
                                            }
                                        }
                                        Err(e) => {
                                            challenge_result.set(Some(Err(format!("Validation error: {:?}", e))));
                                        }
                                    }
                                })
                            }
                        >
                            {"Check Solution"}
                        </button>
                        <button
                            class="exit-button"
                            onclick={
                                let challenge_mode = challenge_mode.clone();
                                let current_challenge_id = current_challenge_id.clone();
                                let challenge_result = challenge_result.clone();
                                Callback::from(move |_| {
                                    challenge_mode.set(false);
                                    current_challenge_id.set(None);
                                    challenge_result.set(None);
                                })
                            }
                        >
                            {"Exit"}
                        </button>
                    </div>
                }
            }

            // Success/Error Banners
            {
                if let Some(result) = &*challenge_result {
                    match result {
                        Ok(message) => html! {
                            <div class="success-banner">
                                <span class="banner-content">{message}</span>
                                <button
                                    class="dismiss-button"
                                    onclick={
                                        let challenge_result = challenge_result.clone();
                                        Callback::from(move |_| challenge_result.set(None))
                                    }
                                >
                                    {"×"}
                                </button>
                            </div>
                        },
                        Err(message) => html! {
                            <div class="error-banner">
                                <span class="banner-content">{message}</span>
                                <button
                                    class="dismiss-button"
                                    onclick={
                                        let challenge_result = challenge_result.clone();
                                        Callback::from(move |_| challenge_result.set(None))
                                    }
                                >
                                    {"×"}
                                </button>
                            </div>
                        }
                    }
                } else {
                    html! {}
                }
            }

            // Modals
            <Modal id="tutorial" title="Tutorial" active={*tutorial_open} on_close={close_tutorial}>
                {Html::from_html_unchecked(AttrValue::from(TUTORIAL_CONTENT))}
            </Modal>

            <ExamplePicker
                id="asm-examples"
                title="COR24 Assembler Examples"
                examples={examples.iter().map(|(t, d, _)| ExampleItem { name: t.clone(), description: d.clone() }).collect::<Vec<_>>()}
                active={*examples_open}
                on_close={close_examples}
                on_select={{
                    let examples = examples.clone();
                    let program_code = program_code.clone();
                    let examples_open = examples_open.clone();
                    let cpu = cpu.clone();
                    let assembly_output = assembly_output.clone();
                    let assembly_lines = assembly_lines.clone();
                    let challenge_mode = challenge_mode.clone();
                    let current_challenge_id = current_challenge_id.clone();
                    let challenge_result = challenge_result.clone();
                    let asm_is_running = asm_is_running.clone();
                    let asm_assembled = asm_assembled.clone();
                    let asm_emu_state = asm_emu_state.clone();
                    let stop_flag = asm_stop_requested.borrow().clone();
                    let asm_load_gen = asm_load_gen.clone();
                    let asm_load_counter = asm_load_counter.borrow().clone();
                    let asm_example_name = asm_example_name.clone();
                    Callback::from(move |idx: usize| {
                        if let Some((name, _, code)) = examples.get(idx) {
                            asm_example_name.set(Some(name.clone()));
                            // Stop any running animation loop
                            stop_flag.set(true);
                            asm_is_running.set(false);
                            asm_assembled.set(false);
                            cpu.set(WasmCpu::new());
                            asm_emu_state.set(EmulatorState::default());
                            assembly_output.set(None);
                            assembly_lines.set(Vec::new());
                            challenge_mode.set(false);
                            current_challenge_id.set(None);
                            challenge_result.set(None);
                            let new_gen = asm_load_counter.get() + 1;
                            asm_load_counter.set(new_gen);
                            asm_load_gen.set(new_gen);
                            program_code.set(code.clone());
                            examples_open.set(false);
                        }
                    })
                }}
            />

            <ExamplePicker
                id="rust-examples"
                title={format!("Rust \u{2192} MSP430 \u{2192} COR24 Examples")}
                examples={rust_examples.iter().map(|ex| ExampleItem { name: ex.name.clone(), description: ex.description.clone() }).collect::<Vec<_>>()}
                active={*rust_examples_open}
                on_close={close_rust_examples}
                on_select={{
                    let rust_examples = rust_examples.clone();
                    let on_rust_load = on_rust_load.clone();
                    let rust_examples_open = rust_examples_open.clone();
                    Callback::from(move |idx: usize| {
                        if let Some(example) = rust_examples.get(idx) {
                            on_rust_load.emit(example.clone());
                            rust_examples_open.set(false);
                        }
                    })
                }}
            />

            <ExamplePicker
                id="c-examples"
                title={format!("C \u{2192} COR24 Examples (Luther Johnson's cc24)")}
                examples={c_examples.iter().map(|ex| ExampleItem { name: ex.name.clone(), description: ex.description.clone() }).collect::<Vec<_>>()}
                active={*c_examples_open}
                on_close={close_c_examples}
                on_select={{
                    let c_examples = c_examples.clone();
                    let on_c_load = on_c_load.clone();
                    let c_examples_open = c_examples_open.clone();
                    Callback::from(move |idx: usize| {
                        if let Some(example) = c_examples.get(idx) {
                            on_c_load.emit(example.clone());
                            c_examples_open.set(false);
                        }
                    })
                }}
            />

            <Modal id="challenges" title="Challenges" active={*challenges_open} on_close={close_challenges}>
                {render_challenges_list(
                    challenge_mode.clone(), current_challenge_id.clone(),
                    program_code.clone(), challenges_open.clone(),
                    asm_is_running.clone(), asm_assembled.clone(),
                    asm_emu_state.clone(), cpu.clone(),
                    assembly_output.clone(), assembly_lines.clone(),
                    asm_stop_requested.borrow().clone(), challenge_result.clone(),
                    asm_load_gen.clone(), asm_load_counter.borrow().clone(),
                )}
            </Modal>

            <Modal id="isaRef" title="ISA Reference" active={*isa_ref_open} on_close={close_isa_ref}>
                {Html::from_html_unchecked(AttrValue::from(ISA_REF_CONTENT))}
            </Modal>

            <Modal id="help" title="Help" active={*help_open} on_close={close_help}>
                {Html::from_html_unchecked(AttrValue::from(HELP_CONTENT))}
            </Modal>

            // GitHub Corner
            <a href="https://github.com/sw-embed/cor24-rs" class="github-corner" aria-label="View source on GitHub" target="_blank">
                <svg width="80" height="80" viewBox="0 0 250 250" style="fill:#00d9ff; color:#1a1a2e; position: absolute; top: 0; border: 0; right: 0;" aria-hidden="true">
                    <path d="M0,0 L115,115 L130,115 L142,142 L250,250 L250,0 Z"></path>
                    <path d="M128.3,109.0 C113.8,99.7 119.0,89.6 119.0,89.6 C122.0,82.7 120.5,78.6 120.5,78.6 C119.2,72.0 123.4,76.3 123.4,76.3 C127.3,80.9 125.5,87.3 125.5,87.3 C122.9,97.6 130.6,101.9 134.4,103.2" fill="currentColor" style="transform-origin: 130px 106px;" class="octo-arm"></path>
                    <path d="M115.0,115.0 C114.9,115.1 118.7,116.5 119.8,115.4 L133.7,101.6 C136.9,99.2 139.9,98.4 142.2,98.6 C133.8,88.0 127.5,74.4 143.8,58.0 C148.5,53.4 154.0,51.2 159.7,51.0 C160.3,49.4 163.2,43.6 171.4,40.1 C171.4,40.1 176.1,42.5 178.8,56.2 C183.1,58.6 187.2,61.8 190.9,65.4 C194.5,69.0 197.7,73.2 200.1,77.6 C213.8,80.2 216.3,84.9 216.3,84.9 C212.7,93.1 206.9,96.0 205.4,96.6 C205.1,102.4 203.0,107.8 198.3,112.5 C181.9,128.9 168.3,122.5 157.7,114.1 C157.9,116.9 156.7,120.9 152.7,124.9 L141.0,136.5 C139.8,137.7 141.6,141.9 141.8,141.8 Z" fill="currentColor" class="octo-body"></path>
                </svg>
            </a>

            // Footer
            <footer class="app-footer">
                <span>{"MIT License"}</span>
                <span class="footer-sep">{"\u{00B7}"}</span>
                <span>{"© 2026 Michael A Wright"}</span>
                <span class="footer-sep">{"\u{00B7}"}</span>
                <span>{env!("VERGEN_BUILD_HOST")}</span>
                <span class="footer-sep">{"\u{00B7}"}</span>
                <span>{env!("VERGEN_GIT_SHA_SHORT")}</span>
                <span class="footer-sep">{"\u{00B7}"}</span>
                <span>{env!("VERGEN_BUILD_TIMESTAMP")}</span>
                <span class="footer-sep">{"\u{00B7}"}</span>
                <a href="https://github.com/sw-embed/cor24-rs/blob/main/CHANGES.md" target="_blank" class="footer-link">{"Changes"}</a>
            </footer>
        </div>
    }
}

// Helper function to render challenges list
#[allow(clippy::too_many_arguments)]
fn render_challenges_list(
    challenge_mode: UseStateHandle<bool>,
    current_challenge_id: UseStateHandle<Option<usize>>,
    program_code: UseStateHandle<String>,
    challenges_open: UseStateHandle<bool>,
    asm_is_running: UseStateHandle<bool>,
    asm_assembled: UseStateHandle<bool>,
    asm_emu_state: UseStateHandle<EmulatorState>,
    cpu: UseStateHandle<WasmCpu>,
    assembly_output: UseStateHandle<Option<Html>>,
    assembly_lines: UseStateHandle<Vec<String>>,
    stop_flag: Rc<Cell<bool>>,
    challenge_result: UseStateHandle<Option<Result<String, String>>>,
    asm_load_gen: UseStateHandle<u32>,
    asm_load_counter: Rc<Cell<u32>>,
) -> Html {
    let challenges = get_challenges();

    html! {
        <div class="challenges-list">
            <h3>{"Available Challenges"}</h3>
            {for challenges.iter().map(|challenge| {
                let id = challenge.id;
                let name = challenge.name.clone();
                let description = challenge.description.clone();
                let hint = challenge.hint.clone();
                let initial_code = challenge.initial_code.clone();

                let challenge_mode = challenge_mode.clone();
                let current_challenge_id = current_challenge_id.clone();
                let program_code = program_code.clone();
                let challenges_open = challenges_open.clone();
                let asm_is_running = asm_is_running.clone();
                let asm_assembled = asm_assembled.clone();
                let asm_emu_state = asm_emu_state.clone();
                let cpu = cpu.clone();
                let assembly_output = assembly_output.clone();
                let assembly_lines = assembly_lines.clone();
                let stop_flag = stop_flag.clone();
                let challenge_result = challenge_result.clone();

                html! {
                    <div class="challenge-item">
                        <button
                            class="load-challenge-btn"
                            onclick={
                                let challenge_mode = challenge_mode.clone();
                                let current_challenge_id = current_challenge_id.clone();
                                let program_code = program_code.clone();
                                let challenges_open = challenges_open.clone();
                                let initial_code = initial_code.clone();
                                let asm_is_running = asm_is_running.clone();
                                let asm_assembled = asm_assembled.clone();
                                let asm_emu_state = asm_emu_state.clone();
                                let cpu = cpu.clone();
                                let assembly_output = assembly_output.clone();
                                let assembly_lines = assembly_lines.clone();
                                let stop_flag = stop_flag.clone();
                                let challenge_result = challenge_result.clone();
                                let asm_load_gen = asm_load_gen.clone();
                                let asm_load_counter = asm_load_counter.clone();
                                Callback::from(move |_| {
                                    // Stop any running animation loop
                                    stop_flag.set(true);
                                    asm_is_running.set(false);
                                    asm_assembled.set(false);
                                    cpu.set(WasmCpu::new());
                                    asm_emu_state.set(EmulatorState::default());
                                    assembly_output.set(None);
                                    assembly_lines.set(Vec::new());
                                    challenge_result.set(None);
                                    challenge_mode.set(true);
                                    current_challenge_id.set(Some(id));
                                    let new_gen = asm_load_counter.get() + 1;
                                    asm_load_counter.set(new_gen);
                                    asm_load_gen.set(new_gen);
                                    program_code.set(initial_code.clone());
                                    challenges_open.set(false);
                                })
                            }
                        >
                            {format!("Load Challenge {}", id)}
                        </button>
                        <p><strong>{name}</strong></p>
                        <p>{description}</p>
                        <p><em>{"Hint: "}{hint}</em></p>
                    </div>
                }
            })}
        </div>
    }
}

// Constants for content
const EXAMPLE_PROGRAM: &str = "; Blink LED: Toggle LED D2 on and off
; LED D2 at 0xFF0000 (write bit 0)
; Click Run to watch the LED blink!

        la      r1,0xFF0000

loop:
        lc      r0,1
        sb      r0,0(r1)

        push    r1
        lc      r1,10
delay1: lc      r2,0
wait1:  lc      r0,1
        add     r2,r0
        lc      r0,127
        clu     r2,r0
        brt     wait1
        lc      r0,1
        sub     r1,r0
        ceq     r1,z
        brf     delay1
        pop     r1

        lc      r0,0
        sb      r0,0(r1)

        push    r1
        lc      r1,10
delay2: lc      r2,0
wait2:  lc      r0,1
        add     r2,r0
        lc      r0,127
        clu     r2,r0
        brt     wait2
        lc      r0,1
        sub     r1,r0
        ceq     r1,z
        brf     delay2
        pop     r1

        bra     loop

halt:   bra     halt";

const TUTORIAL_CONTENT: &str = r#"
<h3>COR24 Assembly Tutorial</h3>

<h4>What is the COR24?</h4>
<p>The COR24 is a 24-bit RISC processor designed by Luther Johnson for the
<a href="https://makerlisp.com" target="_blank">MakerLisp</a> project.
It runs as a soft CPU on Lattice MachXO FPGAs. This emulator lets you write,
assemble, and step through COR24 code in your browser.</p>

<h4>Getting Started</h4>
<ol>
<li>Load an example from the <strong>Examples</strong> button</li>
<li>Click <strong>Assemble</strong> to convert to machine code</li>
<li>Click <strong>Step</strong> to execute one instruction (watch registers change)</li>
<li>Click <strong>Run</strong> for continuous execution (<strong>Stop</strong> to pause)</li>
<li>Expand <strong>Instruction Trace</strong> to see recent execution history</li>
</ol>

<h4>Registers</h4>
<table style="font-size:0.85em; margin-bottom:8px">
<tr><td><code>r0, r1, r2</code></td><td>General purpose (24-bit)</td></tr>
<tr><td><code>fp (r3)</code></td><td>Frame pointer</td></tr>
<tr><td><code>sp (r4)</code></td><td>Stack pointer (init: 0xFEEC00, grows down)</td></tr>
<tr><td><code>z (r5)</code></td><td>Always zero — use in compares: <code>ceq r0,z</code></td></tr>
<tr><td><code>iv (r6)</code></td><td>Interrupt vector (ISR address)</td></tr>
<tr><td><code>ir (r7)</code></td><td>Interrupt return address</td></tr>
</table>

<h4>Loading Constants</h4>
<pre>lc  r0, 42      ; signed 8-bit (-128..127)
lcu r0, 200     ; unsigned 8-bit (0..255)
la  r0, 1000    ; 24-bit immediate (any value)</pre>

<h4>Arithmetic</h4>
<pre>add r0, r1      ; r0 = r0 + r1
add r0, 5       ; r0 = r0 + 5 (8-bit immediate)
sub r0, r1      ; r0 = r0 - r1
mul r0, r1      ; r0 = r0 * r1
sub sp, 12      ; allocate stack space</pre>

<h4>Logic &amp; Shifts</h4>
<pre>and r0, r1      ; bitwise AND
or  r0, r1      ; bitwise OR
xor r0, r1      ; bitwise XOR
shl r0, r1      ; shift left by r1
srl r0, r1      ; shift right (zero fill)
sra r0, r1      ; shift right (sign fill)</pre>

<h4>Compare &amp; Branch</h4>
<p>Compares set the condition flag <strong>C</strong>. Branches test it.</p>
<pre>ceq r0, r1      ; C = (r0 == r1)
clu r0, r1      ; C = (r0 &lt; r1) unsigned
cls r0, r1      ; C = (r0 &lt; r1) signed
bra label       ; branch always
brt label       ; branch if C = true
brf label       ; branch if C = false</pre>
<p>Example — loop while r0 &lt; 10:</p>
<pre>        lc  r0, 0
loop:
        add r0, 1
        lc  r1, 10
        clu r0, r1      ; C = (r0 &lt; 10)
        brt loop</pre>

<h4>Halting</h4>
<p>No halt instruction — use branch-to-self:</p>
<pre>halt:
        bra halt         ; emulator detects this</pre>

<h4>Register Moves</h4>
<pre>mov r0, r1      ; copy register
mov r0, c       ; r0 = condition flag (0 or 1)
mov fp, sp      ; save stack pointer
mov iv, r0      ; set interrupt vector</pre>

<h4>Memory Access</h4>
<p>Base+offset addressing. Offset is signed 8-bit. Base: r0, r1, r2, or fp.</p>
<pre>sb  r0, 0(r1)   ; store byte
sw  r0, 0(r1)   ; store word (3 bytes)
lb  r0, 0(r1)   ; load byte (sign-extended)
lbu r0, 0(r1)   ; load byte (zero-extended)
lw  r0, 0(r1)   ; load word (3 bytes)</pre>

<h4>Stack</h4>
<pre>push r0         ; sp -= 3; mem[sp] = r0
pop  r0         ; r0 = mem[sp]; sp += 3</pre>

<h4>Function Calls</h4>
<pre>; Call: load addr, jump-and-link
        la  r2, my_func
        jal r1, (r2)     ; r1 = return addr

; Return:
my_func:
        push r1           ; save return addr
        ; ... body ...
        pop  r1
        jmp  (r1)         ; return</pre>

<h4>Memory-Mapped I/O</h4>
<table style="font-size:0.85em; margin-bottom:8px">
<tr><td><code>FF0000</code></td><td>LED (write bit 0) / Button (read bit 0)</td></tr>
<tr><td><code>FF0010</code></td><td>Interrupt enable (bit 0 = UART RX)</td></tr>
<tr><td><code>FF0100</code></td><td>UART data (read=RX, write=TX)</td></tr>
<tr><td><code>FF0101</code></td><td>UART status (bit 7=TX busy, bit 1=RX ready)</td></tr>
</table>
<p>Use <code>la</code> with signed decimal: <code>la r1, -65536</code> = 0xFF0000</p>

<h4>Extensions</h4>
<pre>sxt r0          ; sign-extend byte to 24-bit
zxt r0          ; zero-extend byte to 24-bit
nop             ; no operation (0xFF, 1 byte)</pre>

<h4>Instruction Sizes</h4>
<table style="font-size:0.85em; margin-bottom:8px">
<tr><td>1 byte</td><td>add, sub, mul, and, or, xor, mov, push, pop, jmp, jal, ceq, clu, cls, shl, srl, sra, sxt, zxt</td></tr>
<tr><td>2 bytes</td><td>lc, lcu, add imm, bra, brt, brf, lb, lbu, lw, sb, sw</td></tr>
<tr><td>4 bytes</td><td>la, sub sp</td></tr>
</table>

<h4>Tips</h4>
<ul>
<li>All Assembler tab examples are editable — experiment!</li>
<li>C and Rust tab examples are read-only (compiled offline)</li>
<li>Expand <strong>Instruction Trace</strong> after stepping to see execution history</li>
<li>Use the <strong>ISA Reference</strong> button for quick instruction lookup</li>
<li>Try <strong>Assert</strong> example to see validation patterns</li>
<li>Try <strong>Loop Trace</strong> to practice Run/Stop/Trace workflow</li>
</ul>

<p style="font-size:0.85em; color:#888">Full tutorial:
<a href="https://github.com/sw-embed/cor24-rs/blob/main/docs/cor24-tutorial.md" target="_blank" style="color:#7c3aed">docs/cor24-tutorial.md</a></p>
"#;

const ISA_REF_CONTENT: &str = r#"
<h3>COR24 Instruction Set Reference</h3>
<p><em>C-Oriented RISC, 24-bit. 32 opcodes, 211 instruction forms (1, 2, or 4 bytes).
See <a href="https://makerlisp.com" target="_blank">makerlisp.com</a> for the hardware specification.</em></p>

<h4>CPU State</h4>
<p>All registers and addresses are <strong>24 bits</strong> wide (values 0 to 16,777,215).</p>
<table>
<tr><th>State</th><th>Description</th></tr>
<tr><td><strong>PC</strong></td><td>Program counter — address of next instruction. Starts at 0.</td></tr>
<tr><td><strong>C</strong></td><td>Condition flag — single bit, set by compare instructions (ceq, clu, cls), tested by branches (brt, brf). Also writable via <code>mov ra, c</code> and <code>clu z, ra</code>.</td></tr>
</table>

<h4>Registers</h4>
<table>
<tr><th>Register</th><th>Name</th><th>Width</th><th>Purpose</th></tr>
<tr><td><code>r0</code></td><td></td><td>24-bit</td><td>General purpose</td></tr>
<tr><td><code>r1</code></td><td></td><td>24-bit</td><td>General purpose / return address (jal convention)</td></tr>
<tr><td><code>r2</code></td><td></td><td>24-bit</td><td>General purpose</td></tr>
<tr><td><code>r3</code></td><td>fp</td><td>24-bit</td><td>Frame pointer — base for stack-frame locals</td></tr>
<tr><td><code>r4</code></td><td>sp</td><td>24-bit</td><td>Stack pointer — init 0xFEEC00, grows downward (3 bytes per push)</td></tr>
<tr><td><code>r5</code></td><td>z</td><td>24-bit</td><td>Hardwired to zero. Readable in compares: <code>ceq r0, z</code></td></tr>
<tr><td><code>r6</code></td><td>iv</td><td>24-bit</td><td>Interrupt vector — address of interrupt service routine</td></tr>
<tr><td><code>r7</code></td><td>ir</td><td>24-bit</td><td>Interrupt return — saved PC when interrupt fires. Return with <code>jmp (ir)</code></td></tr>
</table>
<p>Only <strong>r0, r1, r2</strong> can be destinations for most ALU/load instructions.
<strong>fp</strong> can be pushed/popped and used as a memory base register.
<strong>sp</strong> is modified by push, pop, and <code>sub sp</code>.</p>

<h4>Load Constants</h4>
<table>
<tr><th>Instruction</th><th>Bytes</th><th>Description</th></tr>
<tr><td><code>lc ra, dd</code></td><td>2</td><td>Load signed 8-bit constant (-128..127). Sign-extends to 24 bits.</td></tr>
<tr><td><code>lcu ra, dd</code></td><td>2</td><td>Load unsigned 8-bit constant (0..255). Zero-extends to 24 bits.</td></tr>
<tr><td><code>la ra, addr</code></td><td>4</td><td>Load 24-bit address/constant. Any value 0..16777215.</td></tr>
</table>

<h4>Arithmetic</h4>
<table>
<tr><th>Instruction</th><th>Bytes</th><th>Description</th></tr>
<tr><td><code>add ra, rb</code></td><td>1</td><td>ra = ra + rb</td></tr>
<tr><td><code>add ra, dd</code></td><td>2</td><td>ra = ra + dd (signed 8-bit immediate)</td></tr>
<tr><td><code>sub ra, rb</code></td><td>1</td><td>ra = ra - rb</td></tr>
<tr><td><code>sub sp, addr</code></td><td>4</td><td>sp = sp - addr (24-bit; allocate stack space)</td></tr>
<tr><td><code>mul ra, rb</code></td><td>1</td><td>ra = ra * rb (24-bit result, overflow wraps)</td></tr>
</table>

<h4>Logic &amp; Shifts</h4>
<table>
<tr><th>Instruction</th><th>Bytes</th><th>Description</th></tr>
<tr><td><code>and ra, rb</code></td><td>1</td><td>ra = ra AND rb</td></tr>
<tr><td><code>or ra, rb</code></td><td>1</td><td>ra = ra OR rb</td></tr>
<tr><td><code>xor ra, rb</code></td><td>1</td><td>ra = ra XOR rb</td></tr>
<tr><td><code>shl ra, rb</code></td><td>1</td><td>ra = ra &lt;&lt; rb (shift left)</td></tr>
<tr><td><code>srl ra, rb</code></td><td>1</td><td>ra = ra &gt;&gt; rb (shift right, zero fill)</td></tr>
<tr><td><code>sra ra, rb</code></td><td>1</td><td>ra = ra &gt;&gt; rb (shift right, sign fill)</td></tr>
</table>

<h4>Compare (set C flag)</h4>
<table>
<tr><th>Instruction</th><th>Bytes</th><th>Description</th></tr>
<tr><td><code>ceq ra, rb</code></td><td>1</td><td>C = (ra == rb)</td></tr>
<tr><td><code>clu ra, rb</code></td><td>1</td><td>C = (ra &lt; rb) unsigned</td></tr>
<tr><td><code>cls ra, rb</code></td><td>1</td><td>C = (ra &lt; rb) signed</td></tr>
</table>
<p>Use <code>z</code> register for zero tests: <code>ceq r0, z</code> sets C if r0 == 0.</p>

<h4>Branch (PC-relative, signed 8-bit offset)</h4>
<table>
<tr><th>Instruction</th><th>Bytes</th><th>Description</th></tr>
<tr><td><code>bra label</code></td><td>2</td><td>Branch always</td></tr>
<tr><td><code>brt label</code></td><td>2</td><td>Branch if C = true</td></tr>
<tr><td><code>brf label</code></td><td>2</td><td>Branch if C = false</td></tr>
</table>

<h4>Memory Access (base + signed 8-bit offset)</h4>
<table>
<tr><th>Instruction</th><th>Bytes</th><th>Description</th></tr>
<tr><td><code>lb ra, dd(rb)</code></td><td>2</td><td>Load byte, sign-extend to 24 bits</td></tr>
<tr><td><code>lbu ra, dd(rb)</code></td><td>2</td><td>Load byte, zero-extend to 24 bits</td></tr>
<tr><td><code>lw ra, dd(rb)</code></td><td>2</td><td>Load word (3 bytes, little-endian)</td></tr>
<tr><td><code>sb ra, dd(rb)</code></td><td>2</td><td>Store byte (low 8 bits of ra)</td></tr>
<tr><td><code>sw ra, dd(rb)</code></td><td>2</td><td>Store word (3 bytes, little-endian)</td></tr>
</table>
<p>Valid base registers: r0, r1, r2, fp. (Not sp — use <code>mov fp, sp</code> then fp.)</p>

<h4>Stack</h4>
<table>
<tr><th>Instruction</th><th>Bytes</th><th>Description</th></tr>
<tr><td><code>push ra</code></td><td>1</td><td>sp -= 3; store ra at sp (word)</td></tr>
<tr><td><code>pop ra</code></td><td>1</td><td>Load ra from sp; sp += 3 (word)</td></tr>
</table>
<p>Can push/pop: r0, r1, r2, fp.</p>

<h4>Jump &amp; Call</h4>
<table>
<tr><th>Instruction</th><th>Bytes</th><th>Description</th></tr>
<tr><td><code>jmp (ra)</code></td><td>1</td><td>PC = ra (unconditional jump)</td></tr>
<tr><td><code>jal ra, (rb)</code></td><td>1</td><td>ra = return addr; PC = rb (jump and link)</td></tr>
</table>

<h4>Register Move</h4>
<table>
<tr><th>Instruction</th><th>Bytes</th><th>Description</th></tr>
<tr><td><code>mov ra, rb</code></td><td>1</td><td>ra = rb (copy register)</td></tr>
<tr><td><code>mov ra, c</code></td><td>1</td><td>ra = condition flag (0 or 1)</td></tr>
<tr><td><code>mov iv, ra</code></td><td>1</td><td>Set interrupt vector</td></tr>
<tr><td><code>mov fp, sp</code></td><td>1</td><td>Save stack pointer to frame pointer</td></tr>
<tr><td><code>mov sp, fp</code></td><td>1</td><td>Restore stack pointer from frame pointer</td></tr>
</table>

<h4>Extensions</h4>
<table>
<tr><th>Instruction</th><th>Bytes</th><th>Description</th></tr>
<tr><td><code>sxt ra</code></td><td>1</td><td>Sign-extend byte: bits 8..23 = bit 7</td></tr>
<tr><td><code>zxt ra</code></td><td>1</td><td>Zero-extend byte: bits 8..23 = 0</td></tr>
<tr><td><code>nop</code></td><td>1</td><td>No operation (0xFF)</td></tr>
</table>

<h4>Idioms</h4>
<table>
<tr><th>Pattern</th><th>Meaning</th></tr>
<tr><td><pre>halt:
        bra halt</pre></td><td>Halt (branch-to-self; emulator detects this)</td></tr>
<tr><td><pre>        la  r2, func
        jal r1, (r2)</pre></td><td>Call function (r1 = return address)</td></tr>
<tr><td><pre>        jmp (r1)</pre></td><td>Return from function</td></tr>
<tr><td><pre>        jmp (ir)</pre></td><td>Return from interrupt</td></tr>
</table>

<h4>Interrupts</h4>
<p>COR24 supports one interrupt source: UART RX data ready.</p>
<table>
<tr><th>Step</th><th>Action</th></tr>
<tr><td>Setup</td><td>Load ISR address into <code>iv</code>: <code>la r0, isr</code> then <code>mov iv, r0</code></td></tr>
<tr><td>Enable</td><td>Write 1 to interrupt enable register at 0xFF0010</td></tr>
<tr><td>Trigger</td><td>When UART receives a byte, CPU saves PC to <code>ir</code> and jumps to <code>iv</code></td></tr>
<tr><td>ISR body</td><td>Save registers (push), read UART data (acknowledges interrupt), process, restore (pop)</td></tr>
<tr><td>Return</td><td><code>jmp (ir)</code> — resumes execution at the interrupted instruction</td></tr>
</table>
<p>Interrupts do not nest — a second interrupt cannot fire while an ISR is running.
Reading the UART data register at 0xFF0100 acknowledges the interrupt.</p>

<h4>Memory Map</h4>
<table>
<tr><th>Address Range</th><th>Region</th><th>Notes</th></tr>
<tr><td><code>000000-0FFFFF</code></td><td>SRAM (1 MB)</td><td>Code at low addresses, data/globals above</td></tr>
<tr><td><code>FEE000-FEFFFF</code></td><td>EBR (8 KB range)</td><td>3 KB on MachXO FPGA; used for stack</td></tr>
<tr><td><code>FEEC00</code></td><td>Initial SP</td><td>Top of 3 KB EBR stack</td></tr>
<tr><td><code>FF0000</code></td><td>LED / Button</td><td>Write bit 0 = LED D2. Read bit 0 = button S2</td></tr>
<tr><td><code>FF0010</code></td><td>Interrupt enable</td><td>Write bit 0 = enable UART RX interrupt</td></tr>
<tr><td><code>FF0100</code></td><td>UART data</td><td>Write = TX. Read = RX (acknowledges interrupt)</td></tr>
<tr><td><code>FF0101</code></td><td>UART status</td><td>Bit 7 = TX busy. Bit 1 = RX data ready</td></tr>
</table>

<h4>Assembly Syntax</h4>
<pre>; Comments start with semicolon
label:                   ; labels on own line (as24 compatible)
        lc  r0, 42      ; instruction with operands
.local:                  ; local labels start with dot
        bra .local</pre>
<p>Numbers: decimal (<code>42</code>) or signed decimal for addresses (<code>la r1, -65536</code> = 0xFF0000).</p>
"#;

const HELP_CONTENT: &str = r#"
<h3>Help & Tips</h3>

<h4>How to Use:</h4>
<ol>
    <li><strong>Write Code</strong>: Enter your assembly program in the editor</li>
    <li><strong>Assemble</strong>: Click "Assemble" to parse and load your program</li>
    <li><strong>Step/Run</strong>: Use "Step" to execute one instruction or "Run" to complete</li>
    <li><strong>Reset</strong>: Click "Reset" to clear the CPU and start over</li>
</ol>

<h4>Assembly Syntax:</h4>
<ul>
    <li>Labels end with colon: <code>loop:</code></li>
    <li>Comments start with semicolon: <code>; comment</code></li>
    <li>Numbers: decimal (42), hex (0x2A)</li>
    <li>Register names: r0-r7, fp, sp, z, iv, ir</li>
</ul>

<h4>Calling Convention:</h4>
<pre>
; Function prologue
push    fp          ; Save frame pointer
push    r2          ; Save callee-saved
push    r1          ; Save return address
mov     fp,sp       ; Set up frame

; Function body...

; Function epilogue
mov     sp,fp       ; Restore stack
pop     r1          ; Restore r1
pop     r2          ; Restore r2
pop     fp          ; Restore fp
jmp     (r1)        ; Return
</pre>

<h4>Debugging Tips:</h4>
<ul>
    <li>Use "Step" to execute one instruction at a time</li>
    <li>Watch registers to see value changes (highlighted)</li>
    <li>Check the memory viewer to see your program</li>
    <li>The condition flag C is shown in the legend</li>
</ul>
"#;

/// Animated run loop: execute one instruction, update full state, yield to browser.
/// Shared by all three tabs (Assembler, C, Rust).
#[allow(clippy::too_many_arguments)]
fn run_one_instruction(
    mut current_cpu: WasmCpu,
    cpu_handle: yew::UseStateHandle<WasmCpu>,
    running_handle: yew::UseStateHandle<bool>,
    state_handle: yew::UseStateHandle<EmulatorState>,
    stop_handle: Rc<Cell<bool>>,
    switch_handle: Rc<Cell<u8>>,
    uart_handle: Rc<RefCell<VecDeque<u8>>>,
    uart_clear_handle: Rc<Cell<bool>>,
    speed_handle: Rc<Cell<u32>>,
    led_on_count: u64,
) {
    if stop_handle.get() {
        let mut s = capture_cpu_state(&current_cpu, &state_handle);
        s.led_on_count = led_on_count;
        s.led_duty_cycle = if s.instruction_count > 0 {
            led_on_count as f32 / s.instruction_count as f32
        } else { 0.0 };
        state_handle.set(s);
        cpu_handle.set(current_cpu);
        running_handle.set(false);
        return;
    }
    if uart_clear_handle.get() {
        uart_clear_handle.set(false);
        current_cpu.clear_uart_output();
    }
    current_cpu.set_switches(switch_handle.get());
    {
        let mut q = uart_handle.borrow_mut();
        while let Some(byte) = q.pop_front() {
            current_cpu.uart_send_char(byte as char);
        }
    }

    let delay = speed_handle.get().clamp(1, 100);
    let batch = if delay >= 50 {
        1u32
    } else if delay >= 10 {
        ((51 - delay) / 5).max(1)
    } else {
        let d = 11 - delay;
        (d * d).max(1)
    };

    let mut halted = false;
    let mut batch_led_on = 0u64;
    for _ in 0..batch {
        if current_cpu.is_halted() {
            halted = true;
            break;
        }
        if current_cpu.step().is_err() {
            halted = true;
            break;
        }
        if current_cpu.is_halted() {
            halted = true;
            break;
        }
        if current_cpu.get_led_value() & 1 == 1 {
            batch_led_on += 1;
        }
    }

    let total_on = led_on_count + batch_led_on;
    let mut s = capture_cpu_state(&current_cpu, &state_handle);
    s.led_on_count = total_on;
    s.led_duty_cycle = if s.instruction_count > 0 {
        total_on as f32 / s.instruction_count as f32
    } else { 0.0 };
    state_handle.set(s);
    cpu_handle.set(current_cpu.clone());

    if halted {
        running_handle.set(false);
    } else {
        let yield_delay = delay.max(1);
        gloo::timers::callback::Timeout::new(yield_delay, move || {
            run_one_instruction(current_cpu, cpu_handle, running_handle, state_handle, stop_handle, switch_handle, uart_handle, uart_clear_handle, speed_handle, total_on);
        }).forget();
    }
}

/// Create a step callback for any tab
fn make_step_callback(
    cpu: yew::UseStateHandle<WasmCpu>,
    emu_state: yew::UseStateHandle<EmulatorState>,
) -> Callback<u32> {
    Callback::from(move |count: u32| {
        let mut new_cpu = (*cpu).clone();
        for _ in 0..count {
            if new_cpu.is_halted() { break; }
            if new_cpu.step().is_err() { break; }
        }
        emu_state.set(capture_cpu_state(&new_cpu, &emu_state));
        cpu.set(new_cpu);
    })
}

/// Create a run callback for any tab
#[allow(clippy::too_many_arguments)]
fn make_run_callback(
    cpu: yew::UseStateHandle<WasmCpu>,
    is_running: yew::UseStateHandle<bool>,
    emu_state: yew::UseStateHandle<EmulatorState>,
    stop_flag: Rc<Cell<bool>>,
    switches: Rc<Cell<u8>>,
    uart_queue: Rc<RefCell<VecDeque<u8>>>,
    uart_clear: Rc<Cell<bool>>,
    speed: Rc<Cell<u32>>,
) -> Callback<()> {
    Callback::from(move |()| {
        is_running.set(true);
        stop_flag.set(false);
        uart_clear.set(false);
        switches.set((*cpu).get_switches());

        let cpu_handle = cpu.clone();
        let running_handle = is_running.clone();
        let state_handle = emu_state.clone();
        let current_cpu = (*cpu).clone();
        let stop_handle = stop_flag.clone();
        let switch_handle = switches.clone();
        let uart_handle = uart_queue.clone();
        let uart_clear_handle = uart_clear.clone();
        let speed_handle = speed.clone();

        gloo::timers::callback::Timeout::new(0, move || {
            run_one_instruction(current_cpu, cpu_handle, running_handle, state_handle,
                stop_handle, switch_handle, uart_handle, uart_clear_handle, speed_handle, 0);
        }).forget();
    })
}

/// Create a switch toggle callback for any tab
fn make_switch_callback(
    switch_value: yew::UseStateHandle<u8>,
    cpu: yew::UseStateHandle<WasmCpu>,
    shared_switches: Rc<Cell<u8>>,
) -> Callback<u8> {
    Callback::from(move |new_value: u8| {
        switch_value.set(new_value);
        shared_switches.set(new_value);
        let mut new_cpu = (*cpu).clone();
        new_cpu.set_switches(new_value);
        cpu.set(new_cpu);
    })
}

/// Create a UART send callback for any tab
fn make_uart_send_callback(
    cpu: yew::UseStateHandle<WasmCpu>,
    emu_state: yew::UseStateHandle<EmulatorState>,
    is_running: yew::UseStateHandle<bool>,
    uart_queue: Rc<RefCell<VecDeque<u8>>>,
) -> Callback<u8> {
    Callback::from(move |byte: u8| {
        if *is_running {
            uart_queue.borrow_mut().push_back(byte);
        } else {
            let mut new_cpu = (*cpu).clone();
            new_cpu.uart_send_char(byte as char);
            let _ = new_cpu.run();
            emu_state.set(capture_cpu_state(&new_cpu, &emu_state));
            cpu.set(new_cpu);
        }
    })
}

/// Create a UART clear callback for any tab
fn make_uart_clear_callback(
    cpu: yew::UseStateHandle<WasmCpu>,
    emu_state: yew::UseStateHandle<EmulatorState>,
    is_running: yew::UseStateHandle<bool>,
    uart_clear_flag: Rc<Cell<bool>>,
) -> Callback<()> {
    Callback::from(move |()| {
        if *is_running {
            uart_clear_flag.set(true);
        } else {
            let mut new_cpu = (*cpu).clone();
            new_cpu.clear_uart_output();
            emu_state.set(capture_cpu_state(&new_cpu, &emu_state));
            cpu.set(new_cpu);
        }
    })
}

/// Capture current CPU state into an EmulatorState, preserving previous state for heatmap
fn capture_cpu_state(cpu: &WasmCpu, prev: &EmulatorState) -> EmulatorState {
    let regs = cpu.get_registers();
    let mut registers = [0u32; 8];
    for (i, &val) in regs.iter().enumerate().take(8) {
        registers[i] = val;
    }

    let memory_low = cpu.get_sparse_sram();
    let mut memory_io_led = Vec::with_capacity(16);
    for addr in 0xFF0000..0xFF0010 {
        memory_io_led.push(cpu.read_byte(addr));
    }
    let mut memory_io_uart = Vec::with_capacity(16);
    for addr in 0xFF0100..0xFF0110 {
        memory_io_uart.push(cpu.read_byte(addr));
    }
    let memory_stack = cpu.get_sparse_ebr();

    EmulatorState {
        registers,
        prev_registers: prev.registers,
        prev_prev_registers: prev.prev_registers,
        pc: cpu.get_pc(),
        condition_flag: cpu.get_condition_flag(),
        is_halted: cpu.is_halted(),
        led_value: cpu.get_led_value(),
        led_on_count: prev.led_on_count + if (cpu.get_led_value() & 1) == 1 { 1 } else { 0 },
        led_duty_cycle: {
            let on = prev.led_on_count + if (cpu.get_led_value() & 1) == 1 { 1 } else { 0 };
            let total = cpu.get_instruction_count() as u64;
            if total > 0 { on as f32 / total as f32 } else { 0.0 }
        },
        instruction_count: cpu.get_instruction_count(),
        memory_low: memory_low.clone(),
        memory_io_led: memory_io_led.clone(),
        memory_io_uart: memory_io_uart.clone(),
        memory_stack: memory_stack.clone(),
        program_end: cpu.get_program_end(),
        prev_memory_low: prev.memory_low.clone(),
        prev_memory_io_led: prev.memory_io_led.clone(),
        prev_memory_io_uart: prev.memory_io_uart.clone(),
        prev_memory_stack: prev.memory_stack.clone(),
        prev_prev_memory_low: prev.prev_memory_low.clone(),
        prev_prev_memory_io_led: prev.prev_memory_io_led.clone(),
        prev_prev_memory_io_uart: prev.prev_memory_io_uart.clone(),
        prev_prev_memory_stack: prev.prev_memory_stack.clone(),
        current_instruction: cpu.get_current_instruction(),
        assembled_lines: cpu.get_assembled_lines(),
        uart_output: cpu.get_uart_output(),
        trace_lines: cpu.get_trace_lines(100),
    }
}

/// Capture initial CPU state (no previous state for heatmap)
fn capture_cpu_state_initial(cpu: &WasmCpu) -> EmulatorState {
    let empty = EmulatorState::default();
    let mut state = capture_cpu_state(cpu, &empty);
    // On initial load, prev = current (no changes highlighted)
    state.prev_registers = state.registers;
    state.prev_prev_registers = state.registers;
    state.prev_memory_low = state.memory_low.clone();
    state.prev_memory_io_led = state.memory_io_led.clone();
    state.prev_memory_io_uart = state.memory_io_uart.clone();
    state.prev_memory_stack = state.memory_stack.clone();
    state.prev_prev_memory_low = state.memory_low.clone();
    state.prev_prev_memory_io_led = state.memory_io_led.clone();
    state.prev_prev_memory_io_uart = state.memory_io_uart.clone();
    state.prev_prev_memory_stack = state.memory_stack.clone();
    state
}

