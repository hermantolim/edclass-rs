use crate::api::auth::TokenClaims;
use crate::common::course;
use crate::{check_user, result_option_match};
use actix_web::web::ReqData;
use actix_web::{get, web, HttpResponse, Responder};
use firestore::FirestoreDb;
use serde_json::json;

#[get("/courses")]
pub async fn list_courses(
    db: web::Data<FirestoreDb>,
    req_user: Option<ReqData<TokenClaims>>,
) -> impl Responder {
    check_user!(req_user, db, u, {
        match course::list_courses(&db, &u).await {
            Ok(c) => HttpResponse::Ok().json(c),
            Err(e) => {
                HttpResponse::InternalServerError().json(json!({"error": format!("{:?}", e)}))
            }
        }
    })
}

#[get("/courses/{course_id}")]
pub async fn get_course(
    db: web::Data<FirestoreDb>,
    req_user: Option<ReqData<TokenClaims>>,
    path: web::Path<String>,
) -> impl Responder {
    check_user!(req_user, db, u, {
        let res = course::get_course(&db, &u, path.as_str()).await;
        result_option_match!(res)
    })
}
