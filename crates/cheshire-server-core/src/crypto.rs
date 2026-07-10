const SALT: &str = "dettimrepsignihtyrevednaeurtsignihton";

pub fn md5_with_salt(content: &str) -> String {
    format!("{:x}", md5::compute(format!("{content}{SALT}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn salted_md5_is_stable() {
        assert_eq!(md5_with_salt("test"), "0c9631a433289a2f93233c2e710ec77a");
    }
}
