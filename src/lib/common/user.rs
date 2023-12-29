use crate::common::{User, UserWithPassword};
use firestore::{path, paths, FirestoreDb};
use futures::TryStreamExt;
use uuid::Uuid;

pub async fn get_user_by_id(
    db: &FirestoreDb,
    id: &Uuid,
) -> Result<Option<User>, Box<dyn std::error::Error + Send + Sync>> {
    let user: Option<User> = db
        .fluent()
        .select()
        .fields(paths!(User::{uid, email, name, role}))
        .by_id_in("users")
        .obj()
        .one(&id.to_string())
        .await?;

    Ok(user)
}

pub async fn try_find_user(
    db: &FirestoreDb,
    username: &str,
) -> Result<Option<UserWithPassword>, Box<dyn std::error::Error + Send + Sync>> {
    let object_stream = db
        .fluent()
        .select()
        .from("users")
        .filter(|q| q.for_all([q.field(path!(UserWithPassword::email)).eq(username)]))
        .limit(1)
        .obj()
        .stream_query_with_errors()
        .await?;

    let user_vec = object_stream.try_collect::<Vec<UserWithPassword>>().await?;
    Ok(user_vec.into_iter().next())
}
