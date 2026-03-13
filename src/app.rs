//! Yew application for COR24 Assembly Emulator

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::Rc;

use components::{
    DebugPanel, ExampleItem, ExamplePicker, Header, Modal, ProgramArea,
    EmulatorState, RustExample, RustPipeline, Sidebar, SidebarButton, SparseMemory, Tab, TabBar, Tooltip,
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
    // Shared UART input queue for Rust tab - run loop drains this each tick
    let rust_uart_queue = use_mut_ref(|| Rc::new(RefCell::new(VecDeque::<u8>::new())));
    // Shared flag for UART clear during animated run
    let rust_uart_clear_flag = use_mut_ref(|| Rc::new(Cell::new(false)));

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
    // Shared UART input queue for assembler tab - run loop drains this each tick
    let asm_uart_queue = use_mut_ref(|| Rc::new(RefCell::new(VecDeque::<u8>::new())));
    // Shared flag for UART clear during animated run
    let asm_uart_clear_flag = use_mut_ref(|| Rc::new(Cell::new(false)));

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
            href: None,
        },
        SidebarButton {
            emoji: "📝".to_string(),
            label: "Examples".to_string(),
            onclick: {
                let examples_open = examples_open.clone();
                Callback::from(move |_| examples_open.set(true))
            },
            title: Some("Load example programs".to_string()),
            href: None,
        },
        SidebarButton {
            emoji: "🎯".to_string(),
            label: "Challenges".to_string(),
            onclick: {
                let challenges_open = challenges_open.clone();
                Callback::from(move |_| challenges_open.set(true))
            },
            title: Some("Test your skills".to_string()),
            href: None,
        },
        SidebarButton {
            emoji: "📖".to_string(),
            label: "ISA Ref".to_string(),
            onclick: {
                let isa_ref_open = isa_ref_open.clone();
                Callback::from(move |_| isa_ref_open.set(true))
            },
            title: Some("Instruction reference".to_string()),
            href: None,
        },
        SidebarButton {
            emoji: "❓".to_string(),
            label: "Help".to_string(),
            onclick: {
                let help_open = help_open.clone();
                Callback::from(move |_| help_open.set(true))
            },
            title: Some("Usage help".to_string()),
            href: None,
        },
        SidebarButton {
            emoji: "📰".to_string(),
            label: "Blog".to_string(),
            onclick: Callback::noop(),
            title: Some("SW-Lab Blog".to_string()),
            href: Some("https://software-wrighter-lab.github.io/".to_string()),
        },
        SidebarButton {
            emoji: "💬".to_string(),
            label: "Discord".to_string(),
            onclick: Callback::noop(),
            title: Some("SW-Lab Discord".to_string()),
            href: Some("https://discord.com/invite/Ctzk5uHggZ".to_string()),
        },
        SidebarButton {
            emoji: "🔧".to_string(),
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

    let on_run = {
        let cpu = cpu.clone();
        let assembly_output = assembly_output.clone();
        let asm_is_running = asm_is_running.clone();
        let asm_emu_state = asm_emu_state.clone();
        let stop_flag = asm_stop_requested.borrow().clone();
        let switches = shared_switches.borrow().clone();
        let uart_q = asm_uart_queue.borrow().clone();
        let uart_clear = asm_uart_clear_flag.borrow().clone();

        Callback::from(move |()| {
            // Start animated run
            asm_is_running.set(true);
            stop_flag.set(false);
            uart_clear.set(false);

            // Initialize shared switch state from current CPU
            switches.set((*cpu).get_switches());

            let cpu_handle = cpu.clone();
            let output_handle = assembly_output.clone();
            let running_handle = asm_is_running.clone();
            let state_handle = asm_emu_state.clone();
            let stop_handle = stop_flag.clone();
            let switch_handle = switches.clone();
            let uart_handle = uart_q.clone();
            let uart_clear_handle = uart_clear.clone();
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
                uart_handle: Rc<RefCell<VecDeque<u8>>>,
                uart_clear_handle: Rc<Cell<bool>>,
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

                // Check if UART clear was requested
                if uart_clear_handle.get() {
                    uart_clear_handle.set(false);
                    current_cpu.clear_uart_output();
                }

                // Read switch state from shared Rc<Cell> (updated by switch onclick)
                current_cpu.set_switches(switch_handle.get());

                // Drain any pending UART input bytes
                {
                    let mut q = uart_handle.borrow_mut();
                    while let Some(byte) = q.pop_front() {
                        current_cpu.uart_send_char(byte as char);
                    }
                }

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
                        run_step(current_cpu, cpu_handle, output_handle, running_handle, state_handle, stop_handle, switch_handle, uart_handle, uart_clear_handle, total_on);
                    }).forget();
                }
            }

            // Start the first step
            gloo::timers::callback::Timeout::new(0, move || {
                run_step(current_cpu, cpu_handle, output_handle, running_handle, state_handle, stop_handle, switch_handle, uart_handle, uart_clear_handle, 0);
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
        let uart_q = rust_uart_queue.borrow().clone();
        let uart_clear = rust_uart_clear_flag.borrow().clone();

        Callback::from(move |()| {
            // Clear stop flag and sync switch state
            stop_flag.borrow().set(false);
            switch_state.borrow().set(*switch_value);
            uart_clear.set(false);

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
            let uart_handle = uart_q.clone();
            let uart_clear_flag = uart_clear.clone();

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
                    prev_mem_low: SparseMemory,
                    prev_mem_io_led: Vec<u8>,
                    prev_mem_io_uart: Vec<u8>,
                    prev_mem_stack: SparseMemory,
                    prev_prev_mem_low: SparseMemory,
                    prev_prev_mem_io_led: Vec<u8>,
                    prev_prev_mem_io_uart: Vec<u8>,
                    prev_prev_mem_stack: SparseMemory,
                    steps: u32,
                    stop_flag: Rc<Cell<bool>>,
                    switch_state: Rc<Cell<u8>>,
                    uart_handle: Rc<RefCell<VecDeque<u8>>>,
                    uart_clear_handle: Rc<Cell<bool>>,
                    cumulative_led_on: u64,
                ) {
                    // Check stop flag
                    if stop_flag.get() {
                        cpu_handle.set(current_cpu);
                        running.set(false);
                        return;
                    }

                    // Check if UART clear was requested
                    if uart_clear_handle.get() {
                        uart_clear_handle.set(false);
                        current_cpu.clear_uart_output();
                    }

                    // Sync switch state before execution
                    current_cpu.set_switches(switch_state.get());

                    // Drain any pending UART input bytes
                    {
                        let mut q = uart_handle.borrow_mut();
                        while let Some(byte) = q.pop_front() {
                            current_cpu.uart_send_char(byte as char);
                        }
                    }

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
                    let memory_low = current_cpu.get_sparse_sram();
                    let mut memory_io_led = Vec::with_capacity(16);
                    for addr in 0xFF0000..0xFF0010 {
                        memory_io_led.push(current_cpu.read_byte(addr));
                    }
                    let mut memory_io_uart = Vec::with_capacity(16);
                    for addr in 0xFF0100..0xFF0110 {
                        memory_io_uart.push(current_cpu.read_byte(addr));
                    }
                    let memory_stack = current_cpu.get_sparse_ebr();

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
                            run_step(current_cpu, cpu_handle, running, state, asm_lines, next_prev_regs, next_prev_prev_regs, next_prev_mem_low, next_prev_mem_io_led, next_prev_mem_io_uart, next_prev_mem_stack, next_prev_prev_mem_low, next_prev_prev_mem_io_led, next_prev_prev_mem_io_uart, next_prev_prev_mem_stack, steps + 500, stop_flag, switch_state, uart_handle, uart_clear_handle, next_led_on);
                        }).forget();
                    }
                }

                run_step(initial_cpu, cpu_handle, running, state, asm_lines, prev_regs, prev_prev_regs, prev_mem_low, prev_mem_io_led, prev_mem_io_uart, prev_mem_stack, prev_prev_mem_low, prev_prev_mem_io_led, prev_prev_mem_io_uart, prev_prev_mem_stack, 0, stop_flag, switch_state, uart_handle, uart_clear_flag, 0);
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

    // Rust pipeline: UART send
    let on_rust_uart_send = {
        let rust_cpu = rust_cpu.clone();
        let rust_emu_state = rust_emu_state.clone();
        let rust_is_running = rust_is_running.clone();
        let uart_q = rust_uart_queue.borrow().clone();
        Callback::from(move |byte: u8| {
            if *rust_is_running {
                // During animated run, push to shared queue — run loop drains it
                uart_q.borrow_mut().push_back(byte);
            } else {
                // Not running — inject directly and step the CPU
                let mut cpu = (*rust_cpu).clone();
                cpu.uart_send_char(byte as char);
                let _ = cpu.run();
                let state = capture_cpu_state(&cpu, &rust_emu_state);
                rust_emu_state.set(state);
                rust_cpu.set(cpu);
            }
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
    let rust_examples = crate::rust_examples::get_rust_examples();

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

    // Assembler UART send callback
    let on_asm_uart_send = {
        let cpu = cpu.clone();
        let asm_emu_state = asm_emu_state.clone();
        let asm_is_running = asm_is_running.clone();
        let uart_q = asm_uart_queue.borrow().clone();
        Callback::from(move |byte: u8| {
            if *asm_is_running {
                // During animated run, push to shared queue — run loop drains it
                uart_q.borrow_mut().push_back(byte);
            } else {
                // Not running — inject directly and step the CPU
                let mut new_cpu = (*cpu).clone();
                new_cpu.uart_send_char(byte as char);
                let _ = new_cpu.run();
                let state = capture_cpu_state(&new_cpu, &asm_emu_state);
                asm_emu_state.set(state);
                cpu.set(new_cpu);
            }
        })
    };

    // Assembler UART clear callback
    let on_asm_uart_clear = {
        let cpu = cpu.clone();
        let asm_emu_state = asm_emu_state.clone();
        let asm_is_running = asm_is_running.clone();
        let clear_flag = asm_uart_clear_flag.borrow().clone();
        Callback::from(move |()| {
            if *asm_is_running {
                // During animated run, signal the run loop to clear
                clear_flag.set(true);
            } else {
                let mut new_cpu = (*cpu).clone();
                new_cpu.clear_uart_output();
                let state = capture_cpu_state(&new_cpu, &asm_emu_state);
                asm_emu_state.set(state);
                cpu.set(new_cpu);
            }
        })
    };

    // Rust pipeline UART clear callback
    let on_rust_uart_clear = {
        let rust_cpu = rust_cpu.clone();
        let rust_emu_state = rust_emu_state.clone();
        let rust_is_running = rust_is_running.clone();
        let clear_flag = rust_uart_clear_flag.borrow().clone();
        Callback::from(move |()| {
            if *rust_is_running {
                clear_flag.set(true);
            } else {
                let mut cpu = (*rust_cpu).clone();
                cpu.clear_uart_output();
                let state = capture_cpu_state(&cpu, &rust_emu_state);
                rust_emu_state.set(state);
                rust_cpu.set(cpu);
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
                        on_uart_send={on_asm_uart_send}
                        on_uart_clear={on_asm_uart_clear}
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
                    on_uart_send={on_rust_uart_send}
                    on_uart_clear={on_rust_uart_clear}
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
                    let asm_is_running = asm_is_running.clone();
                    let asm_assembled = asm_assembled.clone();
                    let asm_emu_state = asm_emu_state.clone();
                    let stop_flag = asm_stop_requested.borrow().clone();
                    Callback::from(move |idx: usize| {
                        if let Some((_, _, code)) = examples.get(idx) {
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
                {render_challenges_list(
                    challenge_mode.clone(), current_challenge_id.clone(),
                    program_code.clone(), challenges_open.clone(),
                    asm_is_running.clone(), asm_assembled.clone(),
                    asm_emu_state.clone(), cpu.clone(),
                    assembly_output.clone(), assembly_lines.clone(),
                    asm_stop_requested.borrow().clone(), challenge_result.clone(),
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
        led_duty_cycle: if (cpu.get_led_value() & 1) == 1 { 1.0 } else { 0.0 },
        led_on_count: 0,
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

