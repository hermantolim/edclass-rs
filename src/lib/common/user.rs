use crate::api::auth::TokenClaims;
use crate::common::{
    StudentsParents, User, UserRole, UserWithPassword, UserWithPasswordStudents,
    STUDENTS_PARENTS_COLLECTION, USERS_COLLECTION,
};
use actix_web::web::ReqData;
use argonautica::Hasher;
use firestore::{path, paths, FirestoreDb};
use futures::TryStreamExt;
use serde::Serialize;
use uuid::Uuid;

pub async fn get_user_by_id(
    db: &FirestoreDb,
    id: &Uuid,
) -> Result<Option<User>, Box<dyn std::error::Error + Send + Sync>> {
    let user: Option<User> = db
        .fluent()
        .select()
        //.fields(paths!(User::{uid, email, name, role, devices}))
        .by_id_in(USERS_COLLECTION)
        .obj()
        .one(&id.to_string())
        .await?;

    Ok(user)
}

pub async fn try_find_user(
    db: &FirestoreDb,
    email_or_id: &str,
) -> Result<Option<UserWithPassword>, Box<dyn std::error::Error + Send + Sync>> {
    let users: Vec<UserWithPassword> = db
        .fluent()
        .select()
        .from(USERS_COLLECTION)
        .filter(|q| {
            q.for_any([
                q.field(path!(UserWithPassword::email)).eq(email_or_id),
                q.field(path!(UserWithPassword::uid)).eq(email_or_id),
            ])
        })
        .limit(1)
        .obj()
        .query()
        .await?;

    Ok(users.into_iter().next())
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
            let mut devices = user.devices.clone();
            if !devices.contains(&device_id) {
                devices.push(device_id);
            }

            db.fluent()
                .update()
                .fields(paths!(User::{devices}))
                .in_col(USERS_COLLECTION)
                .document_id(&user.uid.to_string())
                .object(&User {
                    devices,
                    ..user.clone()
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

pub fn make_user(user: &UserWithPasswordStudents) -> (UserWithPassword, Option<Vec<Uuid>>) {
    let hash_secret = std::env::var("HASH_SECRET").expect("HASH_SECRET must be set!");
    let mut hasher = Hasher::default();
    let hash = hasher
        .with_password(&user.password)
        .with_secret_key(hash_secret)
        .hash()
        .unwrap();

    let (role, kids) = match &user.role {
        UserRole::Parent => (UserRole::Parent, user.students.as_ref().cloned()),
        UserRole::Student => (UserRole::Student, None),
        UserRole::Teacher => (UserRole::Teacher, None),
        UserRole::Admin => (UserRole::Admin, None),
        UserRole::System => (UserRole::System, None),
    };

    (
        UserWithPassword {
            uid: Uuid::new_v4(),
            email: user.email.clone(),
            role,
            name: user.name.clone(),
            password: hash,
            devices: Vec::new(),
        },
        kids,
    )
}

pub async fn try_get_users_from_emails<T: AsRef<str> + Serialize>(
    db: &FirestoreDb,
    emails: &[T],
) -> Result<Vec<User>, Box<dyn std::error::Error>> {
    let box_receivers = db
        .fluent()
        .select()
        .from(USERS_COLLECTION)
        .filter(|q| q.for_any([q.field(path!(User::email)).is_in(emails)]))
        .obj()
        .stream_query_with_errors()
        .await?;

    let receivers: Vec<User> = box_receivers.try_collect().await?;
    Ok(receivers)
}

pub async fn try_get_student_parents<T: ToString>(
    db: &FirestoreDb,
    user_id: T,
) -> Result<Vec<User>, Box<dyn std::error::Error>> {
    let box_students_parents = db
        .fluent()
        .select()
        .from(STUDENTS_PARENTS_COLLECTION)
        .filter(|q| {
            q.for_all([
                //
                q.field(
                    //
                    path!(StudentsParents::student_id),
                )
                .eq(&user_id.to_string()),
            ])
        })
        .obj()
        .stream_query_with_errors()
        .await?;

    let students_parents: Vec<StudentsParents> = box_students_parents.try_collect().await?;
    let parents_ids = students_parents
        .as_slice()
        .iter()
        .map(|sp| sp.parent_id.to_string())
        .collect::<Vec<_>>();

    let box_users = db
        .fluent()
        .select()
        .from(USERS_COLLECTION)
        .filter(|q| {
            q.for_all([
                //
                q.field(
                    //
                    path!(User::uid),
                )
                .is_in(parents_ids.as_slice()),
            ])
        })
        .obj()
        .stream_query_with_errors()
        .await?;
    let parents: Vec<User> = box_users.try_collect().await?;
    Ok(parents)
}

pub async fn get_system_user(db: &FirestoreDb) -> Result<User, Box<dyn std::error::Error>> {
    let obj_stream = db
        .fluent()
        .select()
        .from(USERS_COLLECTION)
        .limit(1)
        .filter(|q| q.for_all(q.field(path!(User::role)).eq(&UserRole::System)))
        .obj()
        .stream_query_with_errors()
        .await?;

    let to_vec: Vec<User> = obj_stream.try_collect().await?;
    match to_vec.into_iter().next() {
        Some(u) => Ok(u),
        _ => Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "user not found",
        ))),
    }
}
