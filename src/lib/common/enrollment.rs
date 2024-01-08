use crate::common::user::get_user_by_id;
use crate::common::{Enrollment, User, ENROLLMENTS_COLLECTION};
use firestore::{path, FirestoreDb};
use uuid::Uuid;

pub async fn enroll(
    db: &FirestoreDb,
    enrollment: &Enrollment,
) -> Result<(), Box<dyn std::error::Error>> {
    let existing_enrollment: Vec<Enrollment> = db
        .fluent()
        .select()
        .from(ENROLLMENTS_COLLECTION)
        .filter(|q| {
            // filter
            q.for_all([
                // all match condition
                q.field(path!(Enrollment::student_id))
                    .eq(&enrollment.student_id.to_string()),
                q.field(path!(Enrollment::course_id))
                    .eq(&enrollment.course_id.to_string()),
            ])
        })
        .obj()
        .query()
        .await?;

    if existing_enrollment.is_empty() {
        db.fluent()
            .insert()
            .into(ENROLLMENTS_COLLECTION)
            .document_id(enrollment.id.to_string())
            .object(enrollment)
            .execute()
            .await?;
    }

    Ok(())
}

pub async fn list_user_enrolled_in(
    db: &FirestoreDb,
    course_id: &Uuid,
) -> Result<Vec<User>, Box<dyn ::std::error::Error + Send + Sync>> {
    let enrollments: Vec<Enrollment> = db
        .fluent()
        .select()
        .from(ENROLLMENTS_COLLECTION)
        .filter(|q| {
            q.for_any([q
                .field(path!(Enrollment::course_id))
                .array_contains(course_id)])
        })
        .obj()
        .query()
        .await?;

    let mut students = Vec::new();

    for e in enrollments {
        if let Some(u) = get_user_by_id(db, &e.student_id).await? {
            students.push(u);
        };
    }

    Ok(students)
}
