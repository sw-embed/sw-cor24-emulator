//! Yew application for COR24 Assembly Emulator

use std::cell::Cell;
use std::rc::Rc;

use components::{
    Collapsible, Header, LegendItem, MemoryViewer, Modal, ProgramArea, Register, RegisterPanel,
    RustCpuState, RustExample, RustPipeline, Sidebar, SidebarButton, Tab, TabBar,
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
    let rust_cpu_state = use_state(RustCpuState::default);
    let rust_is_loaded = use_state(|| false);
    let rust_is_running = use_state(|| false);
    let rust_loaded_example = use_state(|| None::<RustExample>);

    // State management
    let cpu = use_state(WasmCpu::new);
    let program_code = use_state(|| String::from(EXAMPLE_PROGRAM));
    let assembly_output = use_state(|| None::<Html>);
    let assembly_lines = use_state(Vec::<String>::new);
    let last_registers = use_state(|| vec![0u32; 8]);
    let challenge_mode = use_state(|| false);
    let current_challenge_id = use_state(|| None::<usize>);
    let challenge_result = use_state(|| None::<Result<String, String>>);

    // Animated run state for assembler tab
    let asm_is_running = use_state(|| false);
    // Use Rc<Cell> for stop flag - provides immediate visibility across closures
    let asm_stop_requested = use_mut_ref(|| Rc::new(Cell::new(false)));

    // Modal states
    let tutorial_open = use_state(|| false);
    let examples_open = use_state(|| false);
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

        Callback::from(move |code: String| {
            program_code.set(code.clone());

            // Assemble the source code
            let mut new_cpu = (*cpu).clone();
            match new_cpu.assemble(&code) {
                Ok(_output) => {
                    // Get assembled lines for display
                    let lines = new_cpu.get_assembled_lines();
                    assembly_lines.set(lines);
                    cpu.set(new_cpu);

                    assembly_output.set(Some(html! {
                        <div class="success-text">
                            {"✓ Program assembled successfully"}
                        </div>
                    }));
                }
                Err(e) => {
                    assembly_lines.set(Vec::new());
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
        let last_registers = last_registers.clone();

        Callback::from(move |()| {
            let mut new_cpu = (*cpu).clone();

            // Save current state for change tracking
            last_registers.set(new_cpu.get_registers());

            match new_cpu.step() {
                Ok(_) => {
                    cpu.set(new_cpu);
                }
                Err(e) => {
                    assembly_output.set(Some(html! {
                        <div class="error-text">
                            {format!("Error: {:?}", e)}
                        </div>
                    }));
                }
            }
        })
    };

    let on_run = {
        let cpu = cpu.clone();
        let assembly_output = assembly_output.clone();
        let asm_is_running = asm_is_running.clone();
        let stop_flag = asm_stop_requested.borrow().clone();

        Callback::from(move |()| {
            // Start animated run
            asm_is_running.set(true);
            stop_flag.set(false);

            let cpu_handle = cpu.clone();
            let output_handle = assembly_output.clone();
            let running_handle = asm_is_running.clone();
            let stop_handle = stop_flag.clone();
            let current_cpu = (*cpu).clone();

            // Start the animated run loop
            fn run_step(
                mut current_cpu: WasmCpu,
                cpu_handle: yew::UseStateHandle<WasmCpu>,
                output_handle: yew::UseStateHandle<Option<Html>>,
                running_handle: yew::UseStateHandle<bool>,
                stop_handle: Rc<Cell<bool>>,
            ) {
                // Check if stop was requested (immediate - no state delay)
                if stop_handle.get() {
                    cpu_handle.set(current_cpu);
                    running_handle.set(false);
                    output_handle.set(Some(yew::html! {
                        <div class="info-text">
                            {"⏹ Execution stopped"}
                        </div>
                    }));
                    return;
                }

                // Sync switch state from UI (user may have toggled switches)
                let ui_switches = (*cpu_handle).get_switches();
                current_cpu.set_switches(ui_switches);

                // Execute a small batch of instructions (small batch = responsive to input)
                let mut halted = false;
                let mut error_msg = None;
                for _ in 0..10 {
                    if current_cpu.is_halted() {
                        halted = true;
                        break;
                    }
                    if let Err(e) = current_cpu.step() {
                        error_msg = Some(format!("{:?}", e));
                        halted = true;
                        break;
                    }
                }

                // Update CPU state for UI refresh (includes LED state)
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
                        run_step(current_cpu, cpu_handle, output_handle, running_handle, stop_handle);
                    }).forget();
                }
            }

            // Start the first step
            gloo::timers::callback::Timeout::new(0, move || {
                run_step(current_cpu, cpu_handle, output_handle, running_handle, stop_handle);
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
        let assembly_output = assembly_output.clone();
        let assembly_lines = assembly_lines.clone();

        Callback::from(move |()| {
            // Full reset - create new CPU with cleared memory
            cpu.set(WasmCpu::new());
            assembly_lines.set(Vec::new());
            assembly_output.set(None);
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
        let rust_cpu_state = rust_cpu_state.clone();
        let rust_is_loaded = rust_is_loaded.clone();
        let rust_loaded_example = rust_loaded_example.clone();

        Callback::from(move |example: RustExample| {
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
                // Capture high memory (0xFFFF80-0xFFFFFF) - 128 bytes
                let mut memory_high = Vec::new();
                for addr in 0xFFFF80..=0xFFFFFF {
                    memory_high.push(new_cpu.read_byte(addr as u32));
                }

                rust_cpu_state.set(RustCpuState {
                    registers,
                    prev_registers: registers, // No changes on initial load
                    prev_prev_registers: registers,
                    pc: new_cpu.get_pc(),
                    condition_flag: new_cpu.get_condition_flag(),
                    is_halted: new_cpu.is_halted(),
                    led_value: new_cpu.get_led_value(),
                    cycle_count: new_cpu.get_cycle_count(),
                    memory_low: memory_low.clone(),
                    memory_high: memory_high.clone(),
                    prev_memory_low: memory_low.clone(),
                    prev_memory_high: memory_high.clone(),
                    prev_prev_memory_low: memory_low,
                    prev_prev_memory_high: memory_high,
                    current_instruction: new_cpu.get_current_instruction(),
                    assembled_lines,
                });

                rust_cpu.set(new_cpu);
                rust_is_loaded.set(true);
                rust_loaded_example.set(Some(example));
            }
        })
    };

    // Rust pipeline: Step one instruction
    let on_rust_step = {
        let rust_cpu = rust_cpu.clone();
        let rust_cpu_state = rust_cpu_state.clone();

        Callback::from(move |()| {
            let mut new_cpu = (*rust_cpu).clone();
            let prev_state = (*rust_cpu_state).clone();

            if new_cpu.step().is_ok() {
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
                // Capture high memory (0xFFFF80-0xFFFFFF) - 128 bytes
                let mut memory_high = Vec::new();
                for addr in 0xFFFF80..=0xFFFFFF {
                    memory_high.push(new_cpu.read_byte(addr as u32));
                }

                rust_cpu_state.set(RustCpuState {
                    registers,
                    prev_registers: prev_state.registers,
                    prev_prev_registers: prev_state.prev_registers,
                    pc: new_cpu.get_pc(),
                    condition_flag: new_cpu.get_condition_flag(),
                    is_halted: new_cpu.is_halted(),
                    led_value: new_cpu.get_led_value(),
                    cycle_count: new_cpu.get_cycle_count(),
                    memory_low,
                    memory_high,
                    prev_memory_low: prev_state.memory_low,
                    prev_memory_high: prev_state.memory_high,
                    prev_prev_memory_low: prev_state.prev_memory_low,
                    prev_prev_memory_high: prev_state.prev_memory_high,
                    current_instruction: new_cpu.get_current_instruction(),
                    assembled_lines: prev_state.assembled_lines,
                });
                rust_cpu.set(new_cpu);
            }
        })
    };

    // Rust pipeline: Run to completion (or max cycles)
    let on_rust_run = {
        let rust_cpu = rust_cpu.clone();
        let rust_is_running = rust_is_running.clone();
        let rust_cpu_state = rust_cpu_state.clone();

        Callback::from(move |()| {
            rust_is_running.set(true);
            let cpu_handle = rust_cpu.clone();
            let running = rust_is_running.clone();
            let state = rust_cpu_state.clone();
            let asm_lines = (*state).assembled_lines.clone();
            let initial_cpu = (*rust_cpu).clone();
            let prev_regs = (*state).registers;
            let prev_prev_regs = (*state).prev_registers;
            let prev_mem_low = (*state).memory_low.clone();
            let prev_mem_high = (*state).memory_high.clone();
            let prev_prev_mem_low = (*state).prev_memory_low.clone();
            let prev_prev_mem_high = (*state).prev_memory_high.clone();

            // Run with animation using timer
            gloo::timers::callback::Timeout::new(50, move || {
                #[allow(clippy::too_many_arguments)]
                fn run_step(
                    mut current_cpu: WasmCpu,
                    cpu_handle: yew::UseStateHandle<WasmCpu>,
                    running: yew::UseStateHandle<bool>,
                    state: yew::UseStateHandle<RustCpuState>,
                    asm_lines: Vec<String>,
                    prev_regs: [u32; 8],
                    prev_prev_regs: [u32; 8],
                    prev_mem_low: Vec<u8>,
                    prev_mem_high: Vec<u8>,
                    prev_prev_mem_low: Vec<u8>,
                    prev_prev_mem_high: Vec<u8>,
                    steps: u32,
                ) {
                    // Execute a batch of instructions
                    let mut halted = false;
                    for _ in 0..10 {
                        if current_cpu.is_halted() || current_cpu.should_stop_for_led() {
                            halted = true;
                            break;
                        }
                        if current_cpu.step().is_err() {
                            halted = true;
                            break;
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
                    // Capture high memory (0xFFFF80-0xFFFFFF) - 128 bytes
                    let mut memory_high = Vec::new();
                    for addr in 0xFFFF80..=0xFFFFFF {
                        memory_high.push(current_cpu.read_byte(addr as u32));
                    }

                    // Save current values as prev for next iteration
                    let next_prev_regs = registers;
                    let next_prev_prev_regs = prev_regs;
                    let next_prev_mem_low = memory_low.clone();
                    let next_prev_mem_high = memory_high.clone();
                    let next_prev_prev_mem_low = prev_mem_low.clone();
                    let next_prev_prev_mem_high = prev_mem_high.clone();

                    state.set(RustCpuState {
                        registers,
                        prev_registers: prev_regs,
                        prev_prev_registers: prev_prev_regs,
                        pc: current_cpu.get_pc(),
                        condition_flag: current_cpu.get_condition_flag(),
                        is_halted: current_cpu.is_halted(),
                        led_value: current_cpu.get_led_value(),
                        cycle_count: current_cpu.get_cycle_count(),
                        memory_low,
                        memory_high,
                        prev_memory_low: prev_mem_low,
                        prev_memory_high: prev_mem_high,
                        prev_prev_memory_low: prev_prev_mem_low,
                        prev_prev_memory_high: prev_prev_mem_high,
                        current_instruction: current_cpu.get_current_instruction(),
                        assembled_lines: asm_lines.clone(),
                    });

                    if halted || steps > 1000 {
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
                            run_step(current_cpu, cpu_handle, running, state, asm_lines, next_prev_regs, next_prev_prev_regs, next_prev_mem_low, next_prev_mem_high, next_prev_prev_mem_low, next_prev_prev_mem_high, steps + 10);
                        }).forget();
                    }
                }

                run_step(initial_cpu, cpu_handle, running, state, asm_lines, prev_regs, prev_prev_regs, prev_mem_low, prev_mem_high, prev_prev_mem_low, prev_prev_mem_high, 0);
            }).forget();
        })
    };

    // Rust pipeline: Reset
    let on_rust_reset = {
        let rust_cpu = rust_cpu.clone();
        let rust_cpu_state = rust_cpu_state.clone();
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
                    // Capture high memory (0xFFFF80-0xFFFFFF) - 128 bytes
                    let mut memory_high = Vec::new();
                    for addr in 0xFFFF80..=0xFFFFFF {
                        memory_high.push(new_cpu.read_byte(addr as u32));
                    }

                    rust_cpu_state.set(RustCpuState {
                        registers,
                        prev_registers: registers, // No changes on reset
                        prev_prev_registers: registers,
                        pc: new_cpu.get_pc(),
                        condition_flag: new_cpu.get_condition_flag(),
                        is_halted: new_cpu.is_halted(),
                        led_value: new_cpu.get_led_value(),
                        cycle_count: new_cpu.get_cycle_count(),
                        memory_low: memory_low.clone(),
                        memory_high: memory_high.clone(),
                        prev_memory_low: memory_low.clone(),
                        prev_memory_high: memory_high.clone(),
                        prev_prev_memory_low: memory_low,
                        prev_prev_memory_high: memory_high,
                        current_instruction: new_cpu.get_current_instruction(),
                        assembled_lines,
                    });

                    rust_cpu.set(new_cpu);
                }
            }
        })
    };

    // Rust pipeline: Unload (clear loaded state)
    let on_rust_unload = {
        let rust_is_loaded = rust_is_loaded.clone();
        let rust_loaded_example = rust_loaded_example.clone();
        Callback::from(move |()| {
            rust_is_loaded.set(false);
            rust_loaded_example.set(None);
        })
    };

    // Tab definitions
    let tabs = vec![
        Tab { id: "assembler".to_string(), label: "Assembler".to_string() },
        Tab { id: "rust".to_string(), label: "Rust".to_string() },
    ];

    // Register panel data
    let registers = {
        let regs = (*cpu).get_registers();
        let last_regs = &*last_registers;
        let reg_names = ["r0", "r1", "r2", "fp", "sp", "z", "iv", "ir"];
        let mut reg_list = Vec::new();
        for i in 0..8 {
            let changed = regs[i] != last_regs[i];
            reg_list.push(Register {
                name: reg_names[i].to_string(),
                value: format!("0x{:06X} ({})", regs[i], regs[i] as i32),
                changed,
            });
        }
        reg_list
    };

    let legend_items = vec![
        LegendItem {
            label: "fp".to_string(),
            value: "Frame Pointer (r3)".to_string(),
            changed: false,
        },
        LegendItem {
            label: "sp".to_string(),
            value: "Stack Pointer (r4)".to_string(),
            changed: false,
        },
        LegendItem {
            label: "z".to_string(),
            value: "Zero for comparisons (r5)".to_string(),
            changed: false,
        },
        LegendItem {
            label: "iv".to_string(),
            value: "Interrupt Vector (r6)".to_string(),
            changed: false,
        },
        LegendItem {
            label: "ir".to_string(),
            value: "Interrupt Return (r7)".to_string(),
            changed: false,
        },
        LegendItem {
            label: "C".to_string(),
            value: format!("Condition: {}", if (*cpu).get_c_flag() { "1" } else { "0" }),
            changed: false,
        },
    ];

    // Memory data
    let memory = (*cpu).get_memory_slice(0, 128);
    let pc = (*cpu).pc() as u16;

    // Get examples for the modal
    let examples = get_examples();

    // Pre-built Rust examples
    let rust_examples = get_rust_examples();

    html! {
        <div class="container">
            <Header title="COR24 C-Oriented RISC, 24-bit - Assembly Emulator">
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
                    on_step={on_step}
                    on_run={on_run.clone()}
                    on_stop={on_stop}
                    on_reset={on_reset}
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
                    step_enabled={!(*cpu).is_halted()}
                    run_enabled={!(*cpu).is_halted()}
                />

                <div class="right-panels">
                    <div class="registers-panel">
                        <RegisterPanel
                            registers={registers}
                            legend_items={legend_items}
                        />

                        // CPU Status
                        <div class="cpu-status">
                            <div class="status-item">
                                <span class="status-label">{"PC:"}</span>
                                <span class="status-value">{format!("0x{:06X}", (*cpu).pc())}</span>
                            </div>
                            <div class="status-item">
                                <span class="status-label">{"Cycles:"}</span>
                                <span class="status-value">{(*cpu).cycle_count()}</span>
                            </div>
                            <div class="status-item">
                                <span class="status-label">{"Instructions:"}</span>
                                <span class="status-value">{(*cpu).instruction_count()}</span>
                            </div>
                            <div class="status-item">
                                <span class="status-label">{"Status:"}</span>
                                <span class="status-value">
                                    {if (*cpu).is_halted() { "HALTED" } else { "RUNNING" }}
                                </span>
                            </div>
                        </div>
                    </div>

                    // I/O Panel: LEDs and Switches (Collapsible)
                    <Collapsible title="I/O Peripherals" initially_open={true}>
                        <div class="io-section">
                            <div class="io-label">{"LEDs (0xFF0000)"}</div>
                            <div class="led-row">
                                {for (0..8).rev().map(|i| {
                                    let led_on = ((*cpu).get_leds() >> i) & 1 == 1;
                                    let class = if led_on { "led led-on" } else { "led led-off" };
                                    html! {
                                        <div class={class} title={format!("LED {}", i)}>
                                            {i}
                                        </div>
                                    }
                                })}
                            </div>
                        </div>
                        <div class="io-section">
                            <div class="io-label">{"Switches (read from 0xFF0000)"}</div>
                            <div class="switch-row">
                                {for (0..8).rev().map(|i| {
                                    let switch_on = ((*cpu).get_switches() >> i) & 1 == 1;
                                    let class = if switch_on { "switch switch-on" } else { "switch switch-off" };
                                    let cpu = cpu.clone();
                                    let onclick = Callback::from(move |_| {
                                        let mut new_cpu = (*cpu).clone();
                                        new_cpu.toggle_switch(i);
                                        cpu.set(new_cpu);
                                    });
                                    html! {
                                        <div class={class} title={format!("Switch {}", i)} onclick={onclick}>
                                            {i}
                                        </div>
                                    }
                                })}
                            </div>
                        </div>
                        <div class="io-values">
                            <span>{"LEDs: "}{format!("0x{:02X}", (*cpu).get_leds())}</span>
                            <span>{"  SW: "}{format!("0x{:02X}", (*cpu).get_switches())}</span>
                        </div>
                    </Collapsible>

                    <MemoryViewer
                        memory={memory}
                        pc={pc}
                        title={Some("Memory (First 128 Bytes)".to_string())}
                        bytes_per_row={16}
                        bytes_to_show={128}
                    />
                </div>
            </div>

            // Rust Pipeline Tab Content - Full width wizard layout
            <div class={if *active_tab == "rust" { "rust-tab-content full-width" } else { "rust-tab-content hidden" }}>
                <RustPipeline
                    examples={rust_examples}
                    loaded_example={(*rust_loaded_example).clone()}
                    on_load={on_rust_load}
                    on_step={on_rust_step}
                    on_run={on_rust_run}
                    on_reset={on_rust_reset}
                    on_unload={on_rust_unload}
                    cpu_state={(*rust_cpu_state).clone()}
                    is_loaded={*rust_is_loaded}
                    is_running={*rust_is_running}
                    on_tutorial_open={
                        let tutorial_open = tutorial_open.clone();
                        Callback::from(move |_| tutorial_open.set(true))
                    }
                    on_examples_open={
                        let examples_open = examples_open.clone();
                        Callback::from(move |_| examples_open.set(true))
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
                                    match validate_challenge(challenge_id, &(*program_code)) {
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

            <Modal id="examples" title="Examples" active={*examples_open} on_close={close_examples}>
                <div class="examples-list">
                    {for examples.iter().enumerate().map(|(idx, (title, desc, code))| {
                        let program_code = program_code.clone();
                        let examples_open = examples_open.clone();
                        let cpu = cpu.clone();
                        let assembly_output = assembly_output.clone();
                        let assembly_lines = assembly_lines.clone();
                        let code = code.clone();
                        let challenge_mode = challenge_mode.clone();
                        let current_challenge_id = current_challenge_id.clone();
                        let challenge_result = challenge_result.clone();

                        let load_example = Callback::from(move |_: MouseEvent| {
                            // Reset CPU
                            cpu.set(WasmCpu::new());
                            assembly_output.set(None);
                            assembly_lines.set(Vec::new());

                            // Exit challenge mode when loading an example
                            challenge_mode.set(false);
                            current_challenge_id.set(None);
                            challenge_result.set(None);

                            // Load new code
                            program_code.set(code.clone());
                            examples_open.set(false);
                        });

                        html! {
                            <div class="example-item" key={idx} onclick={load_example}>
                                <h4>{title}</h4>
                                <p>{desc}</p>
                            </div>
                        }
                    })}
                </div>
            </Modal>

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
                <div class="footer-left">
                    <span>{"MIT License"}</span>
                    <span>{"© 2026 Michael A Wright"}</span>
                </div>
                <div class="footer-right">
                    <span>{format!("{} | {} | {}", env!("VERGEN_BUILD_HOST"), env!("VERGEN_GIT_SHA_SHORT"), env!("VERGEN_BUILD_TIMESTAMP"))}</span>
                </div>
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
const EXAMPLE_PROGRAM: &str = "; COR24 Example: Basic Arithmetic
; Load constants and add them

        lc      r0,10       ; r0 = 10
        lc      r1,20       ; r1 = 20
        add     r0,r1       ; r0 = r0 + r1 = 30

        lc      r2,5        ; r2 = 5
        add     r0,r2       ; r0 = 35

        halt                ; Stop execution";

const TUTORIAL_CONTENT: &str = r#"
<h3>Welcome to the COR24 Assembly Emulator!</h3>
<p>This emulator teaches you assembly programming using the COR24 C-Oriented RISC architecture.</p>

<h4>CPU Features:</h4>
<ul>
    <li><strong>3 GP Registers (24-bit)</strong>: r0, r1, r2</li>
    <li><strong>5 Special Registers</strong>: fp, sp, z, iv, ir</li>
    <li><strong>16MB Memory</strong>: 24-bit addressable, little-endian</li>
    <li><strong>Single Condition Flag (C)</strong>: Set by compare instructions</li>
    <li><strong>Variable-length Instructions</strong>: 1, 2, or 4 bytes</li>
</ul>

<h4>Register Aliases:</h4>
<ul>
    <li><code>fp (r3)</code> - Frame Pointer</li>
    <li><code>sp (r4)</code> - Stack Pointer</li>
    <li><code>z (r5)</code> - Zero (for compare instructions)</li>
    <li><code>iv (r6)</code> - Interrupt Vector</li>
    <li><code>ir (r7)</code> - Interrupt Return</li>
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
    <li><code>halt</code> - Stop execution</li>
</ul>
"#;

const ISA_REF_CONTENT: &str = r#"
<h3>COR24 Instruction Set Reference</h3>

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

<h4>Special</h4>
<p><strong>halt</strong> - Stop execution (la ir,addr form)</p>
<p><strong>mov ra,rb</strong> - Copy register</p>
<p><strong>mov ra,c</strong> - Move condition flag to register</p>
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

/// Pre-built Rust pipeline examples
fn get_rust_examples() -> Vec<RustExample> {
    vec![
        RustExample {
            name: "LED Blink".to_string(),
            description: "Binary counter with delay loop".to_string(),
            rust_source: r#"#![no_std]
use core::panic::PanicInfo;

const LEDS: *mut u8 = 0xFF0000 as *mut u8;
const DELAY: u32 = 10; // Spin loop iterations

#[panic_handler]
fn panic(_: &PanicInfo) -> ! { loop {} }

#[no_mangle]
pub extern "C" fn main() -> ! {
    let mut counter: u8 = 0;
    loop {
        unsafe { core::ptr::write_volatile(LEDS, counter); }
        // Delay loop
        for _ in 0..DELAY {
            unsafe { core::ptr::read_volatile(LEDS); }
        }
        counter = counter.wrapping_add(1);
    }
}"#.to_string(),
            wasm_hex: "0061 736d 0100 0000 010a 0260 0000 6002\n\
                       7f7f 017f 0303 0200 01".to_string(),
            wasm_size: 64,
            wasm_disassembly: r#"(module
  (func (;0;) (type 0)
    (local i32 i32)
    i32.const 0
    local.set 0
    loop  ;; outer loop
      ;; write LED
      i32.const 0xFF0000
      local.get 0
      i32.store8
      ;; delay loop: count 10 iterations
      i32.const 10
      local.set 1
      loop  ;; delay
        local.get 1
        i32.const 1
        i32.sub
        local.tee 1
        br_if 0
      end
      ;; increment counter
      local.get 0
      i32.const 1
      i32.add
      local.set 0
      br 0
    end))"#.to_string(),
            cor24_assembly: r#"; LED Blink with delay loop
; counter in 0(fp), delay in r2
main:
        push    fp
        mov     fp, sp
        add     sp, -3          ; reserve 1 word for counter
        lc      r0, 0
        sw      r0, 0(fp)       ; counter = 0
.loop:
        ; Write counter to LEDs
        la      r0, 0xFF0000
        lw      r1, 0(fp)
        sb      r1, 0(r0)
        ; Delay loop: r2 = 10, count down to 0
        lc      r2, 10
.delay:
        lc      r0, 1
        sub     r2, r0
        brt     .delay          ; branch if r2 > 0
        ; Increment counter
        lw      r0, 0(fp)
        lc      r1, 1
        add     r0, r1
        sw      r0, 0(fp)
        bra     .loop"#.to_string(),
            machine_code_hex: "6a4c 21fd 4400 8a00 2b00 00ff 9300 8800\n\
                              4602 4401 0a14 fd92 0045 0101 8a00 13e6".to_string(),
            machine_code_size: 32,
            listing: r#"0000: 6A           push    fp
0001: 4C           mov     fp, sp
0002: 21 FD        add     sp, -3
0004: 44 00        lc      r0, 0
0006: 8A 00        sw      r0, 0(fp)
0008: 2B 00 00 FF  .loop: la r0, 0xFF0000
000C: 93 00        lw      r1, 0(fp)
000E: 88 00        sb      r1, 0(r0)
0010: 46 0A        lc      r2, 10
0012: 44 01   .delay: lc r0, 1
0014: 0A           sub     r2, r0
0015: 15 FB        brt     .delay
0017: 92 00        lw      r0, 0(fp)
0019: 45 01        lc      r1, 1
001B: 01           add     r0, r1
001C: 8A 00        sw      r0, 0(fp)
001E: 13 E8        bra     .loop"#.to_string(),
        },
        RustExample {
            name: "Add & Store".to_string(),
            description: "Add two numbers and store result to memory locations".to_string(),
            rust_source: r#"#![no_std]
use core::panic::PanicInfo;

#[panic_handler]
fn panic(_: &PanicInfo) -> ! { loop {} }

// Store result at multiple memory locations
static mut RESULT_A: u32 = 0;
static mut RESULT_B: u32 = 0;
static mut RESULT_C: u32 = 0;

#[no_mangle]
pub extern "C" fn add_and_store(a: i32, b: i32) {
    let sum = a + b;
    unsafe {
        RESULT_A = sum as u32;  // Store at 0x50
        RESULT_B = RESULT_A;    // Copy to 0x53
        RESULT_C = RESULT_B;    // Copy to 0x56
    }
}"#.to_string(),
            wasm_hex: "0061 736d 0100 0000 0107 0160 027f 7f01\n\
                       7f03 0201 0007 0701 0361 6464 0000 0a09\n\
                       0107 0020 0020 016a 0b".to_string(),
            wasm_size: 42,
            wasm_disassembly: r#"(module
  (type (;0;) (func (param i32 i32) (result i32)))
  (func (;0;) (type 0) (param i32 i32) (result i32)
    local.get 0
    local.get 1
    i32.add
    ;; store to memory locations
    global.set $result_a
    global.get $result_a
    global.set $result_b
    global.get $result_b
    global.set $result_c)
  (export "add_and_store" (func 0)))"#.to_string(),
            cor24_assembly: r#"; Add & Store: compute sum and store at 3 memory locations
; Input: r0 = a (10), r1 = b (25)
; Output: result stored at 0x50, 0x53, 0x56
add_and_store:
        push    fp
        mov     fp, sp
        ; Load inputs: a=10, b=25
        lc      r0, 10          ; r0 = 10
        lc      r1, 25          ; r1 = 25
        ; Compute sum
        add     r0, r1          ; r0 = 10 + 25 = 35
        ; Store result at 0x50
        la      r2, 0x000050
        sw      r0, 0(r2)       ; mem[0x50] = 35
        ; Copy to 0x53
        lw      r1, 0(r2)       ; r1 = mem[0x50]
        la      r2, 0x000053
        sw      r1, 0(r2)       ; mem[0x53] = 35
        ; Copy to 0x56
        lw      r0, 0(r2)       ; r0 = mem[0x53]
        la      r2, 0x000056
        sw      r0, 0(r2)       ; mem[0x56] = 35
        ; Cleanup
        mov     sp, fp
        pop     fp
        halt"#.to_string(),
            machine_code_hex: "6a4c 440a 4519 01 2b50 0000 8a00 9300\n\
                              2b53 0000 8b00 9200 2b56 0000 8a00 53 76\n\
                              c700 0000".to_string(),
            machine_code_size: 38,
            listing: r#"0000: 6A           push    fp
0001: 4C           mov     fp, sp
0002: 44 0A        lc      r0, 10
0004: 45 19        lc      r1, 25
0006: 01           add     r0, r1
0007: 2A 50 00 00  la      r2, 0x000050
000B: 8A 00        sw      r0, 0(r2)
000D: 93 00        lw      r1, 0(r2)
000F: 2A 53 00 00  la      r2, 0x000053
0013: 8B 00        sw      r1, 0(r2)
0015: 92 00        lw      r0, 0(r2)
0017: 2A 56 00 00  la      r2, 0x000056
001B: 8A 00        sw      r0, 0(r2)
001D: 53           mov     sp, fp
001E: 76           pop     fp
001F: C7 00 00 00  halt"#.to_string(),
        },
    ]
}
