// src/api/create_user.rs
use crate::common::{StudentsParents, UserRole, UserWithPassword};
use actix_web::{post, web, Responder};
use argonautica::Hasher;
use firestore::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateUser {
    name: String,
    email: String,
    password: String,
    role: UserRole,
    students: Option<Vec<Uuid>>,
}

async fn save_user_to_firestore(
    db: &FirestoreDb,
    user: &UserWithPassword,
    students: Option<Vec<Uuid>>,
) -> Result<(), Box<dyn std::error::Error>> {
    db.fluent()
        .insert()
        .into("users")
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
                    .in_col("students-parents")
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

#[post("/users")]
pub async fn create_user(
    db: web::Data<FirestoreDb>,
    info: web::Json<CreateUser>,
) -> impl Responder {
    let hash_secret = std::env::var("HASH_SECRET").expect("HASH_SECRET must be set!");
    let mut hasher = Hasher::default();
    let hash = hasher
        .with_password(&info.password)
        .with_secret_key(hash_secret)
        .hash()
        .unwrap();

    let (role, kids) = match &info.role {
        UserRole::Parent => (UserRole::Parent, info.students.as_ref().cloned()),
        UserRole::Student => (UserRole::Student, None),
        UserRole::Teacher => (UserRole::Teacher, None),
        UserRole::Admin => (UserRole::Admin, None),
        UserRole::System => (UserRole::System, None),
    };

    let user = UserWithPassword {
        uid: Uuid::new_v4(),
        email: info.email.clone(),
        role,
        name: info.name.clone(),
        password: hash,
    };
    // Create the corresponding role based on the user's input

    save_user_to_firestore(&db, &user, kids)
        .await
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "save users error"))?;
    println!("Created user: {:?}", user);

    // Respond with the created user (excluding the password in the response)
    Ok::<_, std::io::Error>(web::Json(user))
}
