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
    pub instruction_count: u32,
    pub memory_low: Vec<u8>,
    pub memory_io_led: Vec<u8>,         // I/O: LED/Switch at 0xFF0000 (32 bytes)
    pub memory_io_uart: Vec<u8>,        // I/O: UART at 0xFF0100 (16 bytes)
    pub memory_stack: Vec<u8>,          // Stack region around SP
    pub stack_base_addr: u32,           // Base address of stack region
    pub program_end: u32,               // End of code+data region
    pub prev_memory_low: Vec<u8>,
    pub prev_memory_io_led: Vec<u8>,
    pub prev_memory_io_uart: Vec<u8>,
    pub prev_memory_stack: Vec<u8>,
    pub prev_prev_memory_low: Vec<u8>,
    pub prev_prev_memory_io_led: Vec<u8>,
    pub prev_prev_memory_io_uart: Vec<u8>,
    pub prev_prev_memory_stack: Vec<u8>,
    pub current_instruction: String,
    pub assembled_lines: Vec<String>,
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
    pub on_load: Callback<RustExample>,
    pub on_step: Callback<u32>,
    pub on_run: Callback<()>,
    pub on_stop: Callback<()>,
    pub on_reset: Callback<()>,
    #[prop_or_default]
    pub on_unload: Callback<()>,
    pub cpu_state: EmulatorState,
    pub is_loaded: bool,
    pub is_running: bool,
    pub switch_value: u8,
    pub on_switch_toggle: Callback<u8>,
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

    // Load dialog state
    let load_dialog_open = use_state(|| false);

    // Selected example state - None means "Choose an example..." is shown
    let selected_example = use_state(|| None::<RustExample>);

    // Example selector callback
    let on_example_select = {
        let selected_example = selected_example.clone();
        let examples = props.examples.clone();
        Callback::from(move |e: Event| {
            let target = e.target_dyn_into::<web_sys::HtmlSelectElement>();
            if let Some(select) = target {
                let idx = select.selected_index();
                // Index 0 is "Choose an example...", so subtract 1 for actual examples
                if idx > 0
                    && let Some(example) = examples.get((idx - 1) as usize)
                {
                    selected_example.set(Some(example.clone()));
                }
            }
        })
    };

    // Open load dialog
    let on_load_dialog_open = {
        let load_dialog_open = load_dialog_open.clone();
        Callback::from(move |_| {
            load_dialog_open.set(true);
        })
    };

    // Close load dialog
    let on_load_dialog_close = {
        let load_dialog_open = load_dialog_open.clone();
        Callback::from(move |_| {
            load_dialog_open.set(false);
        })
    };

    // Load example callback - resets to Source step and closes dialog
    let on_load_click = {
        let on_load = props.on_load.clone();
        let selected = selected_example.clone();
        let current_step = current_step.clone();
        let load_dialog_open = load_dialog_open.clone();
        Callback::from(move |_| {
            if let Some(example) = &*selected {
                on_load.emit(example.clone());
                current_step.set(WizardStep::Source);
                load_dialog_open.set(false);
                // Scroll notebook to top to show source cell
                gloo::timers::callback::Timeout::new(50, || {
                    if let Some(window) = web_sys::window()
                        && let Some(document) = window.document()
                        && let Some(container) = document.get_element_by_id("notebook-scroll")
                    {
                        container.set_scroll_top(0);
                    }
                }).forget();
            }
        })
    };

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

    // Reset wizard to Source step (always enabled) - also unloads the example
    let on_wizard_reset = {
        let current_step = current_step.clone();
        let on_unload = props.on_unload.clone();
        let selected_example = selected_example.clone();
        Callback::from(move |_| {
            current_step.set(WizardStep::Source);
            selected_example.set(None);
            on_unload.emit(());
        })
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
                    <button onclick={
                        let cb = props.on_tutorial_open.clone();
                        Callback::from(move |_| cb.emit(()))
                    }>{"Tutorial"}</button>
                    <button onclick={
                        let cb = props.on_examples_open.clone();
                        Callback::from(move |_| cb.emit(()))
                    }>{"Examples"}</button>
                    <button onclick={
                        let cb = props.on_isa_ref_open.clone();
                        Callback::from(move |_| cb.emit(()))
                    }>{"ISA Ref"}</button>
                    <button onclick={
                        let cb = props.on_help_open.clone();
                        Callback::from(move |_| cb.emit(()))
                    }>{"Help"}</button>
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
                        >
                            <span class="step-indicator">{indicator}</span>
                            <span class="step-label">{step.label()}</span>
                        </div>
                    }
                })}

                // Action button - Load when not loaded, then Compile/Disassemble/etc.
                if !props.is_loaded {
                    <button class="wizard-action-btn" onclick={on_load_dialog_open.clone()} disabled={props.is_running}>
                        {"Load"}
                    </button>
                } else if *current_step != WizardStep::Assemble {
                    <button class="wizard-action-btn" onclick={advance_step.clone()}>
                        {(*current_step).action_label()}
                    </button>
                }

                // Spacer to push Exit button to bottom
                <div class="wizard-spacer"></div>

                // Exit button - unloads example and returns to Source step
                if props.is_loaded {
                    <button class="wizard-exit-btn" onclick={on_wizard_reset}>
                        {"Exit"}
                    </button>
                }
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
            if *load_dialog_open {
                <div class="wizard-dialog-overlay" onclick={on_load_dialog_close.clone()}>
                    <div class="wizard-dialog" onclick={Callback::from(|e: MouseEvent| e.stop_propagation())}>
                        <div class="wizard-dialog-header">
                            <h3>{"Load Example"}</h3>
                            <button class="wizard-dialog-close" onclick={on_load_dialog_close.clone()}>{"×"}</button>
                        </div>
                        <div class="wizard-dialog-body">
                            <label>{"Select an example:"}</label>
                            <select class="wizard-dialog-select" onchange={on_example_select} disabled={props.is_running}>
                                <option value="" selected={selected_example.is_none()} disabled={true}>
                                    {"Choose an example..."}
                                </option>
                                {for props.examples.iter().map(|ex| {
                                    let is_selected = match &*selected_example {
                                        Some(sel) => sel.name == ex.name,
                                        None => false,
                                    };
                                    html! {
                                        <option value={ex.name.clone()} selected={is_selected}>
                                            {&ex.name}{" - "}{&ex.description}
                                        </option>
                                    }
                                })}
                            </select>
                        </div>
                        <div class="wizard-dialog-footer">
                            <button class="wizard-dialog-cancel" onclick={on_load_dialog_close}>{"Cancel"}</button>
                            <button class="wizard-dialog-load" onclick={on_load_click} disabled={props.is_running || selected_example.is_none()}>{"Load"}</button>
                        </div>
                    </div>
                </div>
            }
        </div>
    }
}
