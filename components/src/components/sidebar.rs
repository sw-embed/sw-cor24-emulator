use yew::prelude::*;

#[derive(Clone, PartialEq)]
pub struct SidebarButton {
    pub label: String,
    pub onclick: Callback<MouseEvent>,
    pub title: Option<String>,
    /// If set, renders as an external link (<a>) instead of a button
    pub href: Option<String>,
}

#[derive(Properties, PartialEq)]
pub struct SidebarProps {
    pub buttons: Vec<SidebarButton>,
}

#[function_component(Sidebar)]
pub fn sidebar(props: &SidebarProps) -> Html {
    html! {
        <div class="sidebar">
            { for props.buttons.iter().map(|btn| {
                let title = btn.title.clone().unwrap_or_else(|| btn.label.clone());

                if let Some(href) = &btn.href {
                    html! {
                        <a
                            href={href.clone()}
                            target="_blank"
                            rel="noopener"
                            class="sidebar-link"
                            data-tooltip={title}
                        >
                            {&btn.label}
                            <span class="ext-icon">{" \u{2197}"}</span>
                        </a>
                    }
                } else {
                    let onclick = btn.onclick.clone();
                    html! {
                        <button
                            {onclick}
                            data-tooltip={title}
                        >
                            {&btn.label}
                        </button>
                    }
                }
            })}
        </div>
    }
}
