//! Shared debug panel component — registers, emulator state, I/O, and memory
//! Used by both the Assembler tab and the Rust Pipeline tab.

use wasm_bindgen::JsCast;
use web_sys::KeyboardEvent;
use yew::prelude::*;

use crate::EmulatorState;

/// Format UART output as hex dump with ASCII sidebar (like od/hexdump).
/// 8 bytes per line: "48 65 6C 6C 6F 20 57 6F  Hello Wo"
fn format_uart_output(s: &str) -> String {
    if s.is_empty() {
        return String::new();
    }
    let bytes: Vec<u8> = s.bytes().collect();
    let mut lines = Vec::new();
    for chunk in bytes.chunks(8) {
        let hex: Vec<String> = chunk.iter().map(|b| format!("{:02X}", b)).collect();
        // Pad hex to 8 columns so ASCII column aligns
        let hex_str = if chunk.len() < 8 {
            let padding = "   ".repeat(8 - chunk.len());
            format!("{}{}", hex.join(" "), padding)
        } else {
            hex.join(" ")
        };
        let ascii: String = chunk
            .iter()
            .map(|&b| if (0x20..0x7F).contains(&b) { b as char } else { '.' })
            .collect();
        lines.push(format!("{}  {}", hex_str, ascii));
    }
    lines.join("\n")
}

#[derive(Properties, PartialEq)]
pub struct DebugPanelProps {
    pub cpu_state: EmulatorState,
    pub is_loaded: bool,
    pub is_running: bool,
    pub on_step: Callback<u32>,
    pub on_run: Callback<()>,
    pub on_stop: Callback<()>,
    pub on_reset: Callback<()>,
    pub switch_value: u8,
    pub on_switch_toggle: Callback<u8>,
    /// Send a byte to UART RX (triggers interrupt if enabled)
    pub on_uart_send: Callback<u8>,
    /// Clear UART TX output buffer
    #[prop_or_default]
    pub on_uart_clear: Callback<()>,
    /// Optional scroll container ID for assembly listing auto-scroll
    #[prop_or_default]
    pub listing_scroll_id: Option<String>,
    /// Whether to show the assembly listing column (default: true).
    /// Set to false when assembly is already shown elsewhere (e.g. ProgramArea).
    #[prop_or(true)]
    pub show_listing: bool,
}

