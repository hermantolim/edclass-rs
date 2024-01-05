use crate::common::course;
use actix_web::{get, web, HttpResponse, Responder};
use firestore::FirestoreDb;
use log::debug;
use serde_json::json;

#[get("/courses")]
pub async fn list_courses(db: web::Data<FirestoreDb>) -> impl Responder {
    let list = course::list_courses(&db).await;
    match list {
        Ok(r) => {
            debug!("[list_courses] {:?}", r);
            HttpResponse::Ok().json(r)
        }
        Err(e) => HttpResponse::InternalServerError().json(json!({"error": format!("{:?}", e)})),
    }
}

#[get("/courses/{course_id}")]
pub async fn get_course(db: web::Data<FirestoreDb>, path: web::Path<String>) -> impl Responder {
    let res = course::get_course(&db, path.as_str()).await;
    match res {
        Ok(Some(c)) => HttpResponse::Ok().json(c),
        Ok(None) => HttpResponse::NotFound().json(json!({"error": "not found"})),
        Err(e) => HttpResponse::InternalServerError().json(json!({"error": format!("{:?}", e)})),
    }
}
