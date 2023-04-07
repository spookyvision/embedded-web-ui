use std::{collections::BTreeMap, path::Path};

use anyhow::anyhow;
use defmt_decoder::{DecodeError, Frame, Location, Locations, StreamDecoder, Table};
use log::{error, warn};

type LocationInfo = (Option<String>, Option<u32>, Option<String>);

// TODO unscrew this massive self-referential screwup by fixing `StreamDecoder`
// James:
// I mean I'd probably say it could be AsRef<Table> instead of &Table
// then you could give it an arc or box or reference or owned type and it should all work
// https://play.rust-lang.org/?version=stable&mode=debug&edition=2021&gist=907d188ca0b8303b67ef05109b653b7a

pub(crate) struct DefmtLogger {
    inner: DefmtSelfRef,
}

impl DefmtLogger {
    pub(crate) fn new(ELF: &[u8]) -> Option<Self> {
        match DefmtSelfRef::build(ELF) {
            Ok(inner) => Some(Self { inner }),
            Err(e) => {
                log::error!("could not build logger: {e:?}");
                None
            }
        }
    }

    pub(crate) fn received(&mut self, chunk: &[u8]) -> anyhow::Result<()> {
        self.inner.received(chunk)
    }
}

use ouroboros::self_referencing;
#[self_referencing]
struct DefmtSelfRef {
    pub table: Table,

    #[borrows(table)]
    #[covariant]
    pub handler: DefmtHandler<'this>,
}

impl DefmtSelfRef {
    pub(super) fn received(&mut self, chunk: &[u8]) -> anyhow::Result<()> {
        self.with_mut(|fields| fields.handler.received(chunk))
    }
    pub fn build(ELF: &[u8]) -> anyhow::Result<DefmtSelfRef> {
        let table = match Table::parse(ELF) {
            Ok(Some(table)) => table,
            Err(e) => {
                return Err(anyhow!("could not parse ELF table: {e:?}"));
            }
            _ => {
                return Err(anyhow!("AAAAAAAAAAAAA!"));
            }
        };
        let locs = table.get_locations(ELF)?;

        let locs = if table.indices().all(|idx| locs.contains_key(&(idx as u64))) {
            Some(locs)
        } else {
            warn!("(BUG) location info is incomplete; it will be omitted from the output");
            None
        };

        Ok(DefmtSelfRef::new(table, |table| {
            DefmtHandler::new(
                table.new_stream_decoder(),
                locs,
                table.encoding().can_recover(),
                true,
                true,
            )
        }))
    }
}

struct DefmtHandler<'a> {
    decoder: Box<dyn StreamDecoder + 'a>,
    can_recover: bool,
    locs: Option<BTreeMap<u64, Location>>,
    // TODO make configurable again
    show_skipped_frames: bool,
    verbose: bool,
}

impl<'a> DefmtHandler<'a> {
    fn new(
        decoder: Box<dyn StreamDecoder + 'a>,
        locs: Option<BTreeMap<u64, Location>>,
        can_recover: bool,
        show_skipped_frames: bool,
        verbose: bool,
    ) -> Self {
        Self {
            decoder,
            locs,
            can_recover,
            show_skipped_frames,
            verbose,
        }
    }

    fn received(&mut self, chunk: &[u8]) -> anyhow::Result<()> {
        let decoder = &mut self.decoder;
        decoder.received(chunk);

        loop {
            match decoder.decode() {
                Ok(frame) => {
                    forward_to_logger(&frame, location_info(&self.locs, &frame, "N/A (todo)"))
                }
                Err(DecodeError::UnexpectedEof) => {
                    // error!("log decoder: EOF");
                    break;
                }
                Err(DecodeError::Malformed) => match self.can_recover {
                    // if recovery is impossible, abort
                    false => return Err(DecodeError::Malformed.into()),
                    // if recovery is possible, skip the current frame and continue with new data
                    true => {
                        if self.show_skipped_frames || self.verbose {
                            error!("(HOST) malformed frame skipped");
                            error!("└─ {} @ {}:{}", env!("CARGO_PKG_NAME"), file!(), line!());
                        }
                        continue;
                    }
                },
            }
        }
        Ok(())
    }
}

fn forward_to_logger(frame: &Frame, location_info: LocationInfo) {
    let (file, line, mod_path) = location_info;
    defmt_decoder::log::log_defmt(frame, file.as_deref(), line, mod_path.as_deref(), false);
}

fn location_info(
    locs: &Option<Locations>,
    frame: &Frame,
    current_dir: impl AsRef<Path>,
) -> LocationInfo {
    let (mut file, mut line, mut mod_path) = (None, None, None);

    // NOTE(`[]` indexing) all indices in `table` have been verified to exist in the `locs` map
    let loc = locs.as_ref().map(|locs| &locs[&frame.index()]);

    if let Some(loc) = loc {
        // try to get the relative path, else the full one
        let path = loc.file.strip_prefix(&current_dir).unwrap_or(&loc.file);

        file = Some(path.display().to_string());
        line = Some(loc.line as u32);
        mod_path = Some(loc.module.clone());
    }

    (file, line, mod_path)
}
