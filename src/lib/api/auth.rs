use crate::common::user::{make_user, save_user_to_db, try_find_user};
use crate::common::{User, UserRole, UserWithPasswordStudents};
use actix_web::web::Data;
use actix_web::{get, post, web, HttpResponse, Responder};
use actix_web_httpauth::extractors::basic::BasicAuth;
use argonautica::Verifier;
use firestore::FirestoreDb;
use hmac::{Hmac, Mac};
use jwt::SignWithKey;
use log::debug;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::Sha256;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone)]
pub struct TokenClaims {
    pub id: Uuid,
}

#[derive(Serialize, Clone)]
pub struct AuthResponse {
    token: String,
    user: User,
}

#[derive(Deserialize, Clone)]
pub struct DeviceIdInfo {
    pub device_id: Option<String>,
}
#[get("/auth")]
pub async fn login(db: Data<FirestoreDb>, credentials: BasicAuth) -> impl Responder {
    let jwt_secret: Hmac<Sha256> = Hmac::new_from_slice(
        std::env::var("JWT_SECRET")
            .expect("JWT_SECRET must be set!")
            .as_bytes(),
    )
    .unwrap();
    let username = credentials.user_id();
    let password = credentials.password();

    debug!("credentials {:?} {:?}", username, password);

    match password {
        None => HttpResponse::Unauthorized().json("Must provide username and password"),
        Some(pass) => {
            let user_data = try_find_user(&db, username).await;

            match user_data {
                Ok(Some(user)) => {
                    let hash_secret =
                        std::env::var("HASH_SECRET").expect("HASH_SECRET must be set!");
                    let mut verifier = Verifier::default();
                    let is_valid = verifier
                        .with_hash(user.password)
                        .with_password(pass)
                        .with_secret_key(hash_secret)
                        .verify()
                        .unwrap();

                    if is_valid {
                        let claims = TokenClaims { id: user.uid };
                        let token_str = claims.sign_with_key(&jwt_secret).unwrap();
                        HttpResponse::Ok().json(AuthResponse {
                            user: User {
                                role: user.role,
                                email: user.email,
                                name: user.name,
                                uid: user.uid,
                                devices: user.devices,
                            },
                            token: token_str,
                        })
                    } else {
                        HttpResponse::Unauthorized()
                            .json(json!({"error": "incorrect username or password"}))
                    }
                }
                _ => HttpResponse::NotFound().json(json!({"error": "user not found"})),
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterUserBody {
    name: String,
    email: String,
    password: String,
    confirm_password: String,
    role: UserRole,
    students: Option<Vec<Uuid>>,
}

#[post("/auth/register")]
pub async fn register_user(
    db: web::Data<FirestoreDb>,
    info: web::Json<RegisterUserBody>,
) -> impl Responder {
    if info.password != info.confirm_password {
        return HttpResponse::NotAcceptable().json(json!({"error": "password do not match"}));
    }

    match try_find_user(&db, info.email.as_str()).await {
        Ok(Some(_)) => {
            return HttpResponse::BadRequest().json(json!({"error": "user exists"}));
        }
        _ => {
            //
        }
    }

    let inner = info.into_inner();
    let (user, kids) = make_user(&UserWithPasswordStudents {
        password: inner.password,
        email: inner.email,
        role: inner.role,
        students: inner.students,
        name: inner.name,
    });

    match save_user_to_db(&db, &user, kids).await {
        Ok(_) => HttpResponse::Ok().json(user),
        Err(e) => HttpResponse::InternalServerError().json(json!({"error": format!("{:?}", e)})),
    }
}
