use std::time::{SystemTime, UNIX_EPOCH};


pub fn generate_timestamp_correlation_id() -> String {
    let now = SystemTime::now();

    let timestamp_nanos = now.duration_since(UNIX_EPOCH)
        .expect("Le temps est revenu en arrière !")
        .as_nanos();


    format!("{}", timestamp_nanos)
}