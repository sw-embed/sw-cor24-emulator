//! Yew application for COR24 Assembly Emulator

use std::cell::Cell;
use std::rc::Rc;

use components::{
    DebugPanel, ExampleItem, ExamplePicker, Header, Modal, ProgramArea,
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
    let rust_switch_value = use_state(|| 0u8);
    // Use Rc<Cell> for immediate stop flag visibility in Rust pipeline
    let rust_stop_requested = use_mut_ref(|| Rc::new(Cell::new(false)));
    // Use Rc<Cell> for switch state during Rust run - avoids race with cpu_handle updates
    let rust_shared_switches = use_mut_ref(|| Rc::new(Cell::new(0u8)));

    // State management
    let cpu = use_state(WasmCpu::new);
    let program_code = use_state(|| String::from(EXAMPLE_PROGRAM));
    let assembly_output = use_state(|| None::<Html>);
    let assembly_lines = use_state(Vec::<String>::new);
    let asm_emu_state = use_state(EmulatorState::default);
    let asm_switch_value = use_state(|| 0u8);
    let challenge_mode = use_state(|| false);
    let current_challenge_id = use_state(|| None::<usize>);
    let challenge_result = use_state(|| None::<Result<String, String>>);

    // Track whether assembly succeeded (enables Step/Run)
    let asm_assembled = use_state(|| false);

    // Animated run state for assembler tab
    let asm_is_running = use_state(|| false);
    // Use Rc<Cell> for stop flag - provides immediate visibility across closures
    let asm_stop_requested = use_mut_ref(|| Rc::new(Cell::new(false)));
    // Use Rc<Cell> for switch state during run - avoids race with cpu_handle updates
    let shared_switches = use_mut_ref(|| Rc::new(Cell::new(0u8)));

    // Modal states
    let tutorial_open = use_state(|| false);
    let examples_open = use_state(|| false);
    let rust_examples_open = use_state(|| false);
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
            emoji: "📚".to_string(),
            label: "Tutorial".to_string(),
            onclick: {
                let tutorial_open = tutorial_open.clone();
                Callback::from(move |_| tutorial_open.set(true))
            },
            title: Some("Learn COR24 basics".to_string()),
        },
        SidebarButton {
            emoji: "📝".to_string(),
            label: "Examples".to_string(),
            onclick: {
                let examples_open = examples_open.clone();
                Callback::from(move |_| examples_open.set(true))
            },
            title: Some("Load example programs".to_string()),
        },
        SidebarButton {
            emoji: "🎯".to_string(),
            label: "Challenges".to_string(),
            onclick: {
                let challenges_open = challenges_open.clone();
                Callback::from(move |_| challenges_open.set(true))
            },
            title: Some("Test your skills".to_string()),
        },
        SidebarButton {
            emoji: "📖".to_string(),
            label: "ISA Ref".to_string(),
            onclick: {
                let isa_ref_open = isa_ref_open.clone();
                Callback::from(move |_| isa_ref_open.set(true))
            },
            title: Some("Instruction reference".to_string()),
        },
        SidebarButton {
            emoji: "❓".to_string(),
            label: "Help".to_string(),
            onclick: {
                let help_open = help_open.clone();
                Callback::from(move |_| help_open.set(true))
            },
            title: Some("Usage help".to_string()),
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

    let on_run = {
        let cpu = cpu.clone();
        let assembly_output = assembly_output.clone();
        let asm_is_running = asm_is_running.clone();
        let asm_emu_state = asm_emu_state.clone();
        let stop_flag = asm_stop_requested.borrow().clone();
        let switches = shared_switches.borrow().clone();

        Callback::from(move |()| {
            // Start animated run
            asm_is_running.set(true);
            stop_flag.set(false);

            // Initialize shared switch state from current CPU
            switches.set((*cpu).get_switches());

            let cpu_handle = cpu.clone();
            let output_handle = assembly_output.clone();
            let running_handle = asm_is_running.clone();
            let state_handle = asm_emu_state.clone();
            let stop_handle = stop_flag.clone();
            let switch_handle = switches.clone();
            let current_cpu = (*cpu).clone();

            // Start the animated run loop
            #[allow(clippy::too_many_arguments)]
            fn run_step(
                mut current_cpu: WasmCpu,
                cpu_handle: yew::UseStateHandle<WasmCpu>,
                output_handle: yew::UseStateHandle<Option<Html>>,
                running_handle: yew::UseStateHandle<bool>,
                state_handle: yew::UseStateHandle<EmulatorState>,
                stop_handle: Rc<Cell<bool>>,
                switch_handle: Rc<Cell<u8>>,
                cumulative_led_on: u64,
            ) {
                // Check if stop was requested (immediate - no state delay)
                if stop_handle.get() {
                    state_handle.set(capture_cpu_state(&current_cpu, &state_handle));
                    cpu_handle.set(current_cpu);
                    running_handle.set(false);
                    output_handle.set(Some(yew::html! {
                        <div class="info-text">
                            {"⏹ Execution stopped"}
                        </div>
                    }));
                    return;
                }

                // Read switch state from shared Rc<Cell> (updated by switch onclick)
                current_cpu.set_switches(switch_handle.get());

                // Execute a batch of instructions per animation frame
                let mut halted = false;
                let mut error_msg = None;
                let mut batch_led_on: u64 = 0;
                for _ in 0..500 {
                    if current_cpu.is_halted() {
                        halted = true;
                        break;
                    }
                    if let Err(e) = current_cpu.step() {
                        error_msg = Some(format!("{:?}", e));
                        halted = true;
                        break;
                    }
                    if current_cpu.get_led_value() & 1 == 1 {
                        batch_led_on += 1;
                    }
                }

                // Update CPU state for UI refresh (includes LED state)
                current_cpu.set_switches(switch_handle.get());
                let mut state = capture_cpu_state(&current_cpu, &state_handle);
                let total_on = cumulative_led_on + batch_led_on;
                state.led_on_count = total_on;
                let total_instr = state.instruction_count as u64;
                state.led_duty_cycle = if total_instr > 0 { total_on as f32 / total_instr as f32 } else { 0.0 };
                state_handle.set(state);
                cpu_handle.set(current_cpu.clone());

                if halted {
                    running_handle.set(false);
                    if let Some(err) = error_msg {
                        output_handle.set(Some(yew::html! {
                            <div class="error-text">
                                {format!("Error: {}", err)}
                            </div>
                        }));
                    } else {
                        output_handle.set(Some(yew::html! {
                            <div class="success-text">
                                {"✓ Program completed"}
                            </div>
                        }));
                    }
                } else {
                    // Continue running - 50ms delay allows browser to process input events
                    gloo::timers::callback::Timeout::new(50, move || {
                        run_step(current_cpu, cpu_handle, output_handle, running_handle, state_handle, stop_handle, switch_handle, total_on);
                    }).forget();
                }
            }

            // Start the first step
            gloo::timers::callback::Timeout::new(0, move || {
                run_step(current_cpu, cpu_handle, output_handle, running_handle, state_handle, stop_handle, switch_handle, 0);
            }).forget();
        })
    };

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

        Callback::from(move |example: RustExample| {
            rust_load_gen.set(*rust_load_gen + 1);
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
                // Capture low memory (0x000000-0x00007F) - 128 bytes
                let mut memory_low = Vec::new();
                for addr in 0x000000..0x000080 {
                    memory_low.push(new_cpu.read_byte(addr));
                }
                // I/O regions: LED/Switch (0xFF0000, 32 bytes) and UART (0xFF0100, 16 bytes)
                let mut memory_io_led = Vec::with_capacity(32);
                for addr in 0xFF0000..0xFF0020 {
                    memory_io_led.push(new_cpu.read_byte(addr));
                }
                let mut memory_io_uart = Vec::with_capacity(16);
                for addr in 0xFF0100..0xFF0110 {
                    memory_io_uart.push(new_cpu.read_byte(addr));
                }
                // Capture stack region (64 bytes around SP)
                let sp = new_cpu.read_register(4);
                // Stack: show from 16-byte-aligned SP up to 0xFEEC00, minimum 16 bytes
                let stack_top: u32 = 0xFEEC00;
                let aligned_start = (sp.min(stack_top - 16)) & !0xF;
                let stack_size = (stack_top - aligned_start).min(256);
                let memory_stack = new_cpu.get_memory_slice(aligned_start, stack_size);
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
                    stack_base_addr: aligned_start,
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
                });

                rust_cpu.set(new_cpu);
                rust_is_loaded.set(true);
                rust_loaded_example.set(Some(example));
            }
        })
    };

    // Rust pipeline: Step N instructions
    let on_rust_step = {
        let rust_cpu = rust_cpu.clone();
        let rust_emu_state = rust_emu_state.clone();

        Callback::from(move |count: u32| {
            let mut new_cpu = (*rust_cpu).clone();
            let prev_state = (*rust_emu_state).clone();

            // Execute up to `count` steps, stopping early on halt/error
            for _ in 0..count {
                if new_cpu.is_halted() {
                    break;
                }
                if new_cpu.step().is_err() {
                    break;
                }
            }

            {
                let regs = new_cpu.get_registers();
                let mut registers = [0u32; 8];
                for (i, &val) in regs.iter().enumerate().take(8) {
                    registers[i] = val;
                }
                // Capture low memory (0x000000-0x00007F) - 128 bytes
                let mut memory_low = Vec::new();
                for addr in 0x000000..0x000080 {
                    memory_low.push(new_cpu.read_byte(addr));
                }
                // I/O regions: LED/Switch (0xFF0000, 32 bytes) and UART (0xFF0100, 16 bytes)
                let mut memory_io_led = Vec::with_capacity(32);
                for addr in 0xFF0000..0xFF0020 {
                    memory_io_led.push(new_cpu.read_byte(addr));
                }
                let mut memory_io_uart = Vec::with_capacity(16);
                for addr in 0xFF0100..0xFF0110 {
                    memory_io_uart.push(new_cpu.read_byte(addr));
                }
                // Capture stack region (64 bytes around SP)
                let sp = new_cpu.read_register(4);
                // Stack: show from 16-byte-aligned SP up to 0xFEEC00, minimum 16 bytes
                let stack_top: u32 = 0xFEEC00;
                let aligned_start = (sp.min(stack_top - 16)) & !0xF;
                let stack_size = (stack_top - aligned_start).min(256);
                let memory_stack = new_cpu.get_memory_slice(aligned_start, stack_size);

                rust_emu_state.set(EmulatorState {
                    registers,
                    prev_registers: prev_state.registers,
                    prev_prev_registers: prev_state.prev_registers,
                    pc: new_cpu.get_pc(),
                    condition_flag: new_cpu.get_condition_flag(),
                    is_halted: new_cpu.is_halted(),
                    led_value: new_cpu.get_led_value(),
                    led_duty_cycle: if (new_cpu.get_led_value() & 1) == 1 { 1.0 } else { 0.0 },
                    led_on_count: 0,
                    instruction_count: new_cpu.get_instruction_count(),
                    memory_low,
                    memory_io_led,
                    memory_io_uart,
                    memory_stack,
                    stack_base_addr: aligned_start,
                    program_end: new_cpu.get_program_end(),
                    prev_memory_low: prev_state.memory_low,
                    prev_memory_io_led: prev_state.memory_io_led,
                    prev_memory_io_uart: prev_state.memory_io_uart,
                    prev_memory_stack: prev_state.memory_stack,
                    prev_prev_memory_low: prev_state.prev_memory_low,
                    prev_prev_memory_io_led: prev_state.prev_memory_io_led,
                    prev_prev_memory_io_uart: prev_state.prev_memory_io_uart,
                    prev_prev_memory_stack: prev_state.prev_memory_stack,
                    current_instruction: new_cpu.get_current_instruction(),
                    assembled_lines: prev_state.assembled_lines,
                    uart_output: new_cpu.get_uart_output(),
                });
                rust_cpu.set(new_cpu);
            }
        })
    };

    // Rust pipeline: Run with stop button and switch support
    let on_rust_run = {
        let rust_cpu = rust_cpu.clone();
        let rust_is_running = rust_is_running.clone();
        let rust_emu_state = rust_emu_state.clone();
        let stop_flag = rust_stop_requested.clone();
        let switch_state = rust_shared_switches.clone();
        let switch_value = rust_switch_value.clone();

        Callback::from(move |()| {
            // Clear stop flag and sync switch state
            stop_flag.borrow().set(false);
            switch_state.borrow().set(*switch_value);

            rust_is_running.set(true);
            let cpu_handle = rust_cpu.clone();
            let running = rust_is_running.clone();
            let state = rust_emu_state.clone();
            let asm_lines = state.assembled_lines.clone();
            let initial_cpu = (*rust_cpu).clone();
            let prev_regs = state.registers;
            let prev_prev_regs = state.prev_registers;
            let prev_mem_low = state.memory_low.clone();
            let prev_mem_io_led = state.memory_io_led.clone();
            let prev_mem_io_uart = state.memory_io_uart.clone();
            let prev_mem_stack = state.memory_stack.clone();
            let prev_prev_mem_low = state.prev_memory_low.clone();
            let prev_prev_mem_io_led = state.prev_memory_io_led.clone();
            let prev_prev_mem_io_uart = state.prev_memory_io_uart.clone();
            let prev_prev_mem_stack = state.prev_memory_stack.clone();
            let stop_flag = Rc::clone(&stop_flag.borrow());
            let switch_state = Rc::clone(&switch_state.borrow());

            // Run with animation using timer
            gloo::timers::callback::Timeout::new(50, move || {
                #[allow(clippy::too_many_arguments, clippy::only_used_in_recursion)]
                fn run_step(
                    mut current_cpu: WasmCpu,
                    cpu_handle: yew::UseStateHandle<WasmCpu>,
                    running: yew::UseStateHandle<bool>,
                    state: yew::UseStateHandle<EmulatorState>,
                    asm_lines: Vec<String>,
                    prev_regs: [u32; 8],
                    prev_prev_regs: [u32; 8],
                    prev_mem_low: Vec<u8>,
                    prev_mem_io_led: Vec<u8>,
                    prev_mem_io_uart: Vec<u8>,
                    prev_mem_stack: Vec<u8>,
                    prev_prev_mem_low: Vec<u8>,
                    prev_prev_mem_io_led: Vec<u8>,
                    prev_prev_mem_io_uart: Vec<u8>,
                    prev_prev_mem_stack: Vec<u8>,
                    steps: u32,
                    stop_flag: Rc<Cell<bool>>,
                    switch_state: Rc<Cell<u8>>,
                    cumulative_led_on: u64,
                ) {
                    // Check stop flag
                    if stop_flag.get() {
                        cpu_handle.set(current_cpu);
                        running.set(false);
                        return;
                    }

                    // Sync switch state before execution
                    current_cpu.set_switches(switch_state.get());

                    // Execute a batch of instructions
                    let mut halted = false;
                    let mut batch_led_on: u64 = 0;
                    for _ in 0..500 {
                        if current_cpu.is_halted() {
                            halted = true;
                            break;
                        }
                        if current_cpu.step().is_err() {
                            halted = true;
                            break;
                        }
                        if current_cpu.get_led_value() & 1 == 1 {
                            batch_led_on += 1;
                        }
                    }

                    // Update state for display
                    let regs = current_cpu.get_registers();
                    let mut registers = [0u32; 8];
                    for (i, &val) in regs.iter().enumerate().take(8) {
                        registers[i] = val;
                    }
                    // Capture low memory (0x000000-0x00007F) - 128 bytes
                    let mut memory_low = Vec::new();
                    for addr in 0x000000..0x000080 {
                        memory_low.push(current_cpu.read_byte(addr));
                    }
                    // I/O regions: LED/Switch (0xFF0000, 32 bytes) and UART (0xFF0100, 16 bytes)
                    let mut memory_io_led = Vec::with_capacity(32);
                    for addr in 0xFF0000..0xFF0020 {
                        memory_io_led.push(current_cpu.read_byte(addr));
                    }
                    let mut memory_io_uart = Vec::with_capacity(16);
                    for addr in 0xFF0100..0xFF0110 {
                        memory_io_uart.push(current_cpu.read_byte(addr));
                    }
                    // Capture stack region (64 bytes around SP)
                    let sp = current_cpu.read_register(4);
                    // Stack: show from 16-byte-aligned SP up to 0xFEEC00, minimum 16 bytes
                    let stack_top: u32 = 0xFEEC00;
                    let aligned_start = (sp.min(stack_top - 16)) & !0xF;
                    let stack_size = (stack_top - aligned_start).min(256);
                    let memory_stack = current_cpu.get_memory_slice(aligned_start, stack_size);

                    // Save current values as prev for next iteration
                    let next_prev_regs = registers;
                    let next_prev_prev_regs = prev_regs;
                    let next_prev_mem_low = memory_low.clone();
                    let next_prev_mem_io_led = memory_io_led.clone();
                    let next_prev_mem_io_uart = memory_io_uart.clone();
                    let next_prev_mem_stack = memory_stack.clone();
                    let next_prev_prev_mem_low = prev_mem_low.clone();
                    let next_prev_prev_mem_io_led = prev_mem_io_led.clone();
                    let next_prev_prev_mem_io_uart = prev_mem_io_uart.clone();
                    let next_prev_prev_mem_stack = prev_mem_stack.clone();

                    let next_led_on = cumulative_led_on + batch_led_on;
                    state.set(EmulatorState {
                        registers,
                        prev_registers: prev_regs,
                        prev_prev_registers: prev_prev_regs,
                        pc: current_cpu.get_pc(),
                        condition_flag: current_cpu.get_condition_flag(),
                        is_halted: current_cpu.is_halted(),
                        led_value: current_cpu.get_led_value(),
                        led_duty_cycle: {
                            let total_instr = current_cpu.get_instruction_count() as u64;
                            if total_instr > 0 { next_led_on as f32 / total_instr as f32 } else { 0.0 }
                        },
                        led_on_count: next_led_on,
                        instruction_count: current_cpu.get_instruction_count(),
                        memory_low,
                        memory_io_led,
                        memory_io_uart,
                        memory_stack,
                        stack_base_addr: aligned_start,
                        program_end: current_cpu.get_program_end(),
                        prev_memory_low: prev_mem_low,
                        prev_memory_io_led: prev_mem_io_led,
                        prev_memory_io_uart: prev_mem_io_uart,
                        prev_memory_stack: prev_mem_stack,
                        prev_prev_memory_low: prev_prev_mem_low,
                        prev_prev_memory_io_led: prev_prev_mem_io_led,
                        prev_prev_memory_io_uart: prev_prev_mem_io_uart,
                        prev_prev_memory_stack: prev_prev_mem_stack,
                        current_instruction: current_cpu.get_current_instruction(),
                        assembled_lines: asm_lines.clone(),
                        uart_output: current_cpu.get_uart_output(),
                    });

                    if halted {
                        // Done - save final CPU state
                        cpu_handle.set(current_cpu);
                        running.set(false);
                    } else {
                        // Continue running - pass CPU value directly to next iteration
                        let cpu_handle = cpu_handle.clone();
                        let running = running.clone();
                        let state = state.clone();
                        let asm_lines = asm_lines.clone();
                        gloo::timers::callback::Timeout::new(30, move || {
                            run_step(current_cpu, cpu_handle, running, state, asm_lines, next_prev_regs, next_prev_prev_regs, next_prev_mem_low, next_prev_mem_io_led, next_prev_mem_io_uart, next_prev_mem_stack, next_prev_prev_mem_low, next_prev_prev_mem_io_led, next_prev_prev_mem_io_uart, next_prev_prev_mem_stack, steps + 500, stop_flag, switch_state, next_led_on);
                        }).forget();
                    }
                }

                run_step(initial_cpu, cpu_handle, running, state, asm_lines, prev_regs, prev_prev_regs, prev_mem_low, prev_mem_io_led, prev_mem_io_uart, prev_mem_stack, prev_prev_mem_low, prev_prev_mem_io_led, prev_prev_mem_io_uart, prev_prev_mem_stack, 0, stop_flag, switch_state, 0);
            }).forget();
        })
    };

    // Rust pipeline: Stop execution
    let on_rust_stop = {
        let stop_flag = rust_stop_requested.clone();
        Callback::from(move |()| {
            stop_flag.borrow().set(true);
        })
    };

    // Rust pipeline: Toggle switch
    let on_rust_switch_toggle = {
        let rust_switch_value = rust_switch_value.clone();
        let rust_cpu = rust_cpu.clone();
        let switch_state = rust_shared_switches.clone();
        Callback::from(move |new_value: u8| {
            rust_switch_value.set(new_value);
            // Update shared state for run loop
            switch_state.borrow().set(new_value);
            // Also update CPU directly for step mode
            let mut cpu = (*rust_cpu).clone();
            cpu.set_switches(new_value);
            rust_cpu.set(cpu);
        })
    };

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
                    // Capture low memory (0x000000-0x00007F) - 128 bytes
                    let mut memory_low = Vec::new();
                    for addr in 0x000000..0x000080 {
                        memory_low.push(new_cpu.read_byte(addr));
                    }
                    // I/O regions: LED/Switch (0xFF0000, 32 bytes) and UART (0xFF0100, 16 bytes)
                    let mut memory_io_led = Vec::with_capacity(32);
                    for addr in 0xFF0000..0xFF0020 {
                        memory_io_led.push(new_cpu.read_byte(addr));
                    }
                    let mut memory_io_uart = Vec::with_capacity(16);
                    for addr in 0xFF0100..0xFF0110 {
                        memory_io_uart.push(new_cpu.read_byte(addr));
                    }
                    // Capture stack region (top 16+ bytes, 16-byte aligned)
                    let sp = new_cpu.read_register(4);
                    let stack_top: u32 = 0xFEEC00;
                    let aligned_start = (sp.min(stack_top - 16)) & !0xF;
                    let stack_size = (stack_top - aligned_start).min(256);
                    let memory_stack = new_cpu.get_memory_slice(aligned_start, stack_size);
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
                        stack_base_addr: aligned_start,
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
        Tab { id: "rust".to_string(), label: "Rust".to_string(), tooltip: Some("Rust → MSP430 → COR24 compilation pipeline".to_string()) },
    ];

    // Get examples for the modal
    let examples = get_examples();

    // Pre-built Rust examples
    let rust_examples = get_rust_examples();

    // Assembler switch toggle callback
    let on_asm_switch_toggle = {
        let asm_switch_value = asm_switch_value.clone();
        let cpu = cpu.clone();
        let switch_state = shared_switches.clone();
        Callback::from(move |new_value: u8| {
            asm_switch_value.set(new_value);
            switch_state.borrow().set(new_value);
            let mut new_cpu = (*cpu).clone();
            new_cpu.set_switches(new_value);
            cpu.set(new_cpu);
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
                    step_enabled={*asm_assembled && !(*cpu).is_halted()}
                    run_enabled={*asm_assembled && !(*cpu).is_halted()}
                    show_exec_buttons={false}
                />

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
                        listing_scroll_id={"asm-debug-listing-scroll".to_string()}
                        show_listing={false}
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
                    Callback::from(move |idx: usize| {
                        if let Some((_, _, code)) = examples.get(idx) {
                            cpu.set(WasmCpu::new());
                            assembly_output.set(None);
                            assembly_lines.set(Vec::new());
                            challenge_mode.set(false);
                            current_challenge_id.set(None);
                            challenge_result.set(None);
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

            <Modal id="challenges" title="Challenges" active={*challenges_open} on_close={close_challenges}>
                {render_challenges_list(challenge_mode.clone(), current_challenge_id.clone(), program_code.clone(), challenges_open.clone())}
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
            </footer>
        </div>
    }
}

// Helper function to render challenges list
fn render_challenges_list(
    challenge_mode: UseStateHandle<bool>,
    current_challenge_id: UseStateHandle<Option<usize>>,
    program_code: UseStateHandle<String>,
    challenges_open: UseStateHandle<bool>,
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
                                Callback::from(move |_| {
                                    challenge_mode.set(true);
                                    current_challenge_id.set(Some(id));
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
<h3>MakerLisp COR24 Assembly Emulator</h3>
<p>This emulator teaches you assembly programming using the
<a href="https://makerlisp.com" target="_blank">MakerLisp</a> COR24
C-Oriented RISC architecture — a 24-bit soft CPU targeting Lattice MachXO FPGAs.</p>

<h4>CPU Features:</h4>
<ul>
    <li><strong>3 GP Registers (24-bit)</strong>: r0, r1, r2</li>
    <li><strong>5 Special Registers</strong>: fp (r3), sp (r4), z (r5), iv (r6), ir (r7)</li>
    <li><strong>32 Operations</strong>: Encoded into 211 instruction forms (1, 2, or 4 bytes)</li>
    <li><strong>16 MB Address Space</strong>: 1 MB SRAM, 3 KB EBR (stack), memory-mapped I/O</li>
    <li><strong>Single Condition Flag (C)</strong>: Set by compare instructions</li>
</ul>

<h4>Registers:</h4>
<ul>
    <li><code>r0, r1, r2</code> - General purpose (24-bit)</li>
    <li><code>fp (r3)</code> - Frame Pointer</li>
    <li><code>sp (r4)</code> - Stack Pointer (init: 0xFEEC00)</li>
    <li><code>z (r5)</code> - Always zero; only usable in compare instructions (e.g. <code>ceq r0,z</code>)</li>
    <li><code>iv (r6)</code> - Interrupt Vector</li>
    <li><code>ir (r7)</code> - Interrupt Return address</li>
</ul>

<h4>Basic Instructions:</h4>
<ul>
    <li><code>lc ra,dd</code> - Load constant (signed 8-bit)</li>
    <li><code>la ra,addr</code> - Load address (24-bit)</li>
    <li><code>add ra,rb</code> - Add registers</li>
    <li><code>add ra,dd</code> - Add immediate</li>
    <li><code>sub ra,rb</code> - Subtract registers</li>
    <li><code>cls ra,rb</code> - Compare less (signed), set C</li>
    <li><code>brt dd</code> - Branch if C=true</li>
    <li><code>brf dd</code> - Branch if C=false</li>
    <li><code>push ra</code> - Push to stack</li>
    <li><code>pop ra</code> - Pop from stack</li>
    <li><code>halt: bra halt</code> - Stop (branch-to-self loop)</li>
</ul>
"#;

const ISA_REF_CONTENT: &str = r#"
<h3>COR24 Instruction Set Reference</h3>
<p><em>32 operations &times; register fields = 211 instruction forms.
See <a href="https://makerlisp.com" target="_blank">makerlisp.com</a> for the hardware specification.</em></p>

<h4>Load Instructions</h4>
<p><strong>lc ra,dd</strong> - Load Constant (signed 8-bit)</p>
<p>Example: <code>lc r0,42</code> loads 42 into r0</p>

<p><strong>lcu ra,dd</strong> - Load Constant Unsigned</p>
<p>Example: <code>lcu r0,255</code> loads 255 into r0</p>

<p><strong>la ra,addr</strong> - Load 24-bit Address</p>
<p>Example: <code>la r0,0x1000</code> loads address into r0</p>

<h4>Arithmetic Instructions</h4>
<p><strong>add ra,rb</strong> - Add registers: ra = ra + rb</p>
<p><strong>add ra,dd</strong> - Add immediate: ra = ra + dd</p>
<p><strong>sub ra,rb</strong> - Subtract: ra = ra - rb</p>
<p><strong>mul ra,rb</strong> - Multiply: ra = ra * rb</p>

<h4>Logic Instructions</h4>
<p><strong>and ra,rb</strong> - Bitwise AND</p>
<p><strong>or ra,rb</strong> - Bitwise OR</p>
<p><strong>xor ra,rb</strong> - Bitwise XOR</p>
<p><strong>shl ra,rb</strong> - Shift left</p>
<p><strong>srl ra,rb</strong> - Shift right logical</p>
<p><strong>sra ra,rb</strong> - Shift right arithmetic</p>

<h4>Compare Instructions (set C flag)</h4>
<p><strong>ceq ra,rb</strong> - C = (ra == rb)</p>
<p><strong>cls ra,rb</strong> - C = (ra < rb) signed</p>
<p><strong>clu ra,rb</strong> - C = (ra < rb) unsigned</p>

<h4>Branch Instructions</h4>
<p><strong>bra dd</strong> - Branch always (PC-relative)</p>
<p><strong>brt dd</strong> - Branch if C=true</p>
<p><strong>brf dd</strong> - Branch if C=false</p>

<h4>Memory Instructions</h4>
<p><strong>lb ra,dd(rb)</strong> - Load byte signed</p>
<p><strong>lbu ra,dd(rb)</strong> - Load byte unsigned</p>
<p><strong>lw ra,dd(rb)</strong> - Load word (3 bytes)</p>
<p><strong>sb ra,dd(rb)</strong> - Store byte</p>
<p><strong>sw ra,dd(rb)</strong> - Store word</p>

<h4>Stack Instructions</h4>
<p><strong>push ra</strong> - Decrement sp, store ra</p>
<p><strong>pop ra</strong> - Load ra, increment sp</p>

<h4>Jump Instructions</h4>
<p><strong>jmp (ra)</strong> - Jump to address in ra</p>
<p><strong>jal ra,(rb)</strong> - Jump and link (call)</p>

<h4>Extension Instructions</h4>
<p><strong>sxt ra</strong> - Sign-extend byte to 24-bit word</p>
<p><strong>zxt ra</strong> - Zero-extend byte to 24-bit word</p>

<h4>Register Operations</h4>
<p><strong>mov ra,rb</strong> - Copy register</p>
<p><strong>mov ra,c</strong> - Move condition flag to register (0 or 1)</p>

<h4>Idioms</h4>
<p><strong>halt: bra halt</strong> - Stop execution (branch-to-self infinite loop)</p>
<p><strong>nop</strong> - No operation (encoded as add r0,r0)</p>

<h4>Memory Map</h4>
<table style="font-size:0.85em">
<tr><td><code>000000-0FFFFF</code></td><td>SRAM (1 MB)</td></tr>
<tr><td><code>FEE000-FEFFFF</code></td><td>EBR — 3 KB on MachXO (8 KB address range)</td></tr>
<tr><td><code>FEEC00</code></td><td>Initial SP (top of 3 KB EBR stack)</td></tr>
<tr><td><code>FF0000</code></td><td>LED (write bit 0) / Button (read bit 0)</td></tr>
<tr><td><code>FF0100</code></td><td>UART data</td></tr>
<tr><td><code>FF0101</code></td><td>UART status (bit 0 = TX busy, bit 1 = RX ready)</td></tr>
</table>
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

/// Pre-built Rust pipeline examples (Rust → MSP430 → COR24)
/// Capture current CPU state into an EmulatorState, preserving previous state for heatmap
fn capture_cpu_state(cpu: &WasmCpu, prev: &EmulatorState) -> EmulatorState {
    let regs = cpu.get_registers();
    let mut registers = [0u32; 8];
    for (i, &val) in regs.iter().enumerate().take(8) {
        registers[i] = val;
    }

    // Capture program + data memory: program area + 128 bytes for nearby data,
    // minimum 256 bytes, 16-byte aligned, max 4KB.
    let prog_end = cpu.get_program_end() as usize;
    let low_size = ((prog_end + 128).max(256) & !0xF).min(0x1000);
    let mut memory_low = Vec::with_capacity(low_size);
    for addr in 0..(low_size as u32) {
        memory_low.push(cpu.read_byte(addr));
    }
    // I/O regions: LED/Switch (0xFF0000, 32 bytes) and UART (0xFF0100, 16 bytes)
    let mut memory_io_led = Vec::with_capacity(32);
    for addr in 0xFF0000..0xFF0020 {
        memory_io_led.push(cpu.read_byte(addr));
    }
    let mut memory_io_uart = Vec::with_capacity(16);
    for addr in 0xFF0100..0xFF0110 {
        memory_io_uart.push(cpu.read_byte(addr));
    }

    let sp = cpu.read_register(4);
    let stack_top: u32 = 0xFEEC00;
    let aligned_start = (sp.min(stack_top - 16)) & !0xF;
    let stack_size = (stack_top - aligned_start).min(256);
    let memory_stack = cpu.get_memory_slice(aligned_start, stack_size);

    EmulatorState {
        registers,
        prev_registers: prev.registers,
        prev_prev_registers: prev.prev_registers,
        pc: cpu.get_pc(),
        condition_flag: cpu.get_condition_flag(),
        is_halted: cpu.is_halted(),
        led_value: cpu.get_led_value(),
        led_duty_cycle: if (cpu.get_led_value() & 1) == 1 { 1.0 } else { 0.0 },
        led_on_count: 0,
        instruction_count: cpu.get_instruction_count(),
        memory_low: memory_low.clone(),
        memory_io_led: memory_io_led.clone(),
        memory_io_uart: memory_io_uart.clone(),
        memory_stack: memory_stack.clone(),
        stack_base_addr: aligned_start,
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

fn get_rust_examples() -> Vec<RustExample> {
    vec![
        // Demo 1: Add
        RustExample {
            name: "Add".to_string(),
            description: "Compute 100 + 200 + 42 = 342, return in r0".to_string(),
            rust_source: r#"#[no_mangle]
pub fn demo_add() -> u16 {
    let a: u16 = 100;
    let b: u16 = 200;
    let c: u16 = 42;
    a + b + c  // = 342 (0x156)
}"#.to_string(),
            msp430_asm: r#"demo_add:
	mov	#342, r12         ; compiler constant-folded!
	ret"#.to_string(),
            cor24_assembly: r#"; --- demo_add: returns 342 (0x156) in r0 ---
; The compiler constant-folds 100+200+42 at compile time.
demo_add:
    la      r0, 0x000156
    pop     r2
    jmp     (r2)"#.to_string(),
        },
        // Demo 2: Blink LED
        RustExample {
            name: "Blink LED".to_string(),
            description: "Toggle LED with delay loop".to_string(),
            rust_source: r#"#[no_mangle]
pub unsafe fn demo_blinky() -> ! {
    loop {
        mmio_write(LED_ADDR, 1);   // LED on
        delay(5000);
        mmio_write(LED_ADDR, 0);   // LED off
        delay(5000);
    }
}"#.to_string(),
            msp430_asm: r#"demo_blinky:
.LBB4_1:
	mov	#-256, r12        ; LED_ADDR = 0xFF00
	mov	#1, r13
	call	#mmio_write
	mov	#5000, r12
	call	#delay
	mov	#-256, r12
	clr	r13
	call	#mmio_write
	mov	#5000, r12
	call	#delay
	jmp	.LBB4_1

mmio_write:
	mov	r13, 0(r12)
	ret

delay:
	sub	#2, r1
	tst	r12
	jeq	.LBB2_3
	add	#-1, r12
.LBB2_2:
	mov	r12, 0(r1)
	add	#-1, r12
	cmp	#-1, r12
	jne	.LBB2_2
.LBB2_3:
	add	#2, r1
	ret"#.to_string(),
            cor24_assembly: r#"; --- demo_blinky: toggle LED on/off with delay ---
demo_blinky:
.LBB4_1:
    la      r0, 0xFF0000
    lc      r1, 1
    la      r2, .Lret_6
    push    r2
    la      r2, mmio_write
    jmp     (r2)
    .Lret_6:
    la      r0, 0x001388      ; delay(5000)
    la      r2, .Lret_7
    push    r2
    la      r2, delay
    jmp     (r2)
    .Lret_7:
    la      r0, 0xFF0000
    lc      r1, 0
    la      r2, .Lret_8
    push    r2
    la      r2, mmio_write
    jmp     (r2)
    .Lret_8:
    la      r0, 0x001388      ; delay(5000)
    la      r2, .Lret_9
    push    r2
    la      r2, delay
    jmp     (r2)
    .Lret_9:
    bra     .LBB4_1

mmio_write:
    sw      r1, 0(r0)
    pop     r2
    jmp     (r2)

delay:
    sub     sp, 3
    ceq     r0, z
    brt     .LBB2_3
    add     r0, -1
.LBB2_2:
    mov     r2, sp
    sw      r0, 0(r2)
    add     r0, -1
    lc      r1, -1
    ceq     r0, r1
    brf     .LBB2_2
.LBB2_3:
    add     sp, 3
    pop     r2
    jmp     (r2)"#.to_string(),
        },
        // Demo 3: Button Echo
        RustExample {
            name: "Button Echo".to_string(),
            description: "LED D2 follows button S2".to_string(),
            rust_source: r#"#[no_mangle]
pub unsafe fn demo_button_echo() -> ! {
    loop {
        let btn = mmio_read(LED_ADDR);
        // S2 is active-low: 1=released, 0=pressed
        // Invert so LED is ON when button pressed
        let led = (btn ^ 1) & 1;
        mmio_write(LED_ADDR, led);
    }
}"#.to_string(),
            msp430_asm: r#"demo_button_echo:
.LBB5_1:
	mov	#-256, r12
	call	#mmio_read
	xor	#1, r12           ; invert S2 (active-low)
	mov	r12, r13
	and	#1, r13
	mov	#-256, r12
	call	#mmio_write
	jmp	.LBB5_1

mmio_read:
	mov	0(r12), r12
	ret

mmio_write:
	mov	r13, 0(r12)
	ret"#.to_string(),
            cor24_assembly: r#"; --- demo_button_echo: LED ON when S2 pressed ---
demo_button_echo:
.LBB5_1:
    la      r0, 0xFF0000
    la      r2, .Lret_10
    push    r2
    la      r2, mmio_read
    jmp     (r2)
    .Lret_10:
    mov     r1, r0          ; r1 = switch value
    lc      r0, 1
    xor     r1, r0          ; invert: 1=released->0, 0=pressed->1
    lc      r0, 1
    and     r1, r0          ; mask to bit 0
    la      r0, 0xFF0000
    la      r2, .Lret_11
    push    r2
    la      r2, mmio_write
    jmp     (r2)
    .Lret_11:
    bra     .LBB5_1

mmio_read:
    lw      r0, 0(r0)
    pop     r2
    jmp     (r2)

mmio_write:
    sw      r1, 0(r0)
    pop     r2
    jmp     (r2)"#.to_string(),
        },
        // Demo 4: Countdown
        RustExample {
            name: "Countdown".to_string(),
            description: "Count 10→0 on LED, then halt".to_string(),
            rust_source: r#"#[no_mangle]
pub unsafe fn demo_countdown() {
    let mut count: u16 = 10;
    while count != 0 {
        mmio_write(LED_ADDR, count);
        delay(1000);
        count -= 1;
    }
    mmio_write(LED_ADDR, 0);  // Clear LED
    loop {}  // Halt
}"#.to_string(),
            msp430_asm: r#"demo_countdown:
	push	r10
	mov	#10, r10
.LBB6_1:
	mov	#-256, r12
	mov	r10, r13
	call	#mmio_write
	mov	#1000, r12
	call	#delay
	add	#-1, r10
	tst	r10
	jne	.LBB6_1
	mov	#-256, r12
	clr	r13
	call	#mmio_write
.LBB6_3:
	jmp	.LBB6_3"#.to_string(),
            cor24_assembly: r#"; --- demo_countdown: count 10 down to 0 on LED ---
demo_countdown:
    lw      r0, 18(fp)
    push    r0
    lc      r0, 10
    sw      r0, 18(fp)
.LBB6_1:
    la      r0, 0xFF0000
    lw      r1, 18(fp)
    la      r2, .Lret_12
    push    r2
    la      r2, mmio_write
    jmp     (r2)
    .Lret_12:
    la      r0, 0x0003E8      ; delay(1000)
    la      r2, .Lret_13
    push    r2
    la      r2, delay
    jmp     (r2)
    .Lret_13:
    lw      r0, 18(fp)
    add     r0, -1
    sw      r0, 18(fp)
    lw      r0, 18(fp)
    ceq     r0, z
    brf     .LBB6_1
    la      r0, 0xFF0000
    lc      r1, 0
    la      r2, .Lret_14
    push    r2
    la      r2, mmio_write
    jmp     (r2)
    .Lret_14:
.LBB6_3:
    bra     .LBB6_3

mmio_write:
    sw      r1, 0(r0)
    pop     r2
    jmp     (r2)

delay:
    sub     sp, 3
    ceq     r0, z
    brt     .LBB2_3
    add     r0, -1
.LBB2_2:
    mov     r2, sp
    sw      r0, 0(r2)
    add     r0, -1
    lc      r1, -1
    ceq     r0, r1
    brf     .LBB2_2
.LBB2_3:
    add     sp, 3
    pop     r2
    jmp     (r2)"#.to_string(),
        },
        // Demo 5: Fibonacci
        RustExample {
            name: "Fibonacci".to_string(),
            description: "Compute fib(10) = 55, display on LED".to_string(),
            rust_source: r#"#[no_mangle]
pub fn fibonacci(n: u16) -> u16 {
    if n <= 1 { return n; }
    let mut a: u16 = 0;
    let mut b: u16 = 1;
    let mut i: u16 = 2;
    while i <= n {
        let tmp = a + b;
        a = b;
        b = tmp;
        i += 1;
    }
    b
}

#[no_mangle]
pub unsafe fn demo_fibonacci() {
    let result = fibonacci(10);
    mmio_write(LED_ADDR, result);
    loop {}  // halt
}"#.to_string(),
            msp430_asm: r#"demo_fibonacci:
	mov	#-256, r12        ; compiler constant-folded fib(10)=55
	mov	#55, r13
	call	#mmio_write
.LBB7_1:
	jmp	.LBB7_1

fibonacci:
	cmp	#2, r12
	jhs	.LBB11_2
	mov	r12, r13
	jmp	.LBB11_4
.LBB11_2:
	mov	#2, r14
	clr	r15
	mov	#1, r13
.LBB11_3:
	mov	r13, r11
	mov	r15, r13
	add	r11, r13
	inc	r14
	cmp	r14, r12
	mov	r11, r15
	jhs	.LBB11_3
.LBB11_4:
	mov	r13, r12
	ret"#.to_string(),
            cor24_assembly: r#"; --- demo_fibonacci: fib(10)=55, write to LED ---
; Note: compiler constant-folded the call, so the
; fibonacci function is included but not called.
demo_fibonacci:
    la      r0, 0xFF0000
    lc      r1, 55
    la      r2, .Lret_15
    push    r2
    la      r2, mmio_write
    jmp     (r2)
    .Lret_15:
.LBB7_1:
    bra     .LBB7_1

fibonacci:
    lc      r1, 2
    clu     r0, r1
    brf     .LBB11_2
    mov     r1, r0
    bra     .LBB11_4
.LBB11_2:
    lc      r2, 2
    lc      r0, 0
    sw      r0, 24(fp)
    lc      r1, 1
.LBB11_3:
    sw      r1, 21(fp)
    lw      r1, 24(fp)
    lw      r0, 21(fp)
    add     r1, r0
    add     r2, 1
    clu     r0, r2
    lw      r0, 21(fp)
    sw      r0, 24(fp)
    brf     .LBB11_3
.LBB11_4:
    mov     r0, r1
    pop     r2
    jmp     (r2)

mmio_write:
    sw      r1, 0(r0)
    pop     r2
    jmp     (r2)"#.to_string(),
        },
        // Demo 6: Memory Access
        RustExample {
            name: "Memory Access".to_string(),
            description: "Store and load values from memory".to_string(),
            rust_source: r#"#[no_mangle]
pub unsafe fn demo_memory() {
    let addr: *mut u8 = 0x0080 as *mut u8;
    let val: u8 = 100;

    // Store byte
    *addr = val;

    // Store word (at offset +4)
    let waddr: *mut u16 = 0x0084 as *mut u16;
    *waddr = val as u16;

    // Load them back
    let b = *addr;           // 100
    let w = *waddr;          // 100

    // Show result on LED
    mmio_write(LED_ADDR, (b as u16) + w);
    loop {}  // halt
}"#.to_string(),
            msp430_asm: r#"demo_memory:
	mov	#100, r12
	mov.b	r12, &0x0080      ; store byte
	mov	r12, &0x0084      ; store word
	mov.b	&0x0080, r13      ; load byte
	add	&0x0084, r13      ; load word, add
	mov	#-256, r12
	call	#mmio_write
.LBB_1:
	jmp	.LBB_1"#.to_string(),
            cor24_assembly: r#"; --- demo_memory: store and load values from memory ---
demo_memory:
    lc      r0, 100           ; val = 100
    la      r1, 0x000080      ; addr = 0x0080

    ; Store byte
    sb      r0, 0(r1)         ; mem[0x0080] = 100

    ; Store word (at offset +4)
    sw      r0, 4(r1)         ; mem[0x0084..87] = 100

    ; Load them back
    lb      r2, 0(r1)         ; r2 = mem[0x0080] = 100
    lw      r3, 4(r1)         ; r3 = mem[0x0084] = 100

    ; Show result on LED: 100 + 100 = 200
    add     r2, r3
    la      r0, 0xFF0000
    mov     r1, r2
    la      r3, .Lret_0
    push    r3
    la      r3, mmio_write
    jmp     (r3)
    .Lret_0:
.LBB_1:
    bra     .LBB_1

mmio_write:
    sw      r1, 0(r0)
    pop     r2
    jmp     (r2)"#.to_string(),
        },
        // Demo 7: Software Multiply
        RustExample {
            name: "Multiply".to_string(),
            description: "6 × 7 = 42 via loop, print to UART".to_string(),
            rust_source: r#"// Software multiply: 6 × 7 = 42 via repeated addition
// Prints "42\n" to UART

#[no_mangle]
pub unsafe fn demo_multiply() {
    let mut sum: u16 = 0;
    for _ in 0..7 {
        sum += 6;     // repeated addition
    }
    // sum = 42, print as decimal
    let tens = sum / 10;       // 4
    let ones = sum % 10;       // 2
    uart_putc(tens as u8 + b'0');  // '4'
    uart_putc(ones as u8 + b'0'); // '2'
    uart_putc(b'\n');
    loop {}
}"#.to_string(),
            msp430_asm: r#"demo_multiply:
	clr	r12               ; sum = 0
	mov	#7, r13           ; counter = 7
.Lloop:
	add	#6, r12           ; sum += 6
	dec	r13
	jnz	.Lloop
	; r12 = 42
	; divide by 10 via repeated subtraction
	clr	r13               ; tens = 0
.Ldiv:
	cmp	#10, r12
	jl	.Ldone
	sub	#10, r12
	inc	r13
	jmp	.Ldiv
.Ldone:
	add	#48, r13          ; tens + '0'
	push	r12
	mov	r13, r12
	call	#uart_putc
	pop	r12
	add	#48, r12          ; ones + '0'
	call	#uart_putc
	mov	#10, r12          ; '\n'
	call	#uart_putc
.Lhalt:
	jmp	.Lhalt"#.to_string(),
            cor24_assembly: r#"; --- demo_multiply: 6 × 7 = 42 via repeated addition ---
; Prints "42\n" to UART

demo_multiply:
    lc      r0, 0             ; sum = 0
    lc      r1, 7             ; counter = 7
.Lloop:
    add     r0, 6             ; sum += 6
    push    r0                ; save sum
    lc      r0, 1
    sub     r1, r0            ; counter--
    pop     r0                ; restore sum
    ceq     r1, z
    brf     .Lloop            ; loop while counter != 0

    ; r0 = 42, divide by 10 via repeated subtraction
    lc      r1, 0             ; tens = 0
.Ldiv10:
    lc      r2, 10
    clu     r0, r2            ; r0 < 10?
    brt     .Ldiv_done        ; yes: ones in r0, tens in r1
    sub     r0, r2            ; r0 -= 10
    add     r1, 1             ; tens++
    bra     .Ldiv10
.Ldiv_done:

    ; r0 = ones (2), r1 = tens (4)
    push    r0                ; save ones
    lc      r0, 48            ; '0'
    add     r0, r1            ; r0 = '4'
    la      r2, .Lret_tens
    push    r2
    bra     uart_putc
.Lret_tens:

    pop     r0                ; restore ones
    lc      r1, 48            ; '0'
    add     r0, r1            ; r0 = '2'
    la      r2, .Lret_ones
    push    r2
    bra     uart_putc
.Lret_ones:

    lc      r0, 10            ; '\n'
    la      r2, .Lhalt
    push    r2
    bra     uart_putc
.Lhalt:
    bra     .Lhalt

; uart_putc: poll TX busy, send byte in r0
uart_putc:
    push    r0                ; save char
    la      r1, 0xFF0100      ; UART base
.Ltx_wait:
    lb      r2, 1(r1)         ; read status
    lcu     r0, 128
    and     r2, r0            ; isolate bit 7 (TX busy)
    ceq     r2, z
    brf     .Ltx_wait         ; spin while busy
    pop     r0                ; restore char
    sb      r0, 0(r1)         ; transmit byte
    pop     r2
    jmp     (r2)"#.to_string(),
        },
        // Demo 8: Nested Calls
        RustExample {
            name: "Nested Calls".to_string(),
            description: "Function call chain showing stack frames".to_string(),
            rust_source: r#"// 3-level call chain: demo → a → b → c
// Shows stack frames and calling convention
// Result: LED = 173

#[inline(never)]
pub unsafe fn level_c(x: u16) {
    mmio_write(LED_ADDR, x);  // LED = 173
    loop {}  // halt — 3 stack frames live
}

#[inline(never)]
pub unsafe fn level_b(x: u16) {
    let doubled = x + x;      // 170
    let offset = doubled + 3;  // 173
    level_c(offset)
}

#[inline(never)]
pub unsafe fn level_a(x: u16) {
    level_b(x + 10)            // level_b(85)
}

#[no_mangle]
pub unsafe fn demo_nested() {
    level_a(75);               // 'K'
}"#.to_string(),
            msp430_asm: r#"demo_nested:
	mov	#75, r12
	call	#level_a

level_a:
	add	#10, r12
	call	#level_b

level_b:
	mov	r12, r13
	add	r12, r12
	add	#3, r12
	call	#level_c

level_c:
	mov	r12, r13
	mov	#-256, r12
	call	#mmio_write
.LBB14_1:
	jmp	.LBB14_1"#.to_string(),
            cor24_assembly: r#"; --- demo_nested: 3-level call chain ---
; demo → level_a → level_b → level_c
; arg in r0, return addr pushed on stack
; Result: LED = 173

demo_nested:
    lc      r0, 75            ; start with 75
    la      r2, .Lhalt
    push    r2
    bra     level_a
.Lhalt:
    bra     .Lhalt

; level_a(x): call level_b(x + 10)
level_a:
    push    r2                ; save return addr (shows frame)
    add     r0, 10            ; r0 = x + 10 = 85
    la      r2, .Lret_a
    push    r2
    bra     level_b
.Lret_a:
    pop     r2
    jmp     (r2)              ; return (not reached)

; level_b(x): call level_c(x*2+3)
level_b:
    push    r2                ; save return addr
    add     r0, r0            ; doubled = 170
    add     r0, 3             ; offset = 173
    la      r2, .Lret_b
    push    r2
    bra     level_c
.Lret_b:
    pop     r2
    jmp     (r2)              ; return (not reached)

; level_c(x in r0): LED=x, halt
level_c:
    la      r1, 0xFF0000
    sb      r0, 0(r1)         ; LED = x = 173
.Lhalt_c:
    bra     .Lhalt_c          ; halt with 3 frames live"#.to_string(),
        },
        // Demo 8: Stack Variables
        RustExample {
            name: "Stack Variables".to_string(),
            description: "Local variables and register spilling".to_string(),
            rust_source: r#"#[inline(never)]
pub unsafe fn accumulate(seed: u16) -> u16 {
    let a = seed + 1;       // 33
    let b = a + seed;       // 65
    let c = b + a;          // 98
    let result = a ^ b ^ c; // 2
    result
}

#[no_mangle]
pub unsafe fn demo_stack_vars() {
    let r = accumulate(32);
    mmio_write(LED_ADDR, r); // LED = 2
    loop {}
}"#.to_string(),
            msp430_asm: r#"demo_stack_vars:
	mov	#32, r12
	call	#accumulate
	mov	r12, r13
	mov	#-256, r12
	call	#mmio_write
.LBB0_1:
	jmp	.LBB0_1

accumulate:
	push	r10
	push	r9
	mov	r12, r10          ; seed = 32
	mov	r10, r9
	inc	r9                ; a = 33
	add	r9, r10           ; b = 65
	mov	r10, r12
	add	r9, r12           ; c = 98
	xor	r9, r10
	xor	r12, r10          ; result = a^b^c = 2
	mov	r10, r12
	pop	r9
	pop	r10
	ret"#.to_string(),
            cor24_assembly: r#"; --- demo_stack_vars: register spilling via push/pop ---
; seed=32: a=33, b=65, c=98, result=a^b^c=2
; Only 3 GPRs (r0,r1,r2) — must spill to stack

demo_stack_vars:
    lc      r0, 32            ; seed = 32
    la      r2, .Lret_main
    push    r2
    bra     accumulate
.Lret_main:
    ; r0 = result
    la      r1, 0xFF0000
    sb      r0, 0(r1)         ; LED = result
.Lhalt:
    bra     .Lhalt

; accumulate(seed in r0): returns a^b^c in r0
accumulate:
    ; r0 = seed = 32
    mov     r1, r0            ; r1 = seed
    add     r0, 1             ; r0 = a = 33
    push    r0                ; spill a (need later)

    ; b = a + seed
    add     r0, r1            ; r0 = b = a + seed = 65
    push    r0                ; spill b

    ; c = b + a — retrieve a from stack
    pop     r1                ; r1 = b
    pop     r2                ; r2 = a
    push    r2                ; save a back
    push    r1                ; save b back
    mov     r0, r1            ; r0 = b (copy)
    add     r0, r2            ; r0 = c = b + a = 98

    ; result = a ^ b ^ c
    ; r0=c, need a and b from stack
    pop     r1                ; r1 = b
    pop     r2                ; r2 = a
    xor     r2, r1            ; r2 = a ^ b
    xor     r2, r0            ; r2 = a ^ b ^ c = result
    mov     r0, r2            ; r0 = result

    pop     r2                ; return address
    jmp     (r2)"#.to_string(),
        },
        // Demo 9: UART Hello World
        RustExample {
            name: "UART Hello".to_string(),
            description: "Write \"Hello\\n\" to UART output".to_string(),
            rust_source: r#"const UART_DATA: *mut u8 = 0xFF0100 as _;
const UART_STAT: *const u8 = 0xFF0101 as _;

#[inline(never)]
pub unsafe fn uart_putc(ch: u8) {
    // Poll TX busy (bit 7) before sending
    while (*UART_STAT) & 0x80 != 0 {}
    *UART_DATA = ch;
}

#[no_mangle]
pub unsafe fn demo_uart_hello() {
    for &ch in b"Hello\n" {
        uart_putc(ch);
    }
    loop {}  // halt
}"#.to_string(),
            msp430_asm: r#"demo_uart_hello:
	mov	#72, r12          ; 'H'
	call	#uart_putc
	mov	#101, r12         ; 'e'
	call	#uart_putc
	mov	#108, r12         ; 'l'
	call	#uart_putc
	mov	#108, r12         ; 'l'
	call	#uart_putc
	mov	#111, r12         ; 'o'
	call	#uart_putc
	mov	#10, r12          ; '\n'
	call	#uart_putc
.LBB10_1:
	jmp	.LBB10_1

uart_putc:
	mov	r12, r13
	mov	#-255, r12        ; UART_DATA
	call	#mmio_write
	ret"#.to_string(),
            cor24_assembly: r#"; --- demo_uart_hello: send "Hello\n" via UART ---
; Polls TX busy (bit 7 of status) before each byte
demo_uart_hello:
    lc      r0, 72            ; 'H'
    la      r2, .Lret_20
    push    r2
    bra     uart_putc
    .Lret_20:
    lc      r0, 101           ; 'e'
    la      r2, .Lret_21
    push    r2
    bra     uart_putc
    .Lret_21:
    lc      r0, 108           ; 'l'
    la      r2, .Lret_22
    push    r2
    bra     uart_putc
    .Lret_22:
    lc      r0, 108           ; 'l'
    la      r2, .Lret_23
    push    r2
    bra     uart_putc
    .Lret_23:
    lc      r0, 111           ; 'o'
    la      r2, .Lret_24
    push    r2
    bra     uart_putc
    .Lret_24:
    lc      r0, 10            ; '\n'
    la      r2, .Lret_25
    push    r2
    bra     uart_putc
    .Lret_25:
.LBB10_1:
    bra     .LBB10_1

; uart_putc: poll TX busy, then send byte in r0
uart_putc:
    push    r0                ; save char
    la      r1, 0xFF0100      ; UART base
.Ltx_wait:
    lb      r2, 1(r1)         ; read status
    lcu     r0, 128
    and     r2, r0            ; isolate bit 7 (TX busy)
    ceq     r2, z
    brf     .Ltx_wait         ; spin while busy
    pop     r0                ; restore char
    sb      r0, 0(r1)         ; transmit byte
    pop     r2
    jmp     (r2)"#.to_string(),
        },
    ]
}

