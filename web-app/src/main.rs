#![allow(non_snake_case)]
use dioxus::{prelude::*, };
use embedded_web_ui::{ Command, UI, WidgetKind, Widget};
mod components;
use components::*;
fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    console_error_panic_hook::set_once();

    dioxus_web::launch(app);
}

fn app(cx: Scope) -> Element {
    let ui_items = use_state(cx, || {
        let res: Vec<UI> = vec![
            UI::Widget(Widget{kind: WidgetKind::Button, label: "heyoo".into(), id: 1}),
            UI::Break,
            UI::Widget(Widget { kind: WidgetKind::Slider, label: "slidos".into(), id: 2 }),
            UI::Break,
            UI::Widget(Widget { kind: WidgetKind::BarChart, label: "a bar chart".into(), id: 3 }),
            UI::Break,
            UI::Widget(Widget { kind: WidgetKind::BarChart, label: "another bar chart".into(), id: 4 }),
            ];
        res
    });
    let slider_vars = use_state(&cx, SliderVars::default);

    cx.render(rsx! (
        div {
            style: "text-align: center;",
            h1 { "🌗 Dioxus 🚀" }
            h3 { "Frontend that scales." }
            p { "Dioxus is a portable, performant, and ergonomic framework for building cross-platform user interfaces in Rust." }
            div {
                ui_items.iter().map(|item| match item {
                    UI::Widget(widget) => {
                        let label = &widget.label;
                        let id = widget.id.to_string();
                        match widget.kind {
                            WidgetKind::TimeSeriesChart => rsx!{"TimeSeriesChart"},

                            WidgetKind::BarChart => rsx!(BarChart {
                                id: format!("bar-chart-{id}"),
                                name: label.to_string()
                            }),

                            WidgetKind::Button => rsx!( button {
                                //key: "{id}",
                                onclick: move |ev| {
                                    to_owned!(id);
                                    log::debug!("click {id}");
                                    ev.stop_propagation();
                                },
                                "{label}"}),
                            WidgetKind::Slider => rsx!(
                                UiSlider {
                                    //key: "{id}",
                                    name: label.to_string(),
                                    vars: slider_vars.clone()
                                }
                            ),
                        }
                    },
                    UI::Break => rsx!{ br{} },
                    _ => rsx!{"TODO"}
                })
            }     
        }
    ))
}
