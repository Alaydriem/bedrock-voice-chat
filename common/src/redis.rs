use blake3;

pub const ACCESS_TOKEN_KEY_SUFFIX: &str = "NCRYPTF_ACCESS_TOKEN_SUFFIX";

pub const REFRESH_TOKEN_KEY_SUFFIX: &str = "NCRYPTF_REFRESH_TOKEN_SUFFIX";

pub fn create_redis_key(key: &str, suffix: &str) -> String {
    let key = blake3::hash(format!("{}+{}", key, suffix).as_bytes());
    return key.to_hex().to_string();
}
