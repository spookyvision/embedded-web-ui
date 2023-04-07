use std::{
    env,
    io::{self, ErrorKind, Write},
};

use embedded_web_ui::{BarData, Command, Input, Widget, WidgetKind, CHART_BARS, UI};

pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ui = [
        Command::Reset,
        Widget {
            kind: WidgetKind::Button,
            label: "LED on".into(),
            id: 1,
        }
        .into(),
        Widget {
            kind: WidgetKind::Button,
            label: "LED off".into(),
            id: 2,
        }
        .into(),
        Widget {
            kind: WidgetKind::Button,
            label: "give data".into(),
            id: 3,
        }
        .into(),
        UI::Break.into(),
        Widget {
            kind: WidgetKind::Slider,
            label: "slidos".into(),
            id: 4,
        }
        .into(),
        UI::Break.into(),
        Widget {
            kind: WidgetKind::BarChart,
            label: "a bar chart".into(),
            id: 5,
        }
        .into(),
    ];

    let ser = serde_json::to_vec(&ui)?;

    io::stdout().write_all(&ser)?;
    Ok(())
}
