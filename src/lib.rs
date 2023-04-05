#![cfg_attr(not(any(test, feature = "std")), no_std)]

#[cfg(feature = "defmt")]
use defmt::Format;
use serde::{Deserialize, Serialize};
pub type Id = u16;
pub type ChartTime = u16;
pub type ChartVal = u8;
pub type SliderVal = u8;

#[derive(Serialize, Deserialize)]
#[cfg_attr(feature = "std", derive(Debug))]
#[cfg_attr(feature = "defmt", derive(Format))]
pub enum Command {
    Log, // TODO - defmt payload goes here
    Reset,
    UI(UI),
    TimeSeriesData(TimeSeriesData),
    BarData(BarData),
}

#[derive(Serialize, Deserialize)]
#[cfg_attr(feature = "std", derive(Debug))]
#[cfg_attr(feature = "defmt", derive(Format))]
pub enum Input {
    Click(Id),
    Slider(Id, SliderVal),
}

#[derive(Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "std", derive(Debug))]
#[cfg_attr(feature = "defmt", derive(Format))]
pub enum UI {
    Widget(Widget),
    Break,
}

#[derive(Serialize, Deserialize)]
#[cfg_attr(feature = "std", derive(Debug))]
#[cfg_attr(feature = "defmt", derive(Format))]
pub struct TimeSeriesData {
    pub id: Id,
    pub time: ChartTime,
    pub val: ChartVal,
}

#[derive(Serialize, Deserialize)]
#[cfg_attr(feature = "std", derive(Debug))]
#[cfg_attr(feature = "defmt", derive(Format))]
pub struct BarData {
    pub id: Id,
    pub vals: heapless::Vec<ChartVal, 64>,
}

#[derive(Serialize, Deserialize)]
#[cfg_attr(feature = "std", derive(Debug))]
#[cfg_attr(feature = "defmt", derive(Format))]
pub enum UIInput {
    Button(Id),
    Slider(Id, SliderVal),
}

#[derive(Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "std", derive(Debug))]
#[cfg_attr(feature = "defmt", derive(Format))]
pub struct Widget {
    pub kind: WidgetKind,
    pub label: heapless::String<32>,
    pub id: Id,
}

#[derive(Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "std", derive(Debug))]
#[cfg_attr(feature = "defmt", derive(Format))]
pub enum WidgetKind {
    TimeSeriesChart,
    BarChart,
    Button,
    Slider,
}

impl From<Widget> for Command {
    fn from(value: Widget) -> Self {
        Command::UI(UI::Widget(value))
    }
}

impl From<UI> for Command {
    fn from(value: UI) -> Self {
        match value {
            UI::Widget(w) => w.into(),
            UI::Break => Command::UI(UI::Break),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        // cool
    }
}
