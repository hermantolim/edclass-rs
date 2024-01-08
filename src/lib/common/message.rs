use crate::api::auth::TokenClaims;
use crate::api::message::MessageBody;
use crate::common::user::get_user_by_id;
use crate::common::{
    send_notification_to_emails, Message, MessageState, User, MESSAGES_COLLECTION,
};
use actix_web::web::ReqData;

use chrono::Utc;
use firestore::{path, FirestoreDb, FirestoreQueryDirection, FirestoreResult};
use futures::stream::BoxStream;
use futures::TryStreamExt;
use log::debug;

use uuid::Uuid;

pub enum MessageType {
    Received,
    Sent,
    All,
}
pub async fn try_list_messages(
    db: &FirestoreDb,
    user: &User,
    message_type: MessageType,
) -> Result<Vec<Message>, Box<dyn std::error::Error + Send + Sync>> {
    let objs_stream: BoxStream<FirestoreResult<Message>> = db
        .fluent()
        .select()
        .from(MESSAGES_COLLECTION)
        .filter(|q| match message_type {
            MessageType::Received => q.for_all([q
                .field("receiver_ids")
                .array_contains(user.email.to_string())]),
            MessageType::Sent => q.for_all([q.field("sender_id").eq(user.uid.to_string())]),
            MessageType::All => q.for_any([
                q.field("receiver_ids")
                    .array_contains(user.email.to_string()),
                q.field("sender_id").eq(user.uid.to_string()),
            ]),
        })
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

pub async fn try_send_messages(
    db: &FirestoreDb,
    http: &reqwest::Client,
    user: &User,
    msg: MessageBody,
) -> Result<(), Box<dyn ::std::error::Error>> {
    let message_data = Message {
        id: Uuid::new_v4(),
        sender_id: user.uid.clone(),
        receiver_ids: msg.receiver_ids,
        subject: msg.subject.to_owned(),
        content: msg.content.to_owned(),
        state: MessageState::Pending,
        created_at: Utc::now(),
    };

    debug!("message {:?}", message_data);

    let req: FirestoreResult<Message> = db
        .fluent()
        .insert()
        .into(MESSAGES_COLLECTION)
        .document_id(message_data.id.to_string())
        .object(&message_data)
        .execute()
        .await;

    match req {
        Ok(_) => {
            let _send = send_notification_to_emails(
                &db,
                &http,
                message_data.receiver_ids.as_slice(),
                message_data.subject.as_deref(),
                &message_data.content,
            )
            .await?;
            Ok(())
        }
        Err(e) => Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            format!("{:?}", e),
        ))),
    }
}
