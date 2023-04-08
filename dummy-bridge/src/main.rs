use bytes::Bytes;
use embedded_web_ui::{Widget, WidgetKind, Command, UI, Input};
use futures::{stream::StreamExt, SinkExt, TryStreamExt};
use listenfd::ListenFd;
use pretty_hex::pretty_hex;
use sender_sink::wrappers::UnboundedSenderSink;
use std::{
    env,
    io::{self, ErrorKind},
    net::SocketAddr,
    sync::Arc,
    time::Duration,
};
use tokio_serial::SerialStream;

use tokio::{
    sync::{
        broadcast,
        mpsc::{self, UnboundedSender},
    },
    time::sleep,
};
use tokio_stream::wrappers::{BroadcastStream, UnboundedReceiverStream};
use tokio_util::codec::Decoder;
use tower_http::{
    cors::CorsLayer,
    trace::{DefaultMakeSpan, TraceLayer},
};
use tracing::{debug, error, info, warn};

use axum::{
    extract::{
        ws::{self, Message, WebSocket, WebSocketUpgrade},
        DefaultBodyLimit, Extension, TypedHeader,
    },
    headers::UserAgent,
    http::{
        header::{AUTHORIZATION, CONTENT_TYPE},
        Method,
    },
    response::IntoResponse,
    routing::{get, post},
    Router,
};

mod serial;

type ServerMessage = Vec<u8>;
struct AppState {
    to_websocket: broadcast::Sender<ServerMessage>,
    to_serial: broadcast::Sender<ServerMessage>,
    from_serial_tx: UnboundedSender<ServerMessage>,
}
impl AppState {
    fn new(
        to_websocket: broadcast::Sender<ServerMessage>,
        to_serial: broadcast::Sender<ServerMessage>,
        from_serial_tx: UnboundedSender<ServerMessage>,
    ) -> Self {
        Self {
            to_websocket,
            to_serial,
            from_serial_tx,
        }
    }
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt::init();
    info!("start");

    let mut args = env::args();

    let serial_dev = args.nth(1);

    // the rx part is created later
    let (to_websocket_tx, _) = broadcast::channel(100);
    // the rx part is created later
    let (to_serial_tx, _) = broadcast::channel(100);
    let (from_serial_tx, from_serial_rx) = mpsc::unbounded_channel();

    // TODO can't do that, apparently :|
    // let forward_serial_to_websocket =
    //     UnboundedReceiverStream::new(from_serial_rx).forward(to_websocket_tx.into());

    let _forward_serial_to_websocket_task = {
        let to_websocket_tx = to_websocket_tx.clone();
        tokio::spawn(async move {
            let mut from_serial_rx = UnboundedReceiverStream::new(from_serial_rx);
            while let Some(item) = from_serial_rx.next().await {
                debug!("serial>ws {}", pretty_hex(&item));
                // TODO ok()
                to_websocket_tx.send(item).ok();
            }
        })
    };

    let app_state = Arc::new(AppState::new(to_websocket_tx, to_serial_tx, from_serial_tx));
    let ass1 = app_state.clone();
    let ass2 = app_state.clone();
    let _serial_task = tokio::spawn(async move {
        // uart_launcher_loop(serial_dev, app_state.clone())
        let to_serial = &ass1.from_serial_tx;

        tokio::time::sleep(Duration::from_millis(3000)).await;

        tokio::spawn(async move {
            let mut rx = ass2.to_serial.subscribe();
            loop {
                match rx.recv().await {
                    Ok(mut m) => {
                        let msg: Input = postcard::from_bytes_cobs(&mut m).unwrap();
                        println!("{msg:?}");
                    },
                    Err(e) => println!("err: {e:?}"),
                }
            }
        });

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

        let msg = postcard::to_stdvec_cobs(ui.as_slice()).unwrap();
        while let Err(_) = to_serial.send(msg.clone()) {
            println!("Send failed...");
            sleep(Duration::from_millis(500)).await;
        }
        println!("Sent!");
        loop {
            sleep(Duration::from_millis(100)).await;
        }

    });
    let app = Router::new()
        .route("/", get(websocket_handler))
        .route("/backdoor", post(backdoor_handler))
        // ELF or whatever else you want to upload size limit
        .layer(DefaultBodyLimit::max(15 * 1024 * 1024))
        .layer(Extension(app_state.clone()))
        // logging so we can see whats going on
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::default().include_headers(true)),
        )
        // of corse
        .layer(
            CorsLayer::new()
                .allow_headers(vec![CONTENT_TYPE, AUTHORIZATION])
                .allow_origin(tower_http::cors::Any)
                .allow_methods(vec![Method::GET, Method::POST, Method::PUT, Method::DELETE]),
        );

    let mut listenfd = ListenFd::from_env();

    let port = 3030;
    let server = if let Some(l) = listenfd.take_tcp_listener(0).unwrap() {
        axum::Server::from_tcp(l)?
    } else {
        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        axum::Server::bind(&addr)
    };

    debug!("startup complete: {:?}", server);
    info!("listening on port {port}");
    server.serve(app.into_make_service()).await.unwrap();
    Ok(())
}

