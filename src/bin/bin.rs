use actix_web::{dev::ServiceRequest, middleware, web, App, Error, HttpMessage, HttpServer};
use actix_web_httpauth::extractors::bearer::BearerAuth;
use actix_web_httpauth::extractors::{bearer, AuthenticationError};
use actix_web_httpauth::middleware::HttpAuthentication;
use dotenv::dotenv;
use edclass_lib::api::auth::{login, register_user, TokenClaims};
use edclass_lib::api::course::{get_course, list_courses};
use edclass_lib::api::enrollment::enroll;
use edclass_lib::api::message::{
    get_message, list_all, list_inbox, list_sent, send_message, update_message_state,
};
use edclass_lib::api::user::update_devices;
use edclass_lib::common::config_env_var;
use firestore::{FirestoreDb, FirestoreResult};
use hmac::{Hmac, Mac};
use jwt::VerifyWithKey;
use sha2::Sha256;
use std::io::ErrorKind;
use std::time::Duration;

async fn setup_firestore_client() -> FirestoreResult<FirestoreDb> {
    let project_id = config_env_var("PROJECT_ID").expect("failed to load project id");
    FirestoreDb::new(&project_id).await
}

async fn validator(
    req: ServiceRequest,
    credentials: BearerAuth,
) -> Result<ServiceRequest, (Error, ServiceRequest)> {
    let jwt_secret: String = std::env::var("JWT_SECRET").expect("JWT_SECRET must be set!");
    let key: Hmac<Sha256> = Hmac::new_from_slice(jwt_secret.as_bytes()).unwrap();
    let token_string = credentials.token();

    let claims: Result<TokenClaims, &str> = token_string
        .verify_with_key(&key)
        .map_err(|_| "Invalid token");

    match claims {
        Ok(value) => {
            req.extensions_mut().insert(value);
            Ok(req)
        }
        Err(_) => {
            let config = req
                .app_data::<bearer::Config>()
                .cloned()
                .unwrap_or_default()
                .scope("");

            Err((AuthenticationError::from(config).into(), req))
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    if let None = std::env::var("RUST_LOG").ok() {
        std::env::set_var(
            "RUST_LOG",
            "edclass_lib=debug,edclass_bin=debug,actix_web=debug,actix_server=debug",
        );
    }

    env_logger::init();

    let firestore_db = setup_firestore_client()
        .await
        .map_err(|_| std::io::Error::new(ErrorKind::Other, "failed to connect firestore"))?;

    let http_client = reqwest::Client::new();

    /*let app_state = AppState {
        db: firestore_db,
        http: http_client,
    };*/

    HttpServer::new(move || {
        let bearer = HttpAuthentication::bearer(validator);
        App::new()
            .wrap(middleware::Logger::default())
            .app_data(web::Data::new(firestore_db.clone()))
            .app_data(web::Data::new(http_client.clone()))
            .service(register_user)
            .service(login)
            .service(
                web::scope("")
                    .wrap(bearer)
                    .service(send_message)
                    .service(list_inbox)
                    .service(list_sent)
                    .service(list_all)
                    .service(get_message)
                    .service(update_message_state)
                    .service(update_devices)
                    .service(list_courses)
                    .service(get_course)
                    .service(enroll),
            )
    })
    .keep_alive(Duration::from_secs(75))
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
