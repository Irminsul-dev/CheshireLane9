pub fn md5_with_salt(content: &str, salt: &str) -> String {
    format!("{:x}", md5::compute(format!("{content}{salt}")))
}
