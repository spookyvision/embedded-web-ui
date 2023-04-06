#![allow(non_snake_case)]

use dioxus::{prelude::*, };
use dioxus_websocket_hooks::{use_ws_context_provider, Message, use_ws_context};
use embedded_web_ui::{ Command, UI, WidgetKind, Widget, Input, Id};
use futures::stream::StreamExt;
mod components;
use components::*;
use gloo::timers::future::TimeoutFuture;
use im_rc::HashSet;
use log::{debug, error, info};
use wasm_bindgen::prelude::wasm_bindgen;
fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    console_error_panic_hook::set_once();

    dioxus_web::launch(app);
}


#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen]
    fn btoa(s: &str) -> String;
}


#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen]
    fn init_chart(id: Id);
}


#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen]
    fn update(id: Id, data: Vec<u8>);
}


fn use_ws_context_provider_binary(
    cx: &ScopeState,
    url: &str,
    handler: impl Fn(Vec<u8>) + 'static,
) {
    let handler = move |msg| {
        if let Message::Bytes(data) = msg {
            handler(data)
        } else {
            debug!("no handler for {msg:?}");
        }
    };
    
    use_ws_context_provider(cx, url, handler)
}


fn app(cx: Scope) -> Element {
    debug!("hey");
    let res = unsafe {
         btoa("awerawreawre")
    };
    debug!("hey {res}");
    let slider_vars = use_state(&cx, SliderVars::default);


    // TODO use im_rc
    let ui_items = use_state(cx, || {
        let res: Vec<UI> = vec![
            UI::Widget(Widget{kind: WidgetKind::Button, label: "plz refresh".into(), id: 0}),
            ];
        res
    });


    let update_ui = use_coroutine(
        &cx,
        |mut rx: UnboundedReceiver<UI>| {
            to_owned![ui_items];
            async move {
                while let Some(ui_elem) = rx.next().await {
                    debug!("got new UI! {ui_elem:?}");
                    ui_items.modify(|items| {
                        let mut items = items.clone();
                        items.push(ui_elem);
                        items
                    });
                }
            }
        },
    )
    .to_owned();
    
    let ws = use_ws_context_provider_binary(cx, "ws://localhost:3030", {
        to_owned![ui_items]; 
        move |mut d|  {
            if let Ok(commands) = postcard::from_bytes_cobs::<Vec<Command>>(d.as_mut_slice()) {
                for command in commands {
                    match command {
                        Command::Log => todo!(),
                        Command::Reset => {
                            info!("RESET");
                            ui_items.modify(|_| vec![]);
                        },
                        Command::UI(ui) => {
                            update_ui.send(ui)
                        },
                        Command::TimeSeriesData(_) => todo!(),
                        Command::BarData(data) => {
                            let id = data.id;
                            init_chart(data.id);
                            debug!("got bar data {data:?}");
                            let mut res = vec![];
                            res.extend(&data.vals);
                            update(data.id, res);
                        },
                    }
                } 
            }
            }
        });
    let ws_cx = use_ws_context(&cx);

    cx.render(rsx! (
        div {
            style: "text-align: center;",
            h1 { "embedded-web-ui" }
            div {
                ui_items.iter().map(|item| match item {
                    UI::Widget(widget) => {
                        to_owned![ws_cx];
                        let label = &widget.label;
                        let id = widget.id;
                        match widget.kind {
                            WidgetKind::TimeSeriesChart => rsx!{"TimeSeriesChart"},

                            WidgetKind::BarChart => {
                                rsx!(BarChart {
                                    id: format!("bar-chart-{id}"),
                                    name: label.to_string()
                                })
                            },

                            WidgetKind::Button => rsx!( button {
                                //key: "{id}",
                                onclick: move |ev| {
                                    log::debug!("click {id}");
                                    let input = Input::Click(id);
                                    ws_cx.send(Message::Bytes(postcard::to_allocvec_cobs(&input).unwrap()));
                                    ev.stop_propagation();
                                },
                                "{label}"}),
                            WidgetKind::Slider => rsx!(
                                UiSlider {
                                    //key: "{id}",
                                    id: id,
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
