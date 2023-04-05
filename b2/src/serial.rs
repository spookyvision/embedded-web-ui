use bytes::BytesMut;
use log::debug;
use tokio_serial::{SerialPortBuilderExt, SerialStream};
use tokio_util::codec::{Decoder, Encoder};

pub(crate) struct NullSepCodec;

impl Decoder for NullSepCodec {
    type Item = Vec<u8>;
    type Error = std::io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let null = src.as_ref().iter().position(|b| *b == b'\0');
        if let Some(n) = null {
            let mut chunk = src.split_to(n + 1);
            return Ok(Some(chunk.into()));
        }
        Ok(None)
    }
}

impl Encoder<Vec<u8>> for NullSepCodec {
    type Error = std::io::Error;

    fn encode(&mut self, item: Vec<u8>, dst: &mut BytesMut) -> Result<(), Self::Error> {
        dst.extend_from_slice(&item);
        Ok(())
    }
}

pub(crate) fn open_tty(arg: Option<String>) -> eyre::Result<SerialStream> {
    #[cfg(target_os = "linux")]
    let default = "/dev/ttyUSB*";
    #[cfg(target_os = "macos")]
    let default = "/dev/cu.usb*";
    #[cfg(target_os = "windows")]
    let default = "COM1";

    let tty_pattern = arg.as_deref().unwrap_or_else(|| default);

    for candidate in glob::glob(tty_pattern)? {
        match &candidate {
            Ok(dev_path_buf) => {
                let dev = dev_path_buf.as_path().to_string_lossy();
                debug!("opening {dev}");
                match tokio_serial::new(dev.clone(), 115_200).open_native_async() {
                    Ok(mut stream) => {
                        // the tokio-serial example sets this, but it doesn't jive well with e.g. micropython's cdc
                        // #[cfg(unix)]
                        // stream
                        //     .set_exclusive(false)
                        //     .expect("Unable to set serial port exclusive to false");
                        debug!("ok!");
                        return Ok(stream);
                    }
                    Err(e) => log::warn!("error opening {dev}: {e}"),
                }
            }
            Err(e) => return Err(eyre::eyre!("glob error with {candidate:?}: {e}")),
        }
    }

    Err(eyre::eyre!("could not find a usable serial port"))
}
