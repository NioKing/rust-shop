use std::net::SocketAddr;

use crate::utils::types::AppState;
use axum::body::Bytes;
use axum::extract::Request;
use axum::extract::connect_info::ConnectInfo;
use axum::extract::ws::{CloseFrame, Message};
use axum::response::IntoResponse;
use axum::{
    extract::{
        State,
        ws::{WebSocket, WebSocketUpgrade},
    },
    response::Response,
};
use axum_extra::{TypedHeader, headers};
use futures_util::StreamExt;

pub async fn handler(
    ws: WebSocketUpgrade,
    State(state): State<crate::utils::types::AppState>,
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: AppState) {
    // let (mut sender, mut receiver) = socket.split();

    while let Some(msg) = socket.recv().await {
        let msg = if let Ok(msg) = msg {
            println!("message: {:?}", msg.to_text().unwrap());
            msg
        } else {
            // client disconnected
            return;
        };

        if socket.send(msg).await.is_err() {
            // client disconnected
            return;
        }
    }
}
