// src/api/message.rs

use crate::api::basic_auth::TokenClaims;
use crate::common::message::{try_list_messages, MessageType};
use crate::common::user::get_user_by_id;
use crate::common::{Message, MessageState, MESSAGES_COLLECTION};
use actix_web::web::ReqData;
use actix_web::{get, post, web, HttpResponse, Responder};
use chrono::Utc;
use firestore::{FirestoreDb, FirestoreResult};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct MessageBody {
    // user uuid
    pub receiver_id: Uuid,
    pub subject: Option<String>,
    pub content: String,
}

#[post("/messages")]
pub async fn send_message(
    db: web::Data<FirestoreDb>,
    req_user: Option<ReqData<TokenClaims>>,
    message: web::Json<MessageBody>,
) -> impl Responder {
    // Simulate sending a message to Firestore
    // Replace this with your actual Firestore logic

    match req_user {
        Some(user) => match get_user_by_id(&db, &user.id).await {
            Ok(Some(user_data)) => {
                let msg: MessageBody = message.into_inner();
                let message_data = Message {
                    id: Uuid::new_v4(),
                    sender_id: user_data.uid,
                    receiver_id: msg.receiver_id,
                    subject: msg.subject.to_owned(),
                    content: msg.content.to_owned(),
                    state: MessageState::Pending,
                    created_at: Utc::now(),
                };

                let req: FirestoreResult<Message> = db
                    .fluent()
                    .insert()
                    .into(MESSAGES_COLLECTION)
                    .document_id(message_data.id.to_string())
                    .object(&message_data)
                    .execute()
                    .await;

                match req {
                    Ok(_) => HttpResponse::Ok().json(json!({"success": true})),
                    Err(e) => HttpResponse::InternalServerError().json(format!("{:?}", e)),
                }
            }
            _ => HttpResponse::Unauthorized().json("Unable to verify identity"),
        },
        _ => HttpResponse::Unauthorized().json("Unable to verify identity"),
    }
}

#[get("/messages/list/inbox")]
pub async fn list_inbox(
    db: web::Data<FirestoreDb>,
    req_user: Option<ReqData<TokenClaims>>,
) -> impl Responder {
    list_messages(&db, req_user, MessageType::Received).await
}

#[get("/messages/list/sent")]
pub async fn list_sent(
    db: web::Data<FirestoreDb>,
    req_user: Option<ReqData<TokenClaims>>,
) -> impl Responder {
    list_messages(&db, req_user, MessageType::Sent).await
}

#[get("/messages/list/all")]
pub async fn list_all(
    db: web::Data<FirestoreDb>,
    req_user: Option<ReqData<TokenClaims>>,
) -> impl Responder {
    list_messages(&db, req_user, MessageType::All).await
}

async fn list_messages(
    db: &FirestoreDb,
    req_user: Option<ReqData<TokenClaims>>,
    message_type: MessageType,
) -> impl Responder {
    match req_user {
        Some(user) => {
            let list = try_list_messages(db, user, message_type).await;
            match list {
                Ok(messages) => HttpResponse::Ok().json(messages),
                Err(e) => HttpResponse::InternalServerError().json(format!("{:?}", e)),
            }
        }
        _ => HttpResponse::Unauthorized().json("Unable to verify identity"),
    }
}
