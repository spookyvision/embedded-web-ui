#![cfg_attr(not(any(test, feature = "std")), no_std)]

// TODO defmt & std are mutually exclusive, should somehow express this
#[cfg(feature = "defmt")]
use defmt::Format;

use serde::{Deserialize, Serialize};
pub type Id = u16;
pub type ChartTime = u16;
pub type ChartVal = u8;
pub type SliderVal = u8;
pub const CHART_BARS: usize = 64;

// experimental
// I have this elsewhere as
// pub enum FromMcu<'a> { LogChunk(&'a [u8]),
// but don't want to deal with lifetimes right now

#[cfg(not(feature = "std"))]
type Payload = heapless::Vec<u8, 1>; // TODO dummy just so the field is there. James: will this break?
#[cfg(feature = "std")]
type Payload = Vec<u8>;

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug, Hash))]
#[cfg_attr(all(feature = "defmt", not(feature = "std")), derive(Format))]
pub enum Log<'a> {
    Elf(&'a [u8]),
    Packet(&'a [u8]),
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug, Hash))]
#[cfg_attr(all(feature = "defmt", not(feature = "std")), derive(Format))]
pub enum Command<'a> {
    #[serde(borrow)]
    Log(Log<'a>),
    Reset,
    UI(UI),
    TimeSeriesData(TimeSeriesData),
    BarData(BarData),
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug, Hash))]
#[cfg_attr(feature = "defmt", derive(Format))]
pub enum Input {
    Click(Id),
    Slider(Id, SliderVal),
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug, Hash))]
#[cfg_attr(feature = "defmt", derive(Format))]
pub enum UI {
    Widget(Widget),
    Break,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug, Hash))]
#[cfg_attr(feature = "defmt", derive(Format))]
pub struct TimeSeriesData {
    pub id: Id,
    pub time: ChartTime,
    pub val: ChartVal,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug, Hash))]
#[cfg_attr(feature = "defmt", derive(Format))]
pub struct BarData {
    pub id: Id,
    pub vals: heapless::Vec<ChartVal, CHART_BARS>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug, Hash))]
#[cfg_attr(feature = "defmt", derive(Format))]
pub enum UIInput {
    Button(Id),
    Slider(Id, SliderVal),
}
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug, Hash))]
#[cfg_attr(feature = "defmt", derive(Format))]
pub struct Widget {
    pub kind: WidgetKind,
    pub label: heapless::String<32>,
    pub id: Id,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug, Hash))]
#[cfg_attr(feature = "defmt", derive(Format))]
pub enum WidgetKind {
    TimeSeriesChart,
    BarChart,
    Button,
    Slider,
}

impl<'a> From<Widget> for Command<'a> {
    fn from(value: Widget) -> Self {
        Command::UI(UI::Widget(value))
    }
}

impl<'a> From<UI> for Command<'a> {
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
