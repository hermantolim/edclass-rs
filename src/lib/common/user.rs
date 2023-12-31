use crate::api::basic_auth::TokenClaims;
use crate::common::{
    StudentsParents, User, UserWithPassword, UsersDevices, STUDENTS_PARENTS_COLLECTION,
    USERS_COLLECTION, USERS_DEVICES_COLLECTION,
};
use actix_web::web::ReqData;
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
        .by_id_in(USERS_COLLECTION)
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
        .from(USERS_COLLECTION)
        .filter(|q| q.for_all([q.field(path!(UserWithPassword::email)).eq(username)]))
        .limit(1)
        .obj()
        .stream_query_with_errors()
        .await?;

    let user_vec = object_stream.try_collect::<Vec<UserWithPassword>>().await?;
    Ok(user_vec.into_iter().next())
}

pub async fn save_user_to_db(
    db: &FirestoreDb,
    user: &UserWithPassword,
    students: Option<Vec<Uuid>>,
) -> Result<(), Box<dyn std::error::Error>> {
    db.fluent()
        .insert()
        .into(USERS_COLLECTION)
        .document_id(&user.uid.to_string())
        .object(user)
        .execute()
        .await?;

    match students {
        Some(ids) => {
            let batch_writer = db.create_simple_batch_writer().await?;
            let mut current_batch = batch_writer.new_batch();
            for id in ids {
                let s_p = StudentsParents {
                    student_id: id,
                    parent_id: user.uid,
                };

                db.fluent()
                    .update()
                    .in_col(STUDENTS_PARENTS_COLLECTION)
                    .document_id(&format!("{}_{}", &s_p.student_id, &s_p.parent_id))
                    .object(&s_p)
                    .add_to_batch(&mut current_batch)?;
            }

            let response = current_batch.write().await?;
            println!("{:?}", response);
        }
        _ => {
            //
        }
    }

    Ok(())
}

pub async fn try_add_device(
    db: &FirestoreDb,
    req: ReqData<TokenClaims>,
    device_id: String,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match get_user_by_id(db, &req.id).await? {
        Some(user) => {
            db.fluent()
                .insert()
                .into(USERS_DEVICES_COLLECTION)
                .document_id(&device_id)
                .object(&UsersDevices {
                    id: device_id,
                    user_id: user.uid,
                })
                .execute()
                .await?;

            Ok(())
        }
        _ => Err(Box::from(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "data not found".to_string(),
        ))),
    }
}
