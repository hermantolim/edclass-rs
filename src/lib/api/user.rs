// src/api/user
use crate::api::basic_auth::TokenClaims;
use crate::common::user::{save_user_to_db, try_add_device};
use crate::common::{UserRole, UserWithPassword};
use actix_web::web::ReqData;
use actix_web::{post, web, HttpResponse, Responder};
use argonautica::Hasher;
use firestore::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateUser {
    name: String,
    email: String,
    password: String,
    role: UserRole,
    students: Option<Vec<Uuid>>,
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

    match save_user_to_db(&db, &user, kids).await {
        Ok(_) => HttpResponse::Ok().json(user),
        Err(e) => HttpResponse::InternalServerError().json(format!("{:?}", e)),
    }
}

#[derive(Deserialize)]
pub struct UpdateDevicesBody {
    device_token: String,
}

#[post("/users/devices")]
pub async fn update_devices(
    db: web::Data<FirestoreDb>,
    req_user: Option<ReqData<TokenClaims>>,
    body: web::Json<UpdateDevicesBody>,
) -> impl Responder {
    match req_user {
        Some(user) => {
            let res = try_add_device(&db, user, body.into_inner().device_token).await;
            match res {
                Ok(_) => HttpResponse::Ok().json(json!({"success": true})),
                Err(e) => HttpResponse::InternalServerError().json(format!("{:?}", e)),
            }
        }
        _ => HttpResponse::Unauthorized().json("Unable to verify identity"),
    }
}
