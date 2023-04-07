use std::{
    env,
    io::{self, ErrorKind, Write},
};

use embedded_web_ui::{Command, Log};

pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = env::args()
        .nth(1)
        .ok_or(std::io::Error::new(ErrorKind::Other, "need a file name"))?;
    let content = std::fs::read(path)?;

    let content = vec![Command::Log(Log::Elf(content))];
    let ser = postcard::to_allocvec_cobs(&content)?;

    io::stdout().write_all(&ser)?;
    Ok(())
}
