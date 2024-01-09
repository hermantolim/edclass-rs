use crate::common::enrollment::list_user_enrolled_in;
use crate::common::user::get_user_by_id;
use crate::common::{
    Course, CourseEnrollment, CourseResponse, Enrollment, EnrollmentCounter, MyCourse, User,
    UserRole, COURSES_COLLECTION, ENROLLMENTS_COLLECTION, USERS_COLLECTION,
};
use firestore::struct_path::path;
use firestore::{FirestoreDb, FirestoreResult};
use futures::stream::BoxStream;
use futures::StreamExt;
use uuid::Uuid;

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

pub async fn list_my_courses(
    db: &FirestoreDb,
    user: &User,
) -> Result<Vec<MyCourse>, Box<dyn std::error::Error>> {
    let mut box_courses: BoxStream<FirestoreResult<Course>> = db
        .fluent()
        .select()
        .from(COURSES_COLLECTION)
        .filter(|q| {
            // filter course
            q.for_all([q.field(path!(Course::teacher_id)).eq(&user.uid)])
        })
        .obj()
        .stream_query_with_errors()
        .await?;

    let mut courses: Vec<MyCourse> = Vec::new();
    while let Some(Ok(c)) = box_courses.next().await {
        let student_counts: Vec<EnrollmentCounter> = db
            .fluent()
            .select()
            .from(ENROLLMENTS_COLLECTION)
            .filter(|q| {
                // filter field
                q.for_all([
                    // filter all
                    q.field(path!(Enrollment::course_id)).eq(&c.id),
                ])
            })
            .aggregate(|a| {
                // aggregate count
                a.fields([a.field(path!(EnrollmentCounter::students)).count()])
            })
            .obj()
            .query()
            .await?;

        let count = student_counts.iter().next();
        courses.push(MyCourse {
            course: c,
            students: match count {
                Some(num) => num.students,
                _ => 0,
            },
        })
    }

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

pub async fn get_teacher(
    db: &FirestoreDb,
    course_id: &Uuid,
) -> Result<User, Box<dyn std::error::Error>> {
    let course_str = course_id.to_string();
    let course: Option<Course> = db
        .fluent()
        .select()
        .by_id_in(COURSES_COLLECTION)
        .obj()
        .one(&course_str)
        .await?;

    match course {
        Some(c) => {
            let teacher_id_str = c.teacher_id.to_string();
            let teacher = db
                .fluent()
                .select()
                .by_id_in(USERS_COLLECTION)
                .obj()
                .one(&teacher_id_str)
                .await?;

            match teacher {
                Some(t) => Ok(t),
                _ => Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "teacher not found",
                ))),
            }
        }
        _ => Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "teacher not found",
        ))),
    }
}

#[cfg(test)]
mod tests {
    use crate::common::course::{get_course, get_teacher, list_courses, list_my_courses};
    use crate::common::{config_env_var, User, UserRole};
    use dotenv::dotenv;
    use firestore::FirestoreDb;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_get_teacher_async() {
        dotenv().unwrap();
        let project_id = config_env_var("PROJECT_ID").expect("failed to load project id");
        let db = FirestoreDb::new(&project_id)
            .await
            .expect("failed to load firestore db");

        let teacher = get_teacher(
            &db,
            &Uuid::parse_str("893a39bd-17d3-4a59-9b08-f2a1572fdd88").expect("failed to parse uuid"),
        )
        .await;
        println!("teacher {:#?}", teacher);
        assert!(teacher.is_ok());
    }
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
            "893a39bd-17d3-4a59-9b08-f2a1572fdd88",
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

    #[tokio::test]
    async fn test_list_my_courses_async() {
        dotenv().unwrap();
        let project_id = config_env_var("PROJECT_ID").expect("failed to load project id");
        let db = FirestoreDb::new(&project_id)
            .await
            .expect("failed to load firestore db");

        let result = list_my_courses(
            &db,
            &User {
                uid: Uuid::parse_str("30e0ebf1-da41-4686-b38d-76ca74ecd523")
                    .expect("failed to parse uuid"),
                devices: Vec::new(),
                email: "t1@t1.com".to_string(),
                name: "t1".to_string(),
                role: UserRole::Teacher,
            },
        )
        .await;

        println!("my courses {:#?}", result)
    }
}
