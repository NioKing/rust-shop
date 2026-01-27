use std::net::SocketAddr;

use super::models::{ChatMessage, Room};
use crate::auth::models::AccessTokenClaims;
use crate::utils::types::AppState;
use axum::Json;
use axum::body::Bytes;
use axum::extract::connect_info::ConnectInfo;
use axum::extract::ws::{CloseFrame, Message};
use axum::extract::{Path, Request};
use axum::response::IntoResponse;
use axum::{
    extract::{
        State,
        ws::{WebSocket, WebSocketUpgrade},
    },
    response::Response,
};
use axum_extra::{TypedHeader, headers};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::broadcast;
use uuid::Uuid;

pub async fn handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    claims: AccessTokenClaims,
) -> Response {
    {
        let mut rooms = state.rooms.lock().await;
        rooms.entry(id).or_insert_with(|| {
            let (tx, _) = broadcast::channel(5);
            tx
        });
    }
    ws.on_upgrade(move |socket| handle_socket(socket, state, id, claims.email))
}

async fn handle_socket(mut socket: WebSocket, state: AppState, room_id: Uuid, email: String) {
    let (mut sender, mut receiver) = socket.split();

    let mut tx = {
        let rooms = state.rooms.lock().await;
        rooms.get(&room_id).unwrap().clone()
    };

    let mut rx = tx.subscribe();

    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            let json = match serde_json::to_string(&msg) {
                Ok(j) => j,
                Err(e) => {
                    tracing::debug!("error : {}", e);
                    continue;
                }
            };
            if sender.send(Message::Text(json.into())).await.is_err() {
                break;
            }
        }
    });

    let tx = state.rooms.lock().await.get(&room_id).unwrap().clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(Message::Text(text))) = receiver.next().await {
            let msg = ChatMessage::Chat {
                email: email.to_string(),
                text: text.to_string(),
            };
            let _ = tx.send(msg);
        }
    });

    tokio::select! {
        _ = &mut send_task => recv_task.abort(),
        _ = &mut recv_task => send_task.abort()
    };

    room_cleanup(&state, &room_id).await;
}

async fn room_cleanup(state: &AppState, room_id: &Uuid) {
    let mut rooms = state.rooms.lock().await;

    if let Some(tx) = rooms.get(room_id) {
        if tx.receiver_count() == 0 {
            rooms.remove(room_id);
        }
    }
}
