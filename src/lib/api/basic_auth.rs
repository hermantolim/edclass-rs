use crate::common::user::try_find_user;
use actix_web::web::Data;
use actix_web::{get, web, HttpResponse, Responder};
use actix_web_httpauth::extractors::basic::BasicAuth;
use argonautica::Verifier;
use firestore::FirestoreDb;
use hmac::{Hmac, Mac};
use jwt::SignWithKey;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone)]
pub struct TokenClaims {
    pub id: Uuid,
}

#[derive(Deserialize, Clone)]
pub struct DeviceIdInfo {
    pub device_id: Option<String>,
}
#[get("/auth")]
pub async fn basic_auth(
    db: Data<FirestoreDb>,
    credentials: BasicAuth,
    device_info: web::Json<DeviceIdInfo>,
) -> impl Responder {
    let jwt_secret: Hmac<Sha256> = Hmac::new_from_slice(
        std::env::var("JWT_SECRET")
            .expect("JWT_SECRET must be set!")
            .as_bytes(),
    )
    .unwrap();
    let username = credentials.user_id();
    let password = credentials.password();

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
                        HttpResponse::Ok().json(token_str)
                    } else {
                        HttpResponse::Unauthorized().json("Incorrect username or password")
                    }
                }
                _ => HttpResponse::Unauthorized().json("user not found"),
            }
        }
    }
}
