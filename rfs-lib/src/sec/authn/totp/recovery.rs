use crate::validation::check_control_whitespace;

pub const MAX_KEY_CHARS: usize = 64;

pub fn key_valid(given: &String) -> bool {
    !given.is_empty() && check_control_whitespace(given, Some(MAX_KEY_CHARS))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn key_validation() {
        let valid = vec![
            String::from("i_am_a_key"),
            String::from("meh_for_emoji_😕"),
        ];

        for test in valid {
            assert!(key_valid(&test), "valid string failed {:?}", test);
        }

        let invalid = vec![
            String::new(),
            String::from(" key \u{0000} stuff "),
            crate::string_to_len(MAX_KEY_CHARS + 1),
        ];

        for test in invalid {
            assert!(!key_valid(&test), "invalid string failed {:?}", test);
        }
    }
}
