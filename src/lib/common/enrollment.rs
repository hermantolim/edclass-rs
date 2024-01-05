use crate::common::{Enrollment, ENROLLMENTS_COLLECTION};
use firestore::FirestoreDb;

pub async fn enroll(
    db: &FirestoreDb,
    enrollment: &Enrollment,
) -> Result<(), Box<dyn std::error::Error>> {
    db.fluent()
        .insert()
        .into(ENROLLMENTS_COLLECTION)
        .document_id(enrollment.id.to_string())
        .object(enrollment)
        .execute()
        .await?;
    Ok(())
}
