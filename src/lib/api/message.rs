// src/api/message.rs

use actix_web::web::ReqData;
use actix_web::{get, post, web, HttpResponse, Responder};

use firestore::struct_path::paths;
use firestore::{FirestoreDb, FirestoreResult};

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::api::auth::TokenClaims;
use crate::common::message::{try_list_messages, try_send_messages, MessageType};
use crate::common::user::get_user_by_id;
use crate::common::{Message, MessageState, MESSAGES_COLLECTION};
use crate::{check_user, result_option_match};

#[derive(Debug, Serialize, Deserialize)]
pub struct MessageBody {
    // user uuid
    pub receiver_ids: Vec<String>,
    pub subject: Option<String>,
    pub content: String,
}

#[post("/messages")]
pub async fn send_message(
    db: web::Data<FirestoreDb>,
    http: web::Data<reqwest::Client>,
    req_user: Option<ReqData<TokenClaims>>,
    message: web::Json<MessageBody>,
) -> impl Responder {
    match req_user {
        Some(user) => match get_user_by_id(&db, &user.id).await {
            Ok(Some(user_data)) => {
                let msg: MessageBody = message.into_inner();
                match try_send_messages(&db, &http, &user_data, msg).await {
                    Ok(_) => HttpResponse::Ok().into(),
                    Err(e) => HttpResponse::InternalServerError()
                        .json(json!({"error": format!("{:?}", e)})),
                }
            }
            _ => HttpResponse::Unauthorized().json(json!({"error": "unable to verify identity"})),
        },
        _ => HttpResponse::Unauthorized().json(json!({"error": "unable to verify identity"})),
    }
}

#[get("/messages/{message_id}")]
pub async fn get_message(db: web::Data<FirestoreDb>, path: web::Path<String>) -> impl Responder {
    let data: FirestoreResult<Option<Message>> = db
        .fluent()
        .select()
        .by_id_in(MESSAGES_COLLECTION)
        .obj()
        .one(path.as_ref())
        .await;

    result_option_match!(data)
}

#[derive(Debug, Deserialize)]
pub struct UpdateMessageStateBody {
    state: MessageState,
}

#[post("/messages/{message_id}/state")]
pub async fn update_message_state(
    db: web::Data<FirestoreDb>,
    path: web::Path<String>,
    body: web::Json<UpdateMessageStateBody>,
) -> impl Responder {
    let obj_by_id: FirestoreResult<Option<Message>> = db
        .fluent()
        .select()
        .by_id_in(MESSAGES_COLLECTION)
        .obj()
        .one(path.as_ref())
        .await;

    match obj_by_id {
        Ok(Some(m)) => {
            let update: FirestoreResult<Message> = db
                .fluent()
                .update()
                .fields(paths!(Message::{state}))
                .in_col(MESSAGES_COLLECTION)
                .document_id(path.as_ref())
                .object(&Message {
                    state: body.into_inner().state,
                    ..m.clone()
                })
                .execute()
                .await;
            match update {
                Ok(_) => HttpResponse::Ok().into(),
                Err(e) => {
                    HttpResponse::InternalServerError().json(json!({"error": format!("{:?}", e)}))
                }
            }
        }
        Ok(None) => HttpResponse::NotFound().json(json!({"error": "not found"})),
        Err(e) => HttpResponse::InternalServerError().json(json!({"error": format!("{:?}", e)})),
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
    check_user!(req_user, db, u, {
        let list = try_list_messages(db, &u, message_type).await;
        match list {
            Ok(messages) => HttpResponse::Ok().json(messages),
            Err(e) => {
                HttpResponse::InternalServerError().json(json!({"error": format!("{:?}", e)}))
            }
        }
    })
}
