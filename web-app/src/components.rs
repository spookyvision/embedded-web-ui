use dioxus::prelude::*;

pub(crate) type SliderVars = im_rc::HashMap<String, f32>;

#[allow(non_snake_case)]
#[inline_props]
pub(crate) fn UiSlider(cx: Scope, name: String, vars: UseState<SliderVars>) -> Element {
    const SCALE: f32 = 100.0;

    let val = vars.get().get(name.as_str()).cloned().unwrap_or_default();
    cx.render(rsx!(
        input {
            r#type: "range",
            value: "{val * SCALE}",
            name: "{name}",

            oninput: move |ev| {
                let new_val = ev.value.parse::<f32>().unwrap_or_default() / SCALE;
                log::debug!("change {name} {new_val}");
                vars.with_mut(|vars| {vars.insert(name.clone(), new_val); });
            },
        }
        label {
            r#for: "{name}",
            "{name}"
        }
    ))
}

#[allow(non_snake_case)]
#[inline_props]
pub(crate) fn BarChart(cx: Scope, id: String, name: String) -> Element {
    cx.render(rsx!(
        div {
            div {
                class: "chart",
                svg {
                    id: "{id}"
                },
            }
            label {
                "{name}"
            }
        }

    ))
}
