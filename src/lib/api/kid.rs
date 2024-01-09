use crate::api::auth::TokenClaims;
use crate::common::UserRole;
use crate::{check_user, common, result_match};
use actix_web::web::{Data, ReqData};
use actix_web::{get, HttpResponse, Responder};
use firestore::FirestoreDb;
use serde_json::json;
#[get("/kids")]
pub async fn get_kids(
    db: Data<FirestoreDb>,
    req_user: Option<ReqData<TokenClaims>>,
) -> impl Responder {
    check_user!(req_user, db, u, {
        if u.role != UserRole::Parent {
            return HttpResponse::BadRequest().json(json!({"error": "not a parent"}));
        }
        let res = common::user::get_kids(&db, &u).await;
        result_match!(res)
    })
}
