use crate::api::basic_auth::TokenClaims;
use crate::common::user::get_user_by_id;
use crate::common::{Message, MESSAGES_COLLECTION};
use actix_web::web::ReqData;
use firestore::{path, FirestoreDb, FirestoreQueryDirection, FirestoreResult};
use futures::stream::BoxStream;
use futures::TryStreamExt;

pub enum MessageType {
    Received,
    Sent,
    All,
}
pub async fn try_list_messages(
    db: &FirestoreDb,
    req: ReqData<TokenClaims>,
    message_type: MessageType,
) -> Result<Vec<Message>, Box<dyn std::error::Error + Send + Sync>> {
    match get_user_by_id(db, &req.id).await? {
        Some(user) => {
            let objs_stream: BoxStream<FirestoreResult<Message>> = db
                .fluent()
                .select()
                .from(MESSAGES_COLLECTION)
                .filter(|q| match message_type {
                    MessageType::Received => {
                        q.for_all([q.field("receiver_id").eq(user.uid.to_string())])
                    }
                    MessageType::Sent => q.for_all([q.field("sender_id").eq(user.uid.to_string())]),
                    MessageType::All => q.for_all([
                        q.field("receiver_id").eq(user.uid.to_string()),
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
        _ => Err(Box::from(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "data not found".to_string(),
        ))),
    }
}