#[function_component(DebugPanel)]
pub fn debug_panel(props: &DebugPanelProps) -> Html {
    let state = &props.cpu_state;

    // Step count selector state
    let step_count = use_state(|| 1u32);
    let trace_expanded = use_state(|| false);

    // Track PC for auto-scroll
    let last_pc = use_state(|| 0u32);
    {
        let pc = state.pc;
        let last_pc = last_pc.clone();
        let scroll_id = props.listing_scroll_id.clone().unwrap_or_else(|| "debug-asm-listing-scroll".to_string());
        use_effect_with(pc, move |&current_pc| {
            if current_pc != *last_pc {
                last_pc.set(current_pc);
                if let Some(window) = web_sys::window()
                    && let Some(document) = window.document()
                    && let Some(container) = document.get_element_by_id(&scroll_id)
                    && let Some(element) = container.query_selector(".asm-line.current-line").ok().flatten()
                {
                    let element_html: &web_sys::HtmlElement = element.unchecked_ref();
                    let container_html: &web_sys::HtmlElement = container.unchecked_ref();
                    let element_top = element_html.offset_top();
                    let container_height = container_html.client_height();
                    let container_scroll = container.scroll_top();
                    let element_bottom = element_top + 20;
                    let visible_top = container_scroll;
                    let visible_bottom = container_scroll + container_height;
                    if element_top < visible_top || element_bottom > visible_bottom {
                        let target_scroll = element_top - (container_height / 2);
                        container.set_scroll_top(target_scroll.max(0));
                    }
                }
            }
        });
    }

    // Emulator state
    let (emu_state_label, emu_state_class) = if state.is_halted {
        ("HALTED", "emu-state emu-halted")
    } else if props.is_running {
        ("RUNNING", "emu-state emu-running")
    } else if state.instruction_count > 0 {
        ("PAUSED", "emu-state emu-paused")
    } else {
        ("READY", "emu-state emu-ready")
    };

    // LED display: when running show duty cycle %, when paused show actual hardware state
    let led_is_on = (state.led_value & 1) == 1;
    let duty = state.led_duty_cycle;
    let (led_class, led_status, led_style) = if props.is_running {
        // Running: show duty cycle with proportional opacity
        let class = if duty > 0.0 { "led led-on led-large" } else { "led led-off led-large" };
        let status = if duty > 0.01 && duty < 0.99 {
            format!("{:.0}%", duty * 100.0)
        } else if duty >= 0.99 {
            "ON".to_string()
        } else {
            "OFF".to_string()
        };
        let style = if duty > 0.0 && duty < 1.0 {
            format!("opacity: {:.2};", 0.3 + duty * 0.7)
        } else {
            String::new()
        };
        (class, status, style)
    } else {
        // Paused: show actual binary LED state
        if led_is_on {
            ("led led-on led-large", "ON".to_string(), String::new())
        } else {
            ("led led-off led-large", "OFF".to_string(), String::new())
        }
    };
    let switch_on = (props.switch_value & 1) == 1;
    let switch_class = if switch_on { "switch switch-on switch-large" } else { "switch switch-off switch-large" };
    let switch_status = if switch_on { "PRESSED" } else { "RELEASED" };

    let on_switch_click = {
        let on_switch_toggle = props.on_switch_toggle.clone();
        let current_value = props.switch_value;
        Callback::from(move |_: MouseEvent| {
            on_switch_toggle.emit(current_value ^ 1);
        })
    };

    // Step callbacks
    let on_exec_step = {
        let on_step = props.on_step.clone();
        let step_count = step_count.clone();
        Callback::from(move |_: MouseEvent| on_step.emit(*step_count))
    };

    let on_step_count_change = {
        let step_count = step_count.clone();
        Callback::from(move |e: Event| {
            if let Some(select) = e.target_dyn_into::<web_sys::HtmlSelectElement>()
                && let Ok(n) = select.value().parse::<u32>()
            {
                step_count.set(n);
            }
        })
    };

    let on_run_click = {
        let on_run = props.on_run.clone();
        Callback::from(move |_| on_run.emit(()))
    };

    let on_stop_click = {
        let on_stop = props.on_stop.clone();
        Callback::from(move |_| on_stop.emit(()))
    };

    let on_reset_click = {
        let on_reset = props.on_reset.clone();
        Callback::from(move |_| on_reset.emit(()))
    };

    let scroll_id = props.listing_scroll_id.clone().unwrap_or_else(|| "debug-asm-listing-scroll".to_string());

    let content_class = if props.show_listing { "debug-content" } else { "debug-content debug-content-single" };

    html! {
        <div class="debug-panel">
            // Debug controls
            <div class="debug-controls">
                <span class="debug-controls-label">{"Emulator:"}</span>
                <button class="step-btn" onclick={on_exec_step}
                    disabled={!props.is_loaded || state.is_halted || props.is_running}
                    data-tooltip="Execute instructions (use multiplier to step multiple)">
                    {"Step"}
                </button>
                <select class="step-count-select" onchange={on_step_count_change}
                    disabled={props.is_running}
                    data-tooltip="Instructions per Step click">
                    <option value="1" selected={*step_count == 1}>{"×1"}</option>
                    <option value="10" selected={*step_count == 10}>{"×10"}</option>
                    <option value="100" selected={*step_count == 100}>{"×100"}</option>
                    <option value="1000" selected={*step_count == 1000}>{"×1K"}</option>
                    <option value="10000" selected={*step_count == 10000}>{"×10K"}</option>
                    <option value="100000" selected={*step_count == 100000}>{"×100K"}</option>
                </select>
                if props.is_running {
                    <button class="stop-btn" onclick={on_stop_click}
                        data-tooltip="Stop continuous execution">
                        {"Stop"}
                    </button>
                } else {
                    <button class="run-btn" onclick={on_run_click}
                        disabled={!props.is_loaded || state.is_halted}
                        data-tooltip="Run continuously until halt or stop">
                        {"Run"}
                    </button>
                }
                <button class="reset-btn" onclick={on_reset_click}
                    disabled={!props.is_loaded || props.is_running}
                    data-tooltip="Reset CPU to initial state, clear memory, load program">
                    {"Reset"}
                </button>
            </div>

            // Debug content — two-column with listing, or single-column without
            <div class={content_class}>
                if props.show_listing {
                    // Left: Assembly listing
                    <div class="debug-left">
                        <h4>{"Assembly"}</h4>
                        if props.is_loaded && !state.assembled_lines.is_empty() {
                            <div class="listing-scroll" id={scroll_id}>
                                {for state.assembled_lines.iter().map(|line| {
                                    let is_current = if line.len() > 4 && line.chars().nth(4) == Some(':') {
                                        if let Ok(addr) = u32::from_str_radix(&line[0..4], 16) {
                                            addr == state.pc
                                        } else {
                                            false
                                        }
                                    } else {
                                        false
                                    };
                                    let class = if is_current { "asm-line current-line" } else { "asm-line" };
                                    html! {
                                        <div class={class}>{line}</div>
                                    }
                                })}
                            </div>
                        }
                    </div>
                }

                // Right: Registers, state, I/O, memory
                <div class="debug-right">
                    <h4>{"Registers"}</h4>
                    <div class="register-grid">
                        {for (0..8).map(|i| {
                            let (name, tooltip) = match i {
                                0 => ("r0", "General purpose register 0"),
                                1 => ("r1", "General purpose register 1"),
                                2 => ("r2", "General purpose register 2"),
                                3 => ("fp", "Frame pointer (r3)"),
                                4 => ("sp", "Stack pointer (r4)"),
                                5 => ("z",  "Constant zero (r5)"),
                                6 => ("iv", "Interrupt vector (r6)"),
                                7 => ("ir", "Interrupt return (r7)"),
                                _ => ("??", ""),
                            };
                            let val = state.registers[i];
                            let is_hot = val != state.prev_registers[i];
                            let is_warm = !is_hot && state.prev_registers[i] != state.prev_prev_registers[i];
                            let row_class = if is_hot {
                                "register-entry hot"
                            } else if is_warm {
                                "register-entry warm"
                            } else {
                                "register-entry"
                            };
                            // z is constant zero — display as single nibble "0"
                            let value_str = if i == 5 {
                                "0".to_string()
                            } else {
                                format!("0x{:06X}", val)
                            };
                            // Show decimal for GP registers (r0-r2) only
                            let full_tooltip = if i <= 2 {
                                if val & 0x800000 != 0 {
                                    let signed = val as i32 - 0x1000000;
                                    format!("{tooltip} = {val} or {signed}")
                                } else {
                                    format!("{tooltip} = {val}")
                                }
                            } else {
                                tooltip.to_string()
                            };
                            html! {
                                <div class={row_class} data-tooltip={full_tooltip}>
                                    <span class="reg-name">{name}</span>
                                    <span class="reg-value">{value_str}</span>
                                </div>
                            }
                        })}
                        <div class="register-entry" data-tooltip="Program counter">
                            <span class="reg-name">{"PC"}</span>
                            <span class="reg-value">{format!("0x{:06X}", state.pc)}</span>
                        </div>
                        <div class="register-entry" data-tooltip="Condition flag (set by compare instructions)">
                            <span class="reg-name">{"C"}</span>
                            <span class="reg-value">{if state.condition_flag { "1" } else { "0" }}</span>
                        </div>
                    </div>

                    <div class="emu-status-line">
                        <span class={emu_state_class}>{emu_state_label}</span>
                        <span class="instruction-count">{"Instructions: "}{state.instruction_count}</span>
                    </div>

                    // I/O: Switch, LED, and UART
                    <div class="debug-io">
                        <div class="peripheral-section-inline">
                            <div class={switch_class} onclick={on_switch_click} data-tooltip="Click to toggle button S2">
                                {"S2"}
                            </div>
                            <span class="io-inline-status">{switch_status}</span>
                        </div>
                        <div class="peripheral-section-inline">
                            <div class={led_class} style={led_style}>{"D2"}</div>
                            <span class="io-inline-status">{led_status}</span>
                        </div>
                        <div class="uart-panel" data-tooltip="UART I/O (0xFF0100). Click input to type characters.">
                            <div class="uart-panel-row">
                                <span class="uart-label">{"RX:"}</span>
                                <input type="text"
                                    class="uart-input"
                                    placeholder="type here..."
                                    readonly=true
                                    onkeydown={
                                        let on_uart_send = props.on_uart_send.clone();
                                        Callback::from(move |e: KeyboardEvent| {
                                            e.prevent_default();
                                            let key = e.key();
                                            let byte: Option<u8> = if key.len() == 1 {
                                                Some(key.as_bytes()[0])
                                            } else if key == "Enter" {
                                                Some(0x0A)
                                            } else {
                                                None
                                            };
                                            if let Some(b) = byte {
                                                on_uart_send.emit(b);
                                            }
                                        })
                                    }
                                />
                            </div>
                            <div class="uart-panel-row">
                                <span class="uart-label">{"TX:"}</span>
                                <span class="uart-output">{
                                    if state.uart_output.is_empty() {
                                        "\u{00a0}".to_string()
                                    } else {
                                        format_uart_output(&state.uart_output)
                                    }
                                }</span>
                                if !state.uart_output.is_empty() {
                                    <button class="uart-clear-btn"
                                        data-tooltip="Clear UART output"
                                        onclick={
                                            let cb = props.on_uart_clear.clone();
                                            Callback::from(move |_| cb.emit(()))
                                        }
                                    >{"×"}</button>
                                }
                            </div>
                        </div>
                    </div>

                    // Instruction trace - collapsible
                    if !state.trace_lines.is_empty() {
                        <div class="trace-section">
                            <div class="trace-header" onclick={
                                let trace_expanded = trace_expanded.clone();
                                Callback::from(move |_| trace_expanded.set(!*trace_expanded))
                            }>
                                <span class="trace-toggle">{if *trace_expanded { "\u{25BC}" } else { "\u{25B6}" }}</span>
                                <span>{format!("Instruction Trace ({} entries)", state.trace_lines.len())}</span>
                            </div>
                            if *trace_expanded {
                                <pre class="trace-content">{
                                    state.trace_lines.join("\n")
                                }</pre>
                            }
                        </div>
                    }

                    // Memory viewer - three regions
                    if props.is_loaded {
                        <div class="memory-section">
                            <h4>{format!("SRAM (0x{:06X}\u{2013}0x{:06X}, code ends 0x{:04X})", state.memory_low.region_start, state.memory_low.region_end - 1, state.program_end)}</h4>
                            <div class="memory-dump-compact">{
                                format_sparse_memory(&state.memory_low, &state.prev_memory_low, &state.prev_prev_memory_low)
                            }</div>
                        </div>
                        <div class="memory-section">
                            <h4>{format!("EBR/Stack (0x{:06X}\u{2013}0x{:06X}, SP=0x{:06X})", state.memory_stack.region_start, state.memory_stack.region_end - 1, state.registers[4])}</h4>
                            <div class="memory-dump-compact">{
                                format_sparse_memory_reversed(&state.memory_stack, &state.prev_memory_stack, &state.prev_prev_memory_stack)
                            }</div>
                        </div>
                        <div class="memory-section">
                            <h4>{"I/O: LED/Switch (0xFF0000)"}</h4>
                            <div class="memory-dump-compact">{
                                format_memory_dump_all(&state.memory_io_led, &state.prev_memory_io_led, &state.prev_prev_memory_io_led, 0xFF0000)
                            }</div>
                        </div>
                        <div class="memory-section">
                            <h4>{"I/O: UART (0xFF0100)"}</h4>
                            <div class="memory-dump-compact">{
                                format_memory_dump_all(&state.memory_io_uart, &state.prev_memory_io_uart, &state.prev_prev_memory_io_uart, 0xFF0100)
                            }</div>
                        </div>
                    }
                </div>
            </div>
        </div>
    }
}

