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
    if !state.rooms.lock().unwrap().contains_key(&id) {
        let (tx, _) = broadcast::channel(5);
        state.rooms.lock().unwrap().insert(id, tx);
    };
    ws.on_upgrade(move |socket| handle_socket(socket, state, id))
}

async fn handle_socket(mut socket: WebSocket, state: AppState, room_id: Uuid) {
    let (mut sender, mut receiver) = socket.split();

    let mut rx = state
        .rooms
        .lock()
        .unwrap()
        .get(&room_id)
        .unwrap()
        .subscribe();

    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if sender.send(Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
    });

    let tx = state.rooms.lock().unwrap().get(&room_id).unwrap().clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(Message::Text(text))) = receiver.next().await {
            let _ = tx.send(text.to_string());
        }
    });

    tokio::select! {
        _ = &mut send_task => recv_task.abort(),
        _ = &mut recv_task => { send_task.abort(); state.rooms.lock().unwrap().remove(&room_id).unwrap();}
    };
}

// async fn get_all_rooms(
//     State(state): State<AppState>,
// ) -> Json<std::collections::HashMap<Uuid, Room>> {
//     let res = state.rooms.write().unwrap();
//
//     Json(res)
// }
