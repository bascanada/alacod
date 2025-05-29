use rand::distributions::Alphanumeric;
use rand::Rng;

pub fn generate_random_correlation_id_with_length(length: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}

pub fn generate_random_correlation_id() -> String {
    generate_random_correlation_id_with_length(6)
}