use crate::SparseMemory;

/// Emit a zero-gap summary line for `gap_start..gap_end` (exclusive end).
fn zero_summary(gap_start: u32, gap_end: u32) -> Html {
    let bytes = gap_end - gap_start;
    html! {
        <div class="memory-row memory-zero-block">
            <span class="memory-addr">{format!("{:06X}\u{2013}{:06X}: ", gap_start, gap_end - 1)}</span>
            <span class="mem-byte">{format!("({} bytes zero)", bytes)}</span>
        </div>
    }
}

/// Render a single non-zero memory row with heatmap coloring.
/// `prev` and `prev_prev` are the SparseMemory from prior steps for change detection.
fn sparse_data_row(addr: u32, data: &[u8], prev: &SparseMemory, prev_prev: &SparseMemory) -> Html {
    html! {
        <div class="memory-row">
            <span class="memory-addr">{format!("{:06X}: ", addr)}</span>
            {for data.iter().enumerate().map(|(j, byte)| {
                let a = addr + j as u32;
                let prev_byte = sparse_lookup(prev, a);
                let prev_prev_byte = sparse_lookup(prev_prev, a);
                let is_hot = *byte != prev_byte;
                let is_warm = !is_hot && prev_byte != prev_prev_byte;
                let class = if is_hot {
                    "mem-byte hot"
                } else if is_warm {
                    "mem-byte warm"
                } else {
                    "mem-byte"
                };
                html! {
                    <span class={class}>{format!("{:02X}", byte)}</span>
                }
            })}
        </div>
    }
}

