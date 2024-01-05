use crate::common::user::try_get_users_from_emails;
use crate::common::{FCM_URL, MAX_FCM_TOKENS_PER_REQUEST};
use firestore::FirestoreDb;
use log::debug;
use serde::Serialize;
use serde_json::json;

pub async fn send_notification_to_emails<I: AsRef<str> + Serialize>(
    db: &FirestoreDb,
    http: &reqwest::Client,
    emails: &[I],
    title: Option<&str>,
    body: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let receivers = try_get_users_from_emails(db, emails).await?;
    let fcm_key = std::env::var("FCM_SEVER_KEY").expect("FCM_SEVER_KEY must be set!");

    debug!("receivers ok {:?}", receivers);

    let devices: Vec<_> = receivers
        .as_slice()
        .iter()
        .flat_map(|u| u.devices.clone())
        .collect();

    let token_batches: Vec<_> = devices.chunks(MAX_FCM_TOKENS_PER_REQUEST).collect();

    for tokens in token_batches {
        // Create a JSON payload for the FCM request with the current batch of tokens
        let payload = json!({
            "registration_ids": tokens,
            "notification": {
                "title": title,
                "body": body,
            },
        });

        // Send the FCM request
        let res = http
            .post(FCM_URL)
            .header("Authorization", format!("key={}", fcm_key))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await;

        match res {
            Ok(r) => {
                // Check if the request was successful (status code 200)
                if r.status().is_success() {
                    debug!("Notification sent successfully to batch!");
                } else {
                    debug!(
                        "failed to send notification to batch. Status code: {}",
                        r.status()
                    );
                }
            }
            _ => {
                debug!("failed to send notification");
            }
        }
    }

    Ok(())
}
