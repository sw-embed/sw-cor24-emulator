//! Rust Pipeline view component - Wizard-driven 3-column layout
//! Shows the compilation pipeline: Rust -> MSP430 ASM -> COR24 ASM -> Machine Code -> Execution

use wasm_bindgen::JsCast;
use yew::prelude::*;

use crate::DebugPanel;

/// Pre-built example for the Rust pipeline demo
#[derive(Clone, PartialEq)]
pub struct RustExample {
    pub name: String,
    pub description: String,
    pub rust_source: String,
    pub msp430_asm: String,
    pub cor24_assembly: String,
}

/// Sparse memory representation: only non-zero 16-byte rows within a region.
/// The display renders summary lines for zero gaps between non-zero rows.
#[derive(Clone, PartialEq, Default)]
pub struct SparseMemory {
    pub region_start: u32,  // First address of the region
    pub region_end: u32,    // Last address + 1 (exclusive)
    /// Non-zero rows: (row_start_address, 16 bytes). Sorted by address.
    pub rows: Vec<(u32, Vec<u8>)>,
}

impl SparseMemory {
    /// Build from a byte slice representing memory at `base_addr..base_addr+data.len()`.
    pub fn from_slice(data: &[u8], base_addr: u32) -> Self {
        let mut rows = Vec::new();
        for (i, chunk) in data.chunks(16).enumerate() {
            if chunk.iter().any(|&b| b != 0) {
                rows.push((base_addr + (i * 16) as u32, chunk.to_vec()));
            }
        }
        SparseMemory {
            region_start: base_addr,
            region_end: base_addr + data.len() as u32,
            rows,
        }
    }
}

/// CPU state for display in the Rust pipeline execution panel
#[derive(Clone, PartialEq, Default)]
pub struct EmulatorState {
    pub registers: [u32; 8],
    pub prev_registers: [u32; 8],      // Changed last step (hot)
    pub prev_prev_registers: [u32; 8], // Changed 2 steps ago (warm)
    pub pc: u32,
    pub condition_flag: bool,
    pub is_halted: bool,
    pub led_value: u8,
    pub led_duty_cycle: f32,  // 0.0 (always off) to 1.0 (always on) over Run
    pub led_on_count: u64,    // Instructions executed with LED on (cumulative during Run)
    pub instruction_count: u32,
    pub memory_low: SparseMemory,       // SRAM (0x000000-0x0FFFFF)
    pub memory_io_led: Vec<u8>,         // I/O: LED/Switch at 0xFF0000 (16 bytes)
    pub memory_io_uart: Vec<u8>,        // I/O: UART at 0xFF0100 (16 bytes)
    pub memory_stack: SparseMemory,     // EBR/Stack (0xFEE000-0xFEEC00)
    pub program_end: u32,               // End of code+data region
    pub prev_memory_low: SparseMemory,
    pub prev_memory_io_led: Vec<u8>,
    pub prev_memory_io_uart: Vec<u8>,
    pub prev_memory_stack: SparseMemory,
    pub prev_prev_memory_low: SparseMemory,
    pub prev_prev_memory_io_led: Vec<u8>,
    pub prev_prev_memory_io_uart: Vec<u8>,
    pub prev_prev_memory_stack: SparseMemory,
    pub current_instruction: String,
    pub assembled_lines: Vec<String>,
    pub uart_output: String,
}

/// Wizard steps for progressive disclosure
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WizardStep {
    Source,    // Rust source code
    Compile,   // MSP430 assembly output
    Translate, // COR24 assembly output
    Assemble,  // Assembled + debugger
}

impl WizardStep {
    fn label(&self) -> &'static str {
        match self {
            WizardStep::Source => "Source",
            WizardStep::Compile => "Compile",
            WizardStep::Translate => "Translate",
            WizardStep::Assemble => "Assemble",
        }
    }

    fn next(&self) -> Option<WizardStep> {
        match self {
            WizardStep::Source => Some(WizardStep::Compile),
            WizardStep::Compile => Some(WizardStep::Translate),
            WizardStep::Translate => Some(WizardStep::Assemble),
            WizardStep::Assemble => None,
        }
    }

    fn action_label(&self) -> &'static str {
        match self {
            WizardStep::Source => "Compile",
            WizardStep::Compile => "Translate",
            WizardStep::Translate => "Assemble",
            WizardStep::Assemble => "",
        }
    }

    fn action_tooltip(&self) -> &'static str {
        match self {
            WizardStep::Source => "Compile Rust to MSP430 assembly",
            WizardStep::Compile => "Translate MSP430 assembly to COR24 assembly",
            WizardStep::Translate => "Assemble COR24 code into machine code",
            WizardStep::Assemble => "",
        }
    }

    fn step_tooltip(&self) -> &'static str {
        match self {
            WizardStep::Source => "Rust source code",
            WizardStep::Compile => "MSP430 assembly (from rustc)",
            WizardStep::Translate => "COR24 assembly (from MSP430)",
            WizardStep::Assemble => "Machine code execution and debug",
        }
    }

    fn cell_id(&self) -> &'static str {
        match self {
            WizardStep::Source => "cell-source",
            WizardStep::Compile => "cell-msp430",
            WizardStep::Translate => "cell-cor24",
            WizardStep::Assemble => "cell-debug",
        }
    }
}

