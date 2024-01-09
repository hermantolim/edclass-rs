#[macro_export]
macro_rules! result_option_match {
     ($id:ident) => {
         match $id {
            Ok(Some(res)) => actix_web::HttpResponse::Ok().json(res),
            Ok(None) => actix_web::HttpResponse::NotFound().json(serde_json::json!({"error": "not found"})),
            Err(e) => actix_web::HttpResponse::InternalServerError().json(serde_json::json!({"error": format!("{:?}", e)})),
        }
     };
 }

#[macro_export]
macro_rules! result_match {
    ($id:ident) => {
        match $id {
            Ok(res) => actix_web::HttpResponse::Ok().json(res),
            Err(e) => actix_web::HttpResponse::InternalServerError().json(serde_json::json!({"error": format!("{:?}", e)})),
        }
    };
}

#[macro_export]
macro_rules! check_user {
    ($id:ident, $db:ident, $u:ident, $action:expr) => {
        match $id {
            Some(user) => match crate::common::user::get_user_by_id(&$db, &user.id).await {
                Ok(Some($u)) => $action,
                Ok(None) => actix_web::HttpResponse::Unauthorized().json(serde_json::json!({"error": "unauthorized"})),
                Err(e) => actix_web::HttpResponse::InternalServerError().json(serde_json::json!({"error": format!("{:?}", e)})),
            },
            _ => actix_web::HttpResponse::Unauthorized().json(serde_json::json!({"error": "unauthorized"})),
        }
    };
}
