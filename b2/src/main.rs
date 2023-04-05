use futures::{stream::StreamExt, SinkExt, TryStreamExt};
use listenfd::ListenFd;
use sender_sink::wrappers::{SinkError, UnboundedSenderSink};
use std::{
    collections::HashMap,
    env,
    io::{self, ErrorKind},
    net::SocketAddr,
    str::{self, from_utf8},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, RwLock,
    },
    time::Duration,
};
use tokio_serial::SerialStream;

use tokio::{
    sync::{
        broadcast,
        mpsc::{self, UnboundedReceiver, UnboundedSender},
    },
    time::sleep,
};
use tokio_stream::wrappers::{BroadcastStream, UnboundedReceiverStream};
use tokio_util::codec::Decoder;
use tower_http::{
    cors::CorsLayer,
    trace::{DefaultMakeSpan, TraceLayer},
};
use tracing::{debug, error, info};

use axum::{
    extract::{
        ws::{self, Message, WebSocket, WebSocketUpgrade},
        Extension, TypedHeader,
    },
    headers::UserAgent,
    http::{
        header::{AUTHORIZATION, CONTENT_TYPE},
        Method,
    },
    response::IntoResponse,
    routing::get,
    Router,
};

mod serial;

type ServerMessage = Vec<u8>;
type ToClientTx = UnboundedSender<ServerMessage>;
type ToClientRx = UnboundedReceiver<ServerMessage>;
struct AppState {
    client_txs: RwLock<HashMap<usize, ToClientTx>>,
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
            client_txs: Default::default(),
            to_websocket,
            to_serial,
            from_serial_tx,
        }
    }

    fn add_client(&self, tx: ToClientTx) -> usize {
        let client_id = NEXT_CLIENT_ID.fetch_add(1, Ordering::AcqRel);
        self.client_txs.write().unwrap().insert(client_id, tx);
        client_id
    }
    fn remove_client(&self, client_id: usize) {
        self.client_txs.write().unwrap().remove(&client_id);
    }
}

static NEXT_CLIENT_ID: AtomicUsize = AtomicUsize::new(1);
#[tokio::main]
async fn main() -> eyre::Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt::init();
    info!("start");

    let mut args = env::args();

    let uart_dev = args.nth(1);

    let (to_websocket_tx, to_websocket_rx) = broadcast::channel(100);
    let (to_serial_tx, to_serial_rx) = broadcast::channel(100);
    let (from_serial_tx, from_serial_rx) = mpsc::unbounded_channel();

    let forward_serial_to_websocket =
        UnboundedReceiverStream::new(from_serial_rx).forward(to_websocket_tx.clone().into());

    let app_state = Arc::new(AppState::new(to_websocket_tx, to_serial_tx, from_serial_tx));
    let _uart = tokio::spawn(uart_launcher_loop(uart_dev, app_state.clone()));
    let app = Router::new()
        .route("/", get(websocket_handler))
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

    let server = if let Some(l) = listenfd.take_tcp_listener(0).unwrap() {
        axum::Server::from_tcp(l)?
    } else {
        let addr = SocketAddr::from(([127, 0, 0, 1], 3030));
        axum::Server::bind(&addr)
    };

    debug!("startup complete: {:?}", server);
    server.serve(app.into_make_service()).await.unwrap();
    Ok(())
}

async fn websocket_handler(
    TypedHeader(user_agent): TypedHeader<UserAgent>,
    ws: WebSocketUpgrade,
    Extension(state): Extension<Arc<AppState>>,
) -> impl IntoResponse {
    debug!("User-Agent: {:?}", user_agent);
    ws.on_upgrade(|socket| do_websocket(socket, state, user_agent))
}
async fn do_websocket(stream: WebSocket, state: Arc<AppState>, user_agent: UserAgent) {
    let (mut local_tx, local_rx) = mpsc::unbounded_channel();
    let mut local_rx = UnboundedReceiverStream::new(local_rx);
    let client_id = state.add_client(local_tx.clone());

    let (mut to_websocket_tx, mut from_websocket_rx) = stream.split();

    let to_websocket_rx = state.to_websocket.subscribe();
    let mut to_websocket_rx = tokio_stream::wrappers::BroadcastStream::new(to_websocket_rx);

    let mut send_task = tokio::spawn(async move {
        loop {
            debug!("shoop da loop {}", client_id);
            while let Some(broadcast_msg) = to_websocket_rx.next().await {
                match broadcast_msg {
                    Ok(broadcast_msg) => {
                        let debug_hex = pretty_hex::pretty_hex(&broadcast_msg);
                        debug!("bc to {client_id}: {debug_hex}");
                        to_websocket_tx
                            .send(ws::Message::Binary(broadcast_msg))
                            .await;
                    }
                    Err(e) => {
                        error!("{e:?}");
                        break;
                    }
                }
            }
        }
    });

    let task_state = state.clone();

    // This task will receive messages from the mcu and send them to broadcast subscribers ("browser windows").
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = from_websocket_rx.next().await {
            debug!("hey! listen! {:?}", msg);
            let client_message: Option<Result<String, eyre::Report>> = match &msg {
                Message::Text(text) => {
                    debug!("Text");
                    Some(Ok(text.clone()))
                }
                Message::Binary(data) => {
                    debug!("Binary");
                    None
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
                    debug!("received from {client_id}: {client_message}");
                    // task_state
                    //     .internal_tx
                    //     .send((client_id, client_message, local_tx.clone()))
                    //     .ok();
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

    debug!("disco {}", client_id);

    state.remove_client(client_id);
}

async fn uart_inner(
    stream: SerialStream,
    to_uart: broadcast::Sender<ServerMessage>,
    from_serial_tx: UnboundedSender<ServerMessage>,
) -> eyre::Result<()> {
    let (tx, mut rx) = serial::NullSepCodec.framed(stream).split();
    let to_serial_rx = BroadcastStream::new(to_uart.subscribe())
        .map_err(|e| io::Error::new(ErrorKind::Other, "TODO"));

    let to_serial_stream = to_serial_rx.forward(tx);

    let from_serial_tx: UnboundedSenderSink<ServerMessage> = from_serial_tx.into();

    let from_serial_stream =
        rx.forward(from_serial_tx.sink_map_err(|e| io::Error::new(ErrorKind::Other, "TODO")));

    tokio::try_join!(to_serial_stream, from_serial_stream)?;

    Ok(())
}

async fn uart_launcher_loop(port_name: Option<String>, app_state: Arc<AppState>) {
    let retry_interval = Duration::from_millis(1000);
    let to_serial = app_state.to_serial.clone();
    let from_serial_tx = app_state.from_serial_tx.clone();
    loop {
        if let Ok(stream) = serial::open_tty(port_name.clone()) {
            if let Err(e) = uart_inner(stream, to_serial.clone(), from_serial_tx.clone()).await {
                error!("{e:?}");
            }
        }

        sleep(retry_interval).await
    }
}
