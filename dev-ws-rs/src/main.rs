use embedded_web_ui::{Command, Input, Widget, WidgetKind, UI};
use futures_util::{SinkExt, StreamExt};
use log::*;
use std::{net::SocketAddr, time::Duration};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::accept_async;
use tungstenite::Message;

async fn accept_connection(peer: SocketAddr, stream: TcpStream) {
    if let Err(e) = handle_connection(peer, stream).await {
        error!("(maybe) error processing connection: {e:?}")
    }
}

async fn handle_connection(
    peer: SocketAddr,
    stream: TcpStream,
) -> Result<(), Box<dyn std::error::Error>> {
    let ws_stream = accept_async(stream).await.expect("Failed to accept");
    info!("New WebSocket connection: {}", peer);
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    let mut interval = tokio::time::interval(Duration::from_millis(1000));

    let items: Vec<Command> = vec![
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
        UI::Break.into(),
        Widget {
            kind: WidgetKind::BarChart,
            label: "another bar chart".into(),
            id: 4,
        }
        .into(),
    ];

    let ser = postcard::to_allocvec_cobs(&items)?;

    loop {
        tokio::select! {
                    msg = ws_receiver.next() => {
                        match msg {
                            Some(msg) => {
                                let msg = msg?;
                                match msg {
                                    Message::Binary(mut data) => {
                                        match postcard::from_bytes_cobs::<Input>(&mut data) {
                                            Ok(input) => info!("received input {input:?}"),
                                            Err(e) => error!("oh noes, {e:?}"),
                                        }
                                    },
                                    Message::Close(_) => break,
                                    _ => info!("naw ")
        }

                            }
                            None => break,
                        }
                    }
                    _ = interval.tick() => {
                        ws_sender.send(Message::Binary(ser.clone())).await?;
                    }
                }
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let addr = "127.0.0.1:9002";
    let listener = TcpListener::bind(&addr).await.expect("Can't listen");
    info!("Listening on: {}", addr);

    while let Ok((stream, _)) = listener.accept().await {
        let peer = stream
            .peer_addr()
            .expect("connected streams should have a peer address");
        info!("Peer address: {}", peer);

        tokio::spawn(accept_connection(peer, stream));
    }
}
