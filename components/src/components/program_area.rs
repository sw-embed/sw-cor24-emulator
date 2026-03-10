use web_sys::HtmlTextAreaElement;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct ProgramAreaProps {
    pub on_assemble: Callback<String>,
    pub on_step: Callback<()>,
    pub on_run: Callback<()>,
    pub on_stop: Callback<()>,
    pub on_reset: Callback<()>,
    pub assembly_output: Option<Html>,
    pub initial_code: Option<String>,
    pub step_enabled: bool,
    pub run_enabled: bool,
    #[prop_or(false)]
    pub is_running: bool,
    /// Hide Step/Run/Reset buttons when DebugPanel provides them (default: true)
    #[prop_or(true)]
    pub show_exec_buttons: bool,
}

#[function_component(ProgramArea)]
pub fn program_area(props: &ProgramAreaProps) -> Html {
    let editor_ref = use_node_ref();
    let code = use_state(|| props.initial_code.clone().unwrap_or_default());

    // Update code when initial_code prop changes
    {
        let code = code.clone();
        let initial_code = props.initial_code.clone();
        use_effect_with(initial_code.clone(), move |_| {
            if let Some(new_code) = initial_code {
                code.set(new_code);
            }
            || ()
        });
    }

    let on_assemble_click = {
        let editor_ref = editor_ref.clone();
        let on_assemble = props.on_assemble.clone();
        let code = code.clone();

        Callback::from(move |_: MouseEvent| {
            if let Some(textarea) = editor_ref.cast::<HtmlTextAreaElement>() {
                let value = textarea.value();
                code.set(value.clone());
                on_assemble.emit(value);
            }
        })
    };

    let on_step_click = {
        let on_step = props.on_step.clone();
        Callback::from(move |_: MouseEvent| {
            on_step.emit(());
        })
    };

    let on_run_click = {
        let on_run = props.on_run.clone();
        Callback::from(move |_: MouseEvent| {
            on_run.emit(());
        })
    };

    let on_stop_click = {
        let on_stop = props.on_stop.clone();
        Callback::from(move |_: MouseEvent| {
            on_stop.emit(());
        })
    };

    let on_reset_click = {
        let on_reset = props.on_reset.clone();
        Callback::from(move |_: MouseEvent| {
            on_reset.emit(());
        })
    };

    html! {
        <div class="program-area">
            <div class="panel-title">{"Program Editor"}</div>

            // Source editor (top half)
            <textarea
                ref={editor_ref}
                id="programEditor"
                placeholder="Enter assembly code here..."
                value={(*code).clone()}
            />

            // Controls (middle)
            <div class="controls">
                <button id="assembleBtn" onclick={on_assemble_click}>{"Assemble"}</button>
                if props.show_exec_buttons {
                    <button id="stepBtn" onclick={on_step_click} disabled={!props.step_enabled || props.is_running}>{"Step"}</button>
                    if props.is_running {
                        <button id="stopBtn" class="stop-btn" onclick={on_stop_click}>{"Stop"}</button>
                    } else {
                        <button id="runBtn" onclick={on_run_click} disabled={!props.run_enabled}>{"Run"}</button>
                    }
                    <button id="resetBtn" onclick={on_reset_click} disabled={props.is_running}>{"Reset"}</button>
                }
            </div>

            // Assembly output (bottom half)
            if let Some(output) = &props.assembly_output {
                <div id="assemblyOutput" style="display: block;">
                    {output.clone()}
                </div>
            }
        </div>
    }
}
