use crate::api::auth::TokenClaims;
use crate::api::message::MessageBody;
use crate::check_user;
use crate::common::course::get_teacher;
use crate::common::enrollment;
use crate::common::message::try_send_messages;
use crate::common::user::{get_system_user, try_get_student_parents};
use crate::common::{Enrollment, UserRole};
use actix_web::web::ReqData;
use actix_web::{post, web, HttpResponse, Responder};
use firestore::FirestoreDb;
use log::debug;
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct EnrollmentBody {
    course_id: Uuid,
}

#[post("/enrollment")]
pub async fn enroll(
    db: web::Data<FirestoreDb>,
    http: web::Data<reqwest::Client>,
    req_user: Option<ReqData<TokenClaims>>,
    data: web::Json<EnrollmentBody>,
) -> impl Responder {
    check_user!(req_user, db, u, {
        if u.role != UserRole::Student {
            return HttpResponse::BadRequest()
                .json(json!({"error": "only student can enroll for a course"}));
        }

        let data = Enrollment {
            id: Uuid::new_v4(),
            course_id: data.course_id,
            student_id: u.uid,
        };

        match enrollment::enroll(&db, &data).await {
            Ok(_) => {
                let parents = try_get_student_parents(&db, &u.uid).await;
                let teacher = get_teacher(&db, &data.course_id).await;

                match parents {
                    Ok(p) => {
                        let mut parents_email =
                            p.iter().map(|pp| pp.email.to_string()).collect::<Vec<_>>();

                        if let Ok(t) = teacher {
                            parents_email.extend_from_slice(&t.devices);
                        }

                        let sys = get_system_user(&db).await;
                        debug!("sys user {:?}", sys);
                        match sys {
                            Ok(s) => {
                                let _send = try_send_messages(
                                    &db,
                                    &http,
                                    &s,
                                    MessageBody {
                                        subject: Some("Enrollment".to_string()),
                                        receiver_ids: parents_email,
                                        content: format!(
                                            "Your kid is enrolled in course {:?}",
                                            &data
                                        ),
                                    },
                                )
                                .await;
                            }
                            Err(_e) => {
                                //
                            }
                        }
                    }
                    _ => {
                        debug!("failed to find parents for user {:?}", &u);
                    }
                }
                //send_notification_to_emails(&db, &http, &);
                HttpResponse::Ok().json(data)
            }
            Err(e) => {
                HttpResponse::InternalServerError().json(json!({"error": format!("{:?}", e)}))
            }
        }
    })
}
