#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(slice_as_chunks)]

use defmt::{panic, *};
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_stm32::{
    gpio::{Level, Output, Speed},
    interrupt,
    peripherals::PC13,
    time::mhz,
    usb_otg::{Driver, Instance},
    Config,
};
use embassy_time::{Duration, Timer};
use embassy_usb::{
    class::cdc_acm::{CdcAcmClass, State},
    driver::EndpointError,
    Builder,
};
use embedded_web_ui::{BarData, Command, Input, Widget, WidgetKind, CHART_BARS, UI};
use futures::future::join;
use panic_probe as _;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("ohai");

    let mut config = Config::default();
    config.rcc.pll48 = true;
    config.rcc.sys_ck = Some(mhz(48));

    let p = embassy_stm32::init(config);

    let mut led = Output::new(p.PC13, Level::High, Speed::Low);

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
    let work_fut = async {
        loop {
            class.wait_connection().await;
            info!("Connected");
            let _ = work(&mut class, &mut led).await;
            info!("Disconnected");
        }
    };

    // Run everything concurrently.
    // If we had made everything `'static` above instead, we could do this using separate tasks instead.
    join(usb_fut, work_fut).await;
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

async fn write_chunked<'d, T: Instance + 'd>(
    data: &[u8],
    class: &mut CdcAcmClass<'d, Driver<'d, T>>,
) -> Result<(), Disconnected> {
    let (chunks, rest) = data.as_chunks::<64>();
    for chunk in chunks {
        class.write_packet(chunk).await?;
    }
    class.write_packet(rest).await?;
    Ok(())
}

async fn work<'d, T: Instance + 'd>(
    class: &mut CdcAcmClass<'d, Driver<'d, T>>,
    led: &mut Output<'_, PC13>,
) -> Result<(), Disconnected> {
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

    // give USB setup some time to settle
    Timer::after(Duration::from_secs(1)).await;

    let seed = [0, 1, 3, 3, 7, 0, 0, 0];
    let mut rng = WyRng::from_seed(seed);

    let mut ser_buf = [0; 96];
    let ser = postcard::to_slice_cobs(ui.as_slice(), &mut ser_buf).unwrap();
    // class.write_packet(ser).await?;
    write_chunked(&ser, class).await?;

    let mut buf = [0; 64];

    loop {
        let n = class.read_packet(&mut buf).await?;
        let data = &mut buf[..n];
        match postcard::from_bytes_cobs::<Input>(data) {
            Ok(input) => {
                info!("got {}", input);
                match input {
                    // refresh hack
                    Input::Click(0) => {
                        let ser = postcard::to_slice_cobs(ui.as_slice(), &mut ser_buf).unwrap();
                        write_chunked(&ser, class).await?;
                    }
                    Input::Click(1) => {
                        led.set_low();
                    }
                    Input::Click(2) => {
                        led.set_high();
                    }
                    Input::Click(3) => {
                        for _ in 0..10 {
                            let vals = random_chart_data(&mut rng);
                            let commands = [Command::BarData(BarData { id: 5, vals })];

                            match postcard::to_slice_cobs(commands.as_slice(), &mut ser_buf) {
                                Ok(ser) => {
                                    write_chunked(&ser, class).await?;
                                }
                                Err(e) => {
                                    error!("no! {}", e)
                                }
                            }
                            Timer::after(Duration::from_millis(200)).await;
                        }
                    }
                    Input::Slider(_, _) => {}
                    _ => {}
                }
            }
            Err(_) => info!("gotnt!"),
        }
    }
}

use rand_core::{RngCore, SeedableRng};
use wyhash::WyRng;

fn random_chart_data(rng: &mut WyRng) -> heapless::Vec<u8, CHART_BARS> {
    let mut vals = heapless::Vec::new();
    for _ in 0..CHART_BARS / 8 {
        let r = rng.next_u64();
        vals.extend(r.to_le_bytes());
    }
    vals
}
