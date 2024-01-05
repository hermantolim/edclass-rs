use crate::common::{Course, COURSES_COLLECTION};
use firestore::FirestoreDb;
use futures::TryStreamExt;

pub async fn list_courses(db: &FirestoreDb) -> Result<Vec<Course>, Box<dyn std::error::Error>> {
    let box_courses = db
        .fluent()
        .list()
        .from(COURSES_COLLECTION)
        .obj()
        .stream_all_with_errors()
        .await?;

    let courses = box_courses.try_collect().await?;
    Ok(courses)
}

pub async fn get_course(
    db: &FirestoreDb,
    id: &str,
) -> Result<Option<Course>, Box<dyn std::error::Error>> {
    let course = db
        .fluent()
        .select()
        .by_id_in(COURSES_COLLECTION)
        .obj()
        .one(id)
        .await?;

    Ok(course)
}
