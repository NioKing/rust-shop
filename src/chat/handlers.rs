use std::net::SocketAddr;
use std::rc::Rc;
use std::sync::Arc;

use super::models::{ActiveRoom, ChatMessage, Room, RoomStatus};
use crate::auth::models::AccessTokenClaims;
use crate::error::AppError;
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
    // claims: AccessTokenClaims,
) -> Response {
    let email = "some@email.com";
    {
        let mut rooms = state.rooms.lock().await;
        rooms.entry(id).or_insert_with(|| {
            let (tx, _) = broadcast::channel(5);
            Room {
                tx,
                id,
                members: 0,
                status: RoomStatus::Created,
            }
        });
    }
    ws.on_upgrade(move |socket| handle_socket(socket, state, id, email.to_string()))
}

async fn handle_socket(mut socket: WebSocket, state: AppState, room_id: Uuid, email: String) {
    let (mut sender, mut receiver) = socket.split();

    let tx = {
        let rooms = state.rooms.lock().await;
        let room = rooms.get(&room_id).unwrap();
        room.tx.clone()
    };

    let mut rx = tx.subscribe();

    let _ = tx.send(ChatMessage::Join {
        email: email.clone(),
        room_id,
    });

    {
        let mut rooms = state.rooms.lock().await;
        let room = rooms.get_mut(&room_id).unwrap();
        room.members += 1;
        println!("members: {}", room.members);
    }

    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            let json = match serde_json::to_string(&msg) {
                Ok(j) => j,
                Err(e) => {
                    tracing::debug!("error : {:?}", e);
                    continue;
                }
            };
            if sender.send(Message::Text(json.into())).await.is_err() {
                break;
            }
        }
    });

    let mut recv_task = {
        let tx = tx.clone();
        let email = email.clone();

        tokio::spawn(async move {
            while let Some(Ok(Message::Text(text))) = receiver.next().await {
                let msg = ChatMessage::Chat {
                    email: email.clone(),
                    text: text.to_string(),
                };
                let _ = tx.send(msg);
            }
        })
    };

    tokio::select! {
        _ = &mut send_task => recv_task.abort(),
        _ = &mut recv_task => send_task.abort()
    };

    let _ = tx.send(ChatMessage::Disconnect {
        email: email.clone(),
        room_id,
    });

    let should_cleanup = {
        let mut rooms = state.rooms.lock().await;
        let room = rooms.get_mut(&room_id).unwrap();
        room.members -= 1;
        println!("members: {}", room.members);

        room.members == 0
    };

    if should_cleanup {
        room_cleanup(&state, &room_id).await;
        println!("room deleted");
    }
}

async fn room_cleanup(state: &AppState, room_id: &Uuid) {
    let mut rooms = state.rooms.lock().await;

    rooms.remove(room_id);
}

pub async fn get_all_rooms(
    State(state): State<AppState>,
) -> Result<Json<Vec<ActiveRoom>>, AppError> {
    let rooms = state
        .rooms
        .lock()
        .await
        .iter()
        .map(|(id, room)| ActiveRoom {
            id: *id,
            members: room.members,
            status: room.status,
        })
        .collect::<Vec<_>>();

    Ok(Json(rooms))
}