/// Look up a byte in a SparseMemory. Returns 0 if the address is in a zero gap.
fn sparse_lookup(sm: &SparseMemory, addr: u32) -> u8 {
    let row_addr = addr & !0xF;
    // Binary search for the row
    if let Ok(idx) = sm.rows.binary_search_by_key(&row_addr, |&(a, _)| a) {
        let offset = (addr - row_addr) as usize;
        sm.rows[idx].1.get(offset).copied().unwrap_or(0)
    } else {
        0
    }
}

/// Format sparse memory (forward order) with zero-gap summaries.
fn format_sparse_memory(mem: &SparseMemory, prev: &SparseMemory, prev_prev: &SparseMemory) -> Html {
    let mut elements: Vec<Html> = Vec::new();
    let mut cursor = mem.region_start;

    for (addr, data) in &mem.rows {
        if *addr > cursor {
            elements.push(zero_summary(cursor, *addr));
        }
        elements.push(sparse_data_row(*addr, data, prev, prev_prev));
        cursor = *addr + data.len() as u32;
    }

    // Trailing zero gap
    if cursor < mem.region_end {
        elements.push(zero_summary(cursor, mem.region_end));
    }

    html! { <>{for elements.into_iter()}</> }
}

/// Format sparse memory in reverse order (high address first) with zero-gap summaries.
/// Used for stack display where high addresses are at the top.
fn format_sparse_memory_reversed(mem: &SparseMemory, prev: &SparseMemory, prev_prev: &SparseMemory) -> Html {
    let mut elements: Vec<Html> = Vec::new();
    let mut cursor = mem.region_end;

    for (addr, data) in mem.rows.iter().rev() {
        let row_end = *addr + data.len() as u32;
        if cursor > row_end {
            elements.push(zero_summary(row_end, cursor));
        }
        elements.push(sparse_data_row(*addr, data, prev, prev_prev));
        cursor = *addr;
    }

    // Leading zero gap (lowest addresses)
    if cursor > mem.region_start {
        elements.push(zero_summary(mem.region_start, cursor));
    }

    html! { <>{for elements.into_iter()}</> }
}

/// Format all memory rows with heatmap (no zero compression, for small I/O regions)
fn format_memory_dump_all(data: &[u8], prev: &[u8], prev_prev: &[u8], base_addr: u32) -> Html {
    html! {
        <>
            {for data.chunks(16).enumerate().map(|(i, chunk)| {
                let addr = base_addr + (i * 16) as u32;
                html! {
                    <div class="memory-row">
                        <span class="memory-addr">{format!("{:06X}: ", addr)}</span>
                        {for chunk.iter().enumerate().map(|(j, byte)| {
                            let idx = i * 16 + j;
                            let prev_byte = prev.get(idx).copied().unwrap_or(0);
                            let prev_prev_byte = prev_prev.get(idx).copied().unwrap_or(0);
                            let is_hot = *byte != prev_byte;
                            let is_warm = !is_hot && prev_byte != prev_prev_byte;
                            let class = if is_hot {
                                "mem-byte hot"
                            } else if is_warm {
                                "mem-byte warm"
                            } else {
                                "mem-byte"
                            };
                            html! {
                                <span class={class}>{format!("{:02X}", byte)}</span>
                            }
                        })}
                    </div>
                }
            })}
        </>
    }
}
