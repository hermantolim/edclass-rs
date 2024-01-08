use crate::common::enrollment::list_user_enrolled_in;
use crate::common::user::get_user_by_id;
use crate::common::{
    Course, CourseEnrollment, CourseResponse, Enrollment, User, UserRole, COURSES_COLLECTION,
    ENROLLMENTS_COLLECTION,
};
use firestore::struct_path::path;
use firestore::{FirestoreDb, FirestoreResult};
use futures::stream::BoxStream;
use futures::StreamExt;

pub async fn list_courses(
    db: &FirestoreDb,
    user: &User,
) -> Result<Vec<CourseEnrollment>, Box<dyn std::error::Error>> {
    let mut box_courses: BoxStream<FirestoreResult<Course>> = db
        .fluent()
        .list()
        .from(COURSES_COLLECTION)
        .obj()
        .stream_all_with_errors()
        .await?;

    let mut courses: Vec<CourseEnrollment> = Vec::new();
    while let Some(Ok(c)) = box_courses.next().await {
        if user.role == UserRole::Student {
            let enrollment: Vec<Enrollment> = db
                .fluent()
                .select()
                .from(ENROLLMENTS_COLLECTION)
                .filter(|q| {
                    // filter query
                    q.for_all([
                        // query enroll id
                        q.field(path!(Enrollment::course_id)).eq(&c.id.to_string()),
                        q.field(path!(Enrollment::student_id))
                            .eq(&user.uid.to_string()),
                    ])
                })
                .limit(1)
                .obj()
                .query()
                .await?;

            courses.push(CourseEnrollment {
                course: c,
                enrolled: enrollment.iter().next().is_some(),
            })
        } else {
            courses.push(CourseEnrollment {
                course: c,
                enrolled: false,
            })
        }
    }
    //box_courses.map(|c: Course| {}).try_collect().await?;
    //let courses = box_courses.try_collect().await?;

    Ok(courses)
}

pub async fn get_course(
    db: &FirestoreDb,
    user: &User,
    id: &str,
) -> Result<Option<CourseResponse>, Box<dyn std::error::Error + Send + Sync>> {
    let select_course: Vec<Course> = db
        .fluent()
        .select()
        .from(COURSES_COLLECTION)
        .filter(|q| q.for_any([q.field(path!(Course::id)).eq(id)]))
        .limit(1)
        .obj()
        .query()
        .await?;

    let course = select_course.into_iter().next();

    match course {
        Some(c) => {
            let students = list_user_enrolled_in(db, &c.id).await?;
            let teacher = get_user_by_id(db, &c.teacher_id).await?;

            match teacher {
                Some(t) => {
                    if user.role == UserRole::Student {
                        let enrolled = students.iter().find(|s| s.uid == user.uid).is_some();
                        Ok(Some(CourseResponse {
                            course: c,
                            teacher: t,
                            students,
                            enrolled,
                        }))
                    } else {
                        Ok(Some(CourseResponse {
                            course: c,
                            teacher: t,
                            students,
                            enrolled: false,
                        }))
                    }
                }
                _ => Ok(None),
            }
        }
        _ => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use crate::common::course::{get_course, list_courses};
    use crate::common::{config_env_var, User, UserRole};
    use dotenv::dotenv;
    use firestore::FirestoreDb;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_get_course_async() {
        dotenv().unwrap();
        let project_id = config_env_var("PROJECT_ID").expect("failed to load project id");
        let db = FirestoreDb::new(&project_id)
            .await
            .expect("failed to load firestore db");

        let course = get_course(
            &db,
            &User {
                uid: Uuid::parse_str("21857bd9-f520-4842-b608-e07d858e9518")
                    .expect("failed to parse uuid"),
                devices: Vec::new(),
                email: "hl@hl.com".to_string(),
                name: "hl".to_string(),
                role: UserRole::Student,
            },
            "55f1e628-e5f1-4aa7-9fc3-e759b84daf45",
        )
        .await
        .expect("");
        println!("course {:#?}", course);
        assert!(course.is_some());
    }

    #[tokio::test]
    async fn test_list_course_async() {
        dotenv().unwrap();
        let project_id = config_env_var("PROJECT_ID").expect("failed to load project id");
        let db = FirestoreDb::new(&project_id)
            .await
            .expect("failed to load firestore db");

        let courses = list_courses(
            &db,
            &User {
                uid: Uuid::parse_str("21857bd9-f520-4842-b608-e07d858e9518")
                    .expect("failed to parse uuid"),
                devices: Vec::new(),
                email: "hl@hl.com".to_string(),
                name: "hl".to_string(),
                role: UserRole::Student,
            },
        )
        .await
        .expect("");
        println!("courses {:#?}", courses);
    }
}