async fn backdoor_handler(
    TypedHeader(_user_agent): TypedHeader<UserAgent>,
    Extension(state): Extension<Arc<AppState>>,
    body: Bytes,
) -> impl IntoResponse {
    let _msg = pretty_hex(&body);
    debug!("got a sneaky update");
    match state.to_websocket.send(body.into()) {
        Ok(_) => "OK",
        Err(_) => "nobody home?",
    }
}
async fn websocket_handler(
    TypedHeader(user_agent): TypedHeader<UserAgent>,
    ws: WebSocketUpgrade,
    Extension(state): Extension<Arc<AppState>>,
) -> impl IntoResponse {
    debug!("User-Agent: {:?}", user_agent);
    ws.on_upgrade(|socket| do_websocket(socket, state, user_agent))
}
async fn do_websocket(stream: WebSocket, state: Arc<AppState>, _user_agent: UserAgent) {
    let (mut to_websocket_tx, mut from_websocket_rx) = stream.split();

    let to_websocket_rx = state.to_websocket.subscribe();
    let mut to_websocket_rx = tokio_stream::wrappers::BroadcastStream::new(to_websocket_rx);

    let to_serial_tx = state.to_serial.clone();

    // BUG/workaround in dioxus websocket context - need to initialize state with a server hello
    to_websocket_tx
        .send(ws::Message::Text("hello".to_string()))
        .await
        .ok();

    // This task will receive messages from the mcu and send them to broadcast subscribers ("browser windows").
    // TODO would rather use .forward()
    let mut send_task = tokio::spawn(async move {
        loop {
            while let Some(to_ws) = to_websocket_rx.next().await {
                match to_ws {
                    Ok(to_ws) => {
                        // TODO ok()
                        to_websocket_tx.send(ws::Message::Binary(to_ws)).await.ok();
                    }
                    Err(e) => {
                        error!("{e:?}");
                        break;
                    }
                }
            }
        }
    });

    // This task will receive messages from browser window(s) and send them to the mcu.
    // TODO would rather use .forward()
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = from_websocket_rx.next().await {
            let client_message: Option<Result<Vec<u8>, eyre::Report>> = match &msg {
                Message::Text(text) => {
                    debug!("Text");
                    Some(Ok(text.as_bytes().into()))
                }
                Message::Binary(data) => {
                    debug!("Binary");
                    Some(Ok(data.clone()))
                }
                Message::Ping(_) => {
                    debug!("PING");
                    None
                }
                Message::Pong(_) => {
                    debug!("PONG");
                    None
                }
                Message::Close(_) => {
                    info!("ws::Message::Close - bye");
                    None
                }
            };
            match client_message {
                Some(Err(oh_no)) => {
                    error!("could not parse client message: {:#?} {:#?}", msg, oh_no)
                }
                Some(Ok(client_message)) => {
                    let msg = pretty_hex(&client_message);
                    debug!("ws>serial: {msg}");

                    // TODO ok()
                    to_serial_tx.send(client_message).ok();
                }
                _ => {}
            }
        }
    });

    // If any one of the tasks exit, abort the other.
    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    };

    debug!("client disconnected");
}

// async fn uart_inner(
//     stream: SerialStream,
//     to_serial_tx: broadcast::Sender<ServerMessage>,
//     from_serial_tx: UnboundedSender<ServerMessage>,
// ) -> eyre::Result<()> {
//     let (tx, rx) = serial::NullSepCodec.framed(stream).split();
//     // TODO weird error stuff
//     let to_serial_rx = BroadcastStream::new(to_serial_tx.subscribe()).map_err(|e| {
//         let inner_error = format!("{e:?}");
//         io::Error::new(ErrorKind::Other, inner_error)
//     });

//     let to_serial_stream = to_serial_rx.forward(tx);

//     let from_serial_tx: UnboundedSenderSink<_> = from_serial_tx.into();

//     // TODO weird error stuff
//     let from_serial_stream = rx.forward(from_serial_tx.sink_map_err(|e| {
//         let inner_error = format!("{e:?}");
//         io::Error::new(ErrorKind::Other, inner_error)
//     }));

//     tokio::try_join!(to_serial_stream, from_serial_stream)?;

//     Ok(())
// }

// async fn uart_launcher_loop(port_name: Option<String>, app_state: Arc<AppState>) {
//     let retry_interval = Duration::from_millis(1000);
//     let to_serial = app_state.to_serial.clone();
//     let from_serial_tx = app_state.from_serial_tx.clone();
//     loop {
//         match serial::open_tty(port_name.clone()) {
//             Ok(stream) => {
//                 if let Err(e) = uart_inner(stream, to_serial.clone(), from_serial_tx.clone()).await
//                 {
//                     warn!("serial device threw up: {e:?}");
//                 }
//             }
//             Err(e) => {
//                 warn!("could not open a serial device: {e:?}");
//             }
//         }

//         sleep(retry_interval).await
//     }
// }
