use bytes::BytesMut;
use tokio_serial::{SerialPortBuilderExt, SerialStream};
use tokio_util::codec::{Decoder, Encoder};
use tracing::{debug, info, warn};

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
    // try to be smart about OS dependent device names... but why, there's available_ports()
    // #[cfg(target_os = "linux")]
    // let default = "/dev/ttyUSB*";
    // #[cfg(target_os = "macos")]
    // let default = "/dev/cu.usb*";
    // #[cfg(target_os = "windows")]
    // let default = "COM1";

    // let tty_pattern = arg.as_deref().unwrap_or_else(|| default);
    // for candidate in glob::glob(tty_pattern)? {
    //     match &candidate {

    let ports = match arg {
        Some(port) => vec![port],

        // filter out macos builtin bluetooth "ports"
        None => tokio_serial::available_ports()?
            .iter()
            .map(|port| port.port_name.clone())
            .filter(|port| !port.to_lowercase().contains("bluetooth"))
            .collect(),
    };

    for port in ports {
        info!("opening {port}");
        match tokio_serial::new(&port, 115_200).open_native_async() {
            Ok(stream) => {
                // the tokio-serial example sets this, but it doesn't seem to get along with e.g. micropython's cdc
                // #[cfg(unix)]
                // stream
                //     .set_exclusive(false)
                //     .expect("Unable to set serial port exclusive to false");
                debug!("ok!");
                return Ok(stream);
            }
            Err(e) => warn!("error opening {port}: {e}"),
        }
    }

    Err(eyre::eyre!("could not find a usable serial port"))
}
