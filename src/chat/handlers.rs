use std::net::SocketAddr;

use super::models::Room;
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
) -> Response {
    {
        let mut rooms = state.rooms.lock().await;
        rooms.entry(id).or_insert_with(|| {
            let (tx, _) = broadcast::channel(5);
            tx
        });
    }
    ws.on_upgrade(move |socket| handle_socket(socket, state, id))
}

async fn handle_socket(mut socket: WebSocket, state: AppState, room_id: Uuid) {
    let (mut sender, mut receiver) = socket.split();

    let mut tx = {
        let rooms = state.rooms.lock().await;
        rooms.get(&room_id).unwrap().clone()
    };

    let mut rx = tx.subscribe();

    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if sender.send(Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
    });

    let tx = state.rooms.lock().await.get(&room_id).unwrap().clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(Message::Text(text))) = receiver.next().await {
            let _ = tx.send(text.to_string());
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

// async fn get_all_rooms(
//     State(state): State<AppState>,
// ) -> Json<std::collections::HashMap<Uuid, Room>> {
//     let res = state.rooms.write().unwrap();
//
//     Json(res)
// }
