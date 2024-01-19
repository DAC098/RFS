pub mod totp;

pub const MIN_PASSWORD_CHARS: usize = 8;
pub const MAX_PASSWORD_CHARS: usize = 512;

pub fn password_valid(given: &String) -> bool {
    let iter = given.chars();
    let mut char_count = 0;

    for ch in iter {
        if ch.is_control() {
            return false;
        }

        char_count += 1;

        if char_count > MAX_PASSWORD_CHARS {
            return false;
        }
    }

    if char_count < MIN_PASSWORD_CHARS {
        return false;
    }

    true
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn password_validation() {
        let valid = vec![
            String::from("-h6Ď♂♱ƲÐȷ♋🙋ȆċŶ😣ƁŨ😌☑æȘŤŎ😕♁🙍"),
            String::from("Sharper Snowboard Equinox Faucet Monoxide0"),
        ];

        for test in valid {
            assert!(password_valid(&test), "valid string failed {:?}", test);
        }

        let invalid = vec![
            String::from("   test  \u{0000} other stuff"),
            crate::string_to_len(MIN_PASSWORD_CHARS - 1),
            crate::string_to_len(MAX_PASSWORD_CHARS + 1),
        ];

        for test in invalid {
            assert!(!password_valid(&test), "invalid string failed {:?}", test);
        }
    }
}
