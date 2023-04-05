#![no_std]

use serde::{Deserialize, Serialize};
pub type Id = u16;

#[derive(Serialize, Deserialize)]
#[cfg_attr(feature = "std", derive(Debug))]
pub enum Command {
    Log, // TODO - defmt payload goes here
    Reset,
    UI(UI),
    TimeSeriesData(TimeSeriesData),
    BarData(BarData),
}

#[derive(Serialize, Deserialize)]
#[cfg_attr(feature = "std", derive(Debug))]
pub enum UI {
    Widget(Widget),
    Break,
}

#[derive(Serialize, Deserialize)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct TimeSeriesData {
    pub id: Id,
    pub time: u16,
    pub val: u16,
}

#[derive(Serialize, Deserialize)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct BarData {
    pub id: Id,
    pub vals: heapless::Vec<u16, 32>,
}

#[derive(Serialize, Deserialize)]
#[cfg_attr(feature = "std", derive(Debug))]
pub enum UIInput {
    Button(Id),
    Slider(Id, u16),
}

#[derive(Serialize, Deserialize)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Widget {
    pub kind: WidgetKind,
    pub label: heapless::String<32>,
    pub id: Id,
}
#[derive(Serialize, Deserialize)]
#[cfg_attr(feature = "std", derive(Debug))]
pub enum WidgetKind {
    TimeSeriesChart,
    BarChart,
    Button,
    Slider,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        // cool
    }
}
