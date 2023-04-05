#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(slice_as_chunks)]

use defmt::{panic, *};
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_stm32::{
    interrupt,
    time::mhz,
    usb_otg::{Driver, Instance},
    Config,
};
use embassy_usb::{
    class::cdc_acm::{CdcAcmClass, State},
    driver::EndpointError,
    Builder,
};
use embedded_web_ui::{Command, Input, Widget, WidgetKind, UI};
use futures::future::join;
use panic_probe as _;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("Hello World!");

    let mut config = Config::default();
    config.rcc.pll48 = true;
    config.rcc.sys_ck = Some(mhz(48));

    let p = embassy_stm32::init(config);

    // Create the driver, from the HAL.
    let irq = interrupt::take!(OTG_FS);
    let mut ep_out_buffer = [0u8; 256];
    let driver = Driver::new_fs(p.USB_OTG_FS, irq, p.PA12, p.PA11, &mut ep_out_buffer);

    // Create embassy-usb Config
    let mut config = embassy_usb::Config::new(0xc0de, 0xcafe);
    config.manufacturer = Some("Embassy");
    config.product = Some("USB-serial example");
    config.serial_number = Some("12345678");

    // Required for windows compatiblity.
    // https://developer.nordicsemi.com/nRF_Connect_SDK/doc/1.9.1/kconfig/CONFIG_CDC_ACM_IAD.html#help
    config.device_class = 0xEF;
    config.device_sub_class = 0x02;
    config.device_protocol = 0x01;
    config.composite_with_iads = true;

    // Create embassy-usb DeviceBuilder using the driver and config.
    // It needs some buffers for building the descriptors.
    let mut device_descriptor = [0; 256];
    let mut config_descriptor = [0; 256];
    let mut bos_descriptor = [0; 256];
    let mut control_buf = [0; 64];

    let mut state = State::new();

    let mut builder = Builder::new(
        driver,
        config,
        &mut device_descriptor,
        &mut config_descriptor,
        &mut bos_descriptor,
        &mut control_buf,
    );

    // Create classes on the builder.
    let mut class = CdcAcmClass::new(&mut builder, &mut state, 64);

    // Build the builder.
    let mut usb = builder.build();

    // Run the USB device.
    let usb_fut = usb.run();

    // Do stuff with the class!
    let echo_fut = async {
        loop {
            class.wait_connection().await;
            info!("Connected");
            let _ = echo(&mut class).await;
            info!("Disconnected");
        }
    };

    // Run everything concurrently.
    // If we had made everything `'static` above instead, we could do this using separate tasks instead.
    join(usb_fut, echo_fut).await;
}

struct Disconnected {}

impl From<EndpointError> for Disconnected {
    fn from(val: EndpointError) -> Self {
        match val {
            EndpointError::BufferOverflow => panic!("Buffer overflow"),
            EndpointError::Disabled => Disconnected {},
        }
    }
}

async fn echo<'d, T: Instance + 'd>(
    class: &mut CdcAcmClass<'d, Driver<'d, T>>,
) -> Result<(), Disconnected> {
    let mut ui: heapless::Vec<_, 6> = heapless::Vec::new();
    ui.extend([
        Command::Reset,
        Widget {
            kind: WidgetKind::Button,
            label: "oh,aye".into(),
            id: 1,
        }
        .into(),
        UI::Break.into(),
        Widget {
            kind: WidgetKind::Slider,
            label: "slidos".into(),
            id: 2,
        }
        .into(),
        UI::Break.into(),
        Widget {
            kind: WidgetKind::BarChart,
            label: "a bar chart".into(),
            id: 3,
        }
        .into(),
    ]);

    let mut ui_buf = [0; 64];
    let ser = postcard::to_slice_cobs(&ui, &mut ui_buf).unwrap();

    let mut buf = [0; 64];
    loop {
        let n = class.read_packet(&mut buf).await?;
        let data = &mut buf[..n];
        match postcard::from_bytes_cobs::<Input>(data) {
            Ok(input) => info!("got {}", input),
            Err(_) => info!("gotnt!"),
        }
        class.write_packet(ser).await?;
    }
}
