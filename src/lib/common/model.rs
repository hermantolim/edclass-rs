use crate::common::{
    COURSES_COLLECTION, ENROLLMENTS_COLLECTION, STUDENTS_PARENTS_COLLECTION, USERS_COLLECTION,
};
use chrono::{DateTime, Utc};
use firestore::struct_path::path;
use firestore::{FirestoreDb, FirestoreResult};
use futures::stream::BoxStream;
use futures::{StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use uuid::Uuid;

#[derive(Debug, Copy, Clone, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum UserRole {
    Student,
    Teacher,
    Parent,
    Admin,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub uid: Uuid,
    pub email: String,
    pub role: UserRole,
    pub name: String,
    pub devices: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StudentsParents {
    pub student_id: Uuid,
    pub parent_id: Uuid,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserWithPassword {
    pub uid: Uuid,
    pub email: String,
    pub password: String,
    pub role: UserRole,
    pub name: String,
    pub devices: Vec<String>,
}

impl From<UserWithPassword> for User {
    fn from(u: UserWithPassword) -> Self {
        User {
            uid: u.uid,
            role: u.role,
            name: u.name,
            devices: u.devices,
            email: u.email,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserWithPasswordStudents {
    pub email: String,
    pub password: String,
    pub role: UserRole,
    pub name: String,
    pub students: Option<Vec<Uuid>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct Course {
    pub id: Uuid,
    pub title: String,
    pub content: String,
    pub teacher_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct CourseEnrollment {
    pub course: Course,
    pub enrolled: bool,
}

impl Hash for Course {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Enrollment {
    pub id: Uuid,
    pub course_id: Uuid,
    // user uuid -> role -> student
    pub student_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageState {
    Pending,
    Failed,
    Sent,
    Received,
    Read,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: Uuid,
    // user uuid
    pub sender_id: Uuid,
    // user uuid
    pub receiver_ids: Vec<String>,
    pub subject: Option<String>,
    pub content: String,
    pub state: MessageState,
    pub created_at: DateTime<Utc>,
}
/*
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsersDevices {
    pub id: String,
    pub user_id: Uuid,
}
*/

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Teacher {
    pub user: User,
    pub courses: HashMap<Course, Vec<User>>,
}
impl Teacher {
    pub fn as_user(&self) -> &User {
        &self.user
    }

    pub async fn from_user<U: Into<User>>(
        into_user: U,
        db: &FirestoreDb,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let user = into_user.into();
        if user.role != UserRole::Teacher {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "not a teacher",
            )));
        }

        let uuid_str = user.uid.to_string();
        let box_courses = db
            .fluent()
            .select()
            .from(COURSES_COLLECTION)
            .filter(|q| q.for_any([q.field(path!(Course::teacher_id)).eq(&uuid_str)]))
            .obj()
            .stream_query_with_errors()
            .await?;

        let to_courses: Vec<Course> = box_courses.try_collect().await?;
        let mut courses_map = HashMap::new();

        for course in to_courses {
            let course_id = course.id.to_string();
            let box_enroll: BoxStream<FirestoreResult<Enrollment>> = db
                .fluent()
                .select()
                .from(ENROLLMENTS_COLLECTION)
                .filter(|q| {
                    q.for_any([
                        //
                        q.field(path!(Enrollment::course_id)).eq(&course_id),
                    ])
                })
                .obj()
                .stream_query_with_errors()
                .await?;

            let course_enrollments: Vec<Enrollment> = box_enroll.try_collect().await?;
            let student_ids = course_enrollments
                .into_iter()
                .map(|ce| ce.student_id.to_string())
                .collect::<Vec<_>>();

            let mut box_users: BoxStream<(String, Option<User>)> = db
                .fluent()
                .select()
                .by_id_in(USERS_COLLECTION)
                .obj()
                .batch(&student_ids)
                .await?;

            let mut course_users = Vec::new();
            while let Some((_i, u)) = box_users.next().await {
                if let Some(su) = u {
                    course_users.push(su);
                }
            }

            //let course_users: Vec<User> = box_users.collect().await?;
            //println!("course_users {:?}", course_users);
            courses_map.insert(course, course_users);
        }

        Ok(Teacher {
            user,
            courses: courses_map,
        })
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parent {
    pub user: User,
    pub children: Vec<User>,
}

impl Parent {
    pub fn as_user(&self) -> &User {
        &self.user
    }

    pub async fn from_user<U: Into<User>>(
        into_user: U,
        db: &FirestoreDb,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let user = into_user.into();
        if user.role != UserRole::Parent {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "not a parent",
            )));
        }

        let box_children_uuid: BoxStream<FirestoreResult<StudentsParents>> = db
            .fluent()
            .select()
            .from(STUDENTS_PARENTS_COLLECTION)
            .filter(|q| {
                q.for_any([
                    //
                    q.field(path!(StudentsParents::parent_id)).eq(&user.uid),
                ])
            })
            .obj()
            .stream_query_with_errors()
            .await?;

        let students_parents: Vec<StudentsParents> = box_children_uuid.try_collect().await?;
        let children_uuid = students_parents
            .into_iter()
            .map(|sp| sp.student_id)
            .collect::<Vec<_>>();

        let box_users = db
            .fluent()
            .select()
            .from(USERS_COLLECTION)
            .filter(|q| {
                q.for_any([
                    // query by uid
                    q.field(path!(User::uid)).is_in(children_uuid.as_slice()),
                ])
            })
            .obj()
            .stream_query_with_errors()
            .await?;

        let children = box_users.try_collect().await?;

        Ok(Parent { user, children })
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CourseResponse {
    pub course: Course,
    pub teacher: User,
    pub students: Vec<User>,
    pub enrolled: bool,
}

#[cfg(test)]
mod tests {
    use crate::common::user::try_find_user;
    use crate::common::{config_env_var, Parent, Teacher};
    use dotenv::dotenv;
    use firestore::FirestoreDb;

    #[tokio::test]
    async fn test_parents_async() {
        dotenv().unwrap();
        let project_id = config_env_var("PROJECT_ID").expect("failed to load project id");
        let db = FirestoreDb::new(&project_id)
            .await
            .expect("failed to load firestore db");

        let user = try_find_user(&db, "mom@mom.com").await.unwrap().unwrap();
        let user_id = user.uid.clone();
        let parent = Parent::from_user(user, &db).await.unwrap();
        let parent_id = parent.user.uid.clone();
        println!("parent {:#?}", parent);
        assert_eq!(user_id, parent_id);
    }

    #[tokio::test]
    async fn test_teachers_async() {
        dotenv().unwrap();
        let project_id = config_env_var("PROJECT_ID").expect("failed to load project id");
        let db = FirestoreDb::new(&project_id)
            .await
            .expect("failed to load firestore db");

        let user = try_find_user(&db, "t1@t1.com").await.unwrap().unwrap();
        let user_id = user.uid.clone();
        let teacher = Teacher::from_user(user, &db).await.unwrap();
        let teacher_id = teacher.user.uid.clone();
        println!("teacher {:#?}", teacher);
        assert_eq!(user_id, teacher_id);
    }
}
