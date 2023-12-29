// src/api/message.rs

use crate::api::basic_auth::TokenClaims;
use crate::common::user::get_user_by_id;
use crate::common::{Message, MessageState, MESSAGES_COLLECTION};
use actix_web::web::ReqData;
use actix_web::{post, web, HttpResponse, Responder};
use chrono::Utc;
use firestore::{path, FirestoreDb, FirestoreQueryDirection, FirestoreResult};
use futures::stream::BoxStream;
use futures::TryStreamExt;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

// Placeholder function for listing messages

async fn try_list_messages(
    db: &FirestoreDb,
    req: ReqData<TokenClaims>,
) -> Result<Vec<Message>, Box<dyn std::error::Error + Send + Sync>> {
    match get_user_by_id(db, &req.id).await? {
        Some(user) => {
            let objs_stream: BoxStream<FirestoreResult<Message>> = db
                .fluent()
                .select()
                .from(MESSAGES_COLLECTION)
                .filter(|q| q.for_all([q.field("receiver_id").eq(user.uid.to_string())]))
                .order_by([(
                    path!(Message::created_at),
                    FirestoreQueryDirection::Descending,
                )])
                .obj()
                .stream_query_with_errors()
                .await?;
            let as_vec: Vec<Message> = objs_stream.try_collect().await?;
            Ok(as_vec)
        }
        _ => Err(Box::from(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "data not found".to_string(),
        ))),
    }
}

pub async fn list_messages(
    db: web::Data<FirestoreDb>,
    req: ReqData<TokenClaims>,
) -> impl Responder {
    let messages = try_list_messages(&db, req).await.map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            "failed to list messages".to_string(),
        )
    })?;

    Ok::<_, std::io::Error>(web::Json(messages))
}

// Placeholder function for sending a message

#[derive(Debug, Serialize, Deserialize)]
pub struct MessageBody {
    // user uuid
    pub receiver_id: Uuid,
    pub subject: Option<String>,
    pub content: String,
}

#[post("/message")]
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
