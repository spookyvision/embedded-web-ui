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
            let chunk = src.split_to(n + 1);
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
    // filter out macos builtin bluetooth "ports"
    let available_ports = tokio_serial::available_ports()?;
    let auto_ports = || {
        available_ports
            .iter()
            .map(|port| port.port_name.clone())
            .filter(|port| !port.to_lowercase().contains("bluetooth"))
            .collect()
    };

    // ensure arg is not empty
    let ports = match arg.as_deref() {
        Some("") => auto_ports(),
        Some(port) => vec![port.to_string()],
        None => auto_ports(),
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
