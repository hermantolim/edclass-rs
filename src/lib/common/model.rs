use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserWithPasswordStudents {
    pub email: String,
    pub password: String,
    pub role: UserRole,
    pub name: String,
    pub students: Option<Vec<Uuid>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Course {
    pub id: Uuid,
    pub title: String,
    pub content: String,
    pub teacher_id: Uuid,
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
