#![allow(non_snake_case)]

use components::*;
use defmt_handler::DefmtLogger;
use dioxus::prelude::*;
use dioxus_websocket_hooks::{use_ws_context, use_ws_context_provider, Message};
use embedded_web_ui::{Command, Id, Input, Log, Widget, WidgetKind, UI};
use futures::stream::StreamExt;
use im_rc::HashSet;
use log::{debug, error, info, warn};
use wasm_bindgen::prelude::wasm_bindgen;
use web_app::ser_de::{decode, encode};

mod components;
mod defmt_handler;

fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    // suppress dioxus logging - TODO needs more love
    // wasm_logger::init(wasm_logger::Config::default().module_prefix("web-app"));
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

fn use_ws_context_provider_binary(cx: &ScopeState, url: &str, handler: impl Fn(Vec<u8>) + 'static) {
    let handler = move |msg| {
        if let Message::Bytes(data) = msg {
            handler(data)
        } else {
            debug!("no handler for {msg:?}");
        }
    };

    use_ws_context_provider(cx, url, handler)
}

// TODO UseRef isn't optimal, `with_mut` causes Dioxus to rerender. What to do…
fn handle_log(logger: &UseRef<Option<DefmtLogger>>, log: Log) {
    match log {
        Log::Elf(elf) => {
            log::warn!(
                "very probably UB around here, this vec clone fixes broken elf data down the line"
            );
            let elf: Vec<u8> = elf.into();
            let hash = md5::compute(&elf);
            debug!("creating new log handler {} {:?}", elf.len(), hash);
            logger.with_mut(|logger| *logger = defmt_handler::DefmtLogger::new(&elf))
        }
        Log::Packet(chunk) => {
            logger.with_mut(|logger| match logger {
                Some(logger) => {
                    if let Err(e) = logger.received(&chunk) {
                        error!("logger: {e:?}");
                    }
                }
                None => warn!("received log packet but do not have a logger"),
            });
        }
    }
}

fn app(cx: Scope) -> Element {
    let logger = use_ref(cx, || None);
    // TODO use_ref, remove im_rc
    let slider_vars = use_state(&cx, SliderVars::default);
    let charts = use_ref(cx, || HashSet::new());

    let ui_items = use_ref(cx, || {
        let res: Vec<UI> = vec![UI::Widget(Widget {
            kind: WidgetKind::Button,
            label: "plz refresh".into(),
            id: 0,
        })];
        res
    });

    let update_ui = use_coroutine(&cx, |mut rx: UnboundedReceiver<UI>| {
        to_owned![ui_items];
        async move {
            while let Some(ui_elem) = rx.next().await {
                debug!("got new UI! {ui_elem:?}");
                ui_items.with_mut(|items| items.push(ui_elem));
            }
        }
    })
    .to_owned();

    let _ws = use_ws_context_provider_binary(cx, "ws://localhost:3030", {
        to_owned![ui_items, charts, logger];
        move |mut d| {
            if let Ok(commands) = decode::<Vec<Command>>(d.as_mut_slice()) {
                for command in commands {
                    match command {
                        Command::Log(log) => handle_log(&logger, log),
                        Command::Reset => {
                            info!("RESET");
                            charts.with_mut(|charts| charts.clear());
                            ui_items.with_mut(|items| items.clear());
                        }
                        Command::UI(ui) => {
                            update_ui.send(ui.clone());
                        }
                        Command::TimeSeriesData(data) => {
                            debug!("time series data: {data:?}");
                        }
                        Command::BarData(data) => {
                            let id = data.id;

                            if !charts.read().contains(&id) {
                                charts.with_mut(|charts| charts.insert(id));
                                init_chart(id);
                            }
                            let mut res = vec![];
                            res.extend(&data.vals);
                            update(data.id, res);
                        }
                    }
                }
            } else {
                warn!("got garbage")
            }
        }
    });

    let ws_cx = use_ws_context(&cx);

    let ui_items_it = ui_items.read();
    cx.render(rsx! (
        div { style: "text-align: center;",
            h1 { "embedded-web-ui" }
            div {
                ui_items_it.iter().map(|item| match item {
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
                                    debug!("click {id}");
                                    let input = Input::Click(id);
                                    ws_cx.send(Message::Bytes(encode(&input).unwrap()));
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
                    UI::Break => rsx!{ br{} }
                })
            }
        }
    ))
}
