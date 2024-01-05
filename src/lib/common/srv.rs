use firestore::FirestoreDb;
use reqwest::Client;

#[derive(Debug, Clone)]
pub struct AppState {
    pub db: FirestoreDb,
    pub http: Client,
}