#[derive(Properties, PartialEq)]
pub struct RustPipelineProps {
    pub examples: Vec<RustExample>,
    #[prop_or_default]
    pub loaded_example: Option<RustExample>,
    #[prop_or_default]
    pub load_generation: u32,
    pub on_load: Callback<RustExample>,
    pub on_step: Callback<u32>,
    pub on_run: Callback<()>,
    pub on_stop: Callback<()>,
    pub on_reset: Callback<()>,
    pub cpu_state: EmulatorState,
    pub is_loaded: bool,
    pub is_running: bool,
    pub switch_value: u8,
    pub on_switch_toggle: Callback<u8>,
    pub on_uart_send: Callback<u8>,
    #[prop_or_default]
    pub on_uart_clear: Callback<()>,
    // Modal callbacks - passed from app.rs
    #[prop_or_default]
    pub on_tutorial_open: Callback<()>,
    #[prop_or_default]
    pub on_examples_open: Callback<()>,
    #[prop_or_default]
    pub on_isa_ref_open: Callback<()>,
    #[prop_or_default]
    pub on_help_open: Callback<()>,
}

#[function_component(RustPipeline)]
pub fn rust_pipeline(props: &RustPipelineProps) -> Html {
    // Wizard step state
    let current_step = use_state(|| WizardStep::Source);

    // Auto-load "Blink LED" on first render
    {
        let on_load = props.on_load.clone();
        let examples = props.examples.clone();
        use_effect_with((), move |_| {
            if let Some(blink) = examples.iter().find(|e| e.name == "Blink LED") {
                on_load.emit(blink.clone());
            }
            || ()
        });
    }

    // Reset wizard to Source step when an example is loaded (including re-selecting same one)
    {
        let current_step = current_step.clone();
        let load_gen = props.load_generation;
        use_effect_with(load_gen, move |_| {
            current_step.set(WizardStep::Source);
            || ()
        });
    }

    // Advance to next wizard step with scroll
    let advance_step = {
        let current_step = current_step.clone();
        Callback::from(move |_| {
            let current = *current_step;
            if let Some(next) = current.next() {
                current_step.set(next);
                // Determine which cell to scroll to:
                // - For Assemble step: scroll to Execution cell (the new cell)
                // - For other steps: scroll to keep previous cell (N-1) at top
                let scroll_to_cell = if next == WizardStep::Assemble {
                    next.cell_id().to_string() // Scroll to Execution
                } else {
                    current.cell_id().to_string() // Scroll to previous cell (N-1)
                };

                gloo::timers::callback::Timeout::new(100, move || {
                    if let Some(window) = web_sys::window()
                        && let Some(document) = window.document()
                        && let Some(container) = document.get_element_by_id("notebook-scroll")
                        && let Some(element) = document.get_element_by_id(&scroll_to_cell)
                    {
                        let element_html: &web_sys::HtmlElement = element.unchecked_ref();
                        container.set_scroll_top(element_html.offset_top());
                    }
                }).forget();
            }
        })
    };

    // Click on a wizard step to jump to it
    let on_step_click = {
        let current_step = current_step.clone();
        move |step: WizardStep| {
            let current_step = current_step.clone();
            Callback::from(move |_| {
                if step <= *current_step {
                    // Scroll to the cell - align at top
                    let cell_id = step.cell_id().to_string();
                    if let Some(window) = web_sys::window()
                        && let Some(document) = window.document()
                        && let Some(container) = document.get_element_by_id("notebook-scroll")
                        && let Some(element) = document.get_element_by_id(&cell_id)
                    {
                        let element_html: &web_sys::HtmlElement = element.unchecked_ref();
                        container.set_scroll_top(element_html.offset_top());
                    }
                }
            })
        }
    };


    // All wizard steps for rendering
    let all_steps = [
        WizardStep::Source,
        WizardStep::Compile,
        WizardStep::Translate,
        WizardStep::Assemble,
    ];

    html! {
        <div class="rust-wizard-layout">
            // Column 1: Sidebar with buttons and peripherals
            <div class="wizard-sidebar">
                <div class="wizard-buttons">
                    <button data-tooltip="Step-by-step guide to the Rust pipeline" onclick={
                        let cb = props.on_tutorial_open.clone();
                        Callback::from(move |_| cb.emit(()))
                    }>{"Tutorial"}</button>
                    <button data-tooltip="Load a pre-built Rust example" onclick={
                        let cb = props.on_examples_open.clone();
                        Callback::from(move |_| cb.emit(()))
                    }>{"Examples"}</button>
                    <button data-tooltip="COR24 instruction set reference" onclick={
                        let cb = props.on_isa_ref_open.clone();
                        Callback::from(move |_| cb.emit(()))
                    }>{"ISA Ref"}</button>
                    <button data-tooltip="Help and keyboard shortcuts" onclick={
                        let cb = props.on_help_open.clone();
                        Callback::from(move |_| cb.emit(()))
                    }>{"Help"}</button>
                    <a href="https://software-wrighter-lab.github.io/" target="_blank" rel="noopener"
                       class="sidebar-link" data-tooltip="SW-Lab Blog">{"Blog"}</a>
                    <a href="https://discord.com/invite/Ctzk5uHggZ" target="_blank" rel="noopener"
                       class="sidebar-link" data-tooltip="SW-Lab Discord">{"Discord"}</a>
                    <a href="https://www.makerlisp.com/" target="_blank" rel="noopener"
                       class="sidebar-link" data-tooltip="MakerLisp">{"MakerLisp"}</a>
                </div>

                // LED and Switch are shown in the DebugPanel
            </div>

            // Column 2: Wizard steps
            <div class="wizard-steps">
                <div class="wizard-header-space"></div>

                {for all_steps.iter().map(|&step| {
                    // Step is completed if we're at or past it (and an example is loaded)
                    let is_completed = props.is_loaded && step <= *current_step;
                    let is_active = step == *current_step;
                    let is_disabled = step > *current_step;

                    let class = classes!(
                        "wizard-step",
                        is_active.then_some("active"),
                        is_completed.then_some("completed"),
                        is_disabled.then_some("disabled"),
                    );

                    let indicator = if is_completed { "✓" } else { "○" };

                    html! {
                        <div
                            class={class}
                            onclick={if !is_disabled { Some(on_step_click(step)) } else { None }}
                            data-tooltip={step.step_tooltip()}
                        >
                            <span class="step-indicator">{indicator}</span>
                            <span class="step-label">{step.label()}</span>
                        </div>
                    }
                })}

                // Action button - Compile/Translate/Assemble (or nothing at final step)
                if props.is_loaded && *current_step != WizardStep::Assemble {
                    <button class="wizard-action-btn" onclick={advance_step.clone()}
                        data-tooltip={(*current_step).action_tooltip()}>
                        {(*current_step).action_label()}
                    </button>
                }

                <div class="wizard-spacer"></div>
            </div>

            // Column 3: Notebook cells
            <div class="notebook-cells" id="notebook-scroll">
                if let Some(example) = &props.loaded_example {
                    // Cell 1: Rust Source (always visible)
                    <div class="notebook-cell" id="cell-source">
                        <div class="cell-header">
                            <span>{"Rust Source"}</span>
                            <span class="cell-header-example">{&example.name}</span>
                        </div>
                        <div class="cell-content">
                            <pre class="code-block rust-code">{&example.rust_source}</pre>
                        </div>
                    </div>

                    // Cell 2: MSP430 Assembly (visible after Compile step)
                    if *current_step >= WizardStep::Compile {
                        <div class="notebook-cell" id="cell-msp430">
                            <div class="cell-header">
                                <span>{"MSP430 Assembly"}</span>
                                <span class="cell-header-example">{&example.name}</span>
                            </div>
                            <div class="cell-content">
                                <pre class="code-block asm-code">{&example.msp430_asm}</pre>
                            </div>
                        </div>
                    }

                    // Cell 3: COR24 Assembly (visible after Translate step)
                    if *current_step >= WizardStep::Translate {
                        <div class="notebook-cell" id="cell-cor24">
                            <div class="cell-header">
                                <span>{"COR24 Assembly"}</span>
                                <span class="cell-header-example">{&example.name}</span>
                            </div>
                            <div class="cell-content">
                                <pre class="code-block asm-code">{&example.cor24_assembly}</pre>
                            </div>
                        </div>
                    }

                    // Cell 5: Debug Panel (visible after Assemble step)
                    if *current_step >= WizardStep::Assemble {
                        <div class="notebook-cell notebook-cell-debug" id="cell-debug">
                            <div class="cell-header">
                                <span>{"Execution"}</span>
                                <span class="cell-header-example">{&example.name}</span>
                            </div>
                            <DebugPanel
                                cpu_state={props.cpu_state.clone()}
                                is_loaded={props.is_loaded}
                                is_running={props.is_running}
                                on_step={props.on_step.clone()}
                                on_run={props.on_run.clone()}
                                on_stop={props.on_stop.clone()}
                                on_reset={props.on_reset.clone()}
                                switch_value={props.switch_value}
                                on_switch_toggle={props.on_switch_toggle.clone()}
                                on_uart_send={props.on_uart_send.clone()}
                                on_uart_clear={props.on_uart_clear.clone()}
                                listing_scroll_id={"asm-listing-scroll".to_string()}
                            />
                        </div>
                    }
                } else {
                    // No example loaded - show placeholder
                    <div class="notebook-placeholder">
                        <p>{"Click "}<strong>{"Load Example"}</strong>{" to get started."}</p>
                    </div>
                }

                // Pipeline note
                <div class="pipeline-note">
                    <em>{"Pipeline: Rust \u{2192} rustc (msp430-none-elf) \u{2192} MSP430 ASM \u{2192} COR24 ASM \u{2192} Machine Code. Examples are pre-built."}</em>
                </div>
            </div>

            // Load Example Dialog
        </div>
    }
}
