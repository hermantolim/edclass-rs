// src/api/user
use crate::api::auth::TokenClaims;
use crate::common::user::try_add_device;
use crate::common::UserRole;
use actix_web::web::ReqData;
use actix_web::{post, web, HttpResponse, Responder};
use firestore::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateUserBody {
    name: String,
    email: String,
    password: String,
    role: UserRole,
    students: Option<Vec<Uuid>>,
}
/*
#[post("/users")]
pub async fn create_user(
    db: web::Data<FirestoreDb>,
    info: web::Json<CreateUserBody>,
) -> impl Responder {
    let inner = info.into_inner();
    let (user, kids) = make_user(&UserWithPasswordStudents {
        name: inner.name,
        students: inner.students,
        email: inner.email,
        password: inner.password,
        role: inner.role,
    });

    // Create the corresponding role based on the user's input
    match save_user_to_db(&db, &user, kids).await {
        Ok(_) => HttpResponse::Ok().json(user),
        Err(e) => HttpResponse::InternalServerError().json(json!({"error": format!("{:?}", e)})),
    }
}*/

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
                Err(e) => {
                    HttpResponse::InternalServerError().json(json!({"error": format!("{:?}", e)}))
                }
            }
        }
        _ => HttpResponse::Unauthorized().json(json!({"error": "unable to verify identity"})),
    }
}
