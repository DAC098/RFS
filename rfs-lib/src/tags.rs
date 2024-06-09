use crate::validation::check_control_leading_trailing;

pub const MAX_KEY_CHARS: usize = 128;
pub const MAX_VALUE_CHARS: usize = 512;

pub fn key_valid(given: &String) -> bool {
    !given.is_empty() && check_control_leading_trailing(given, Some(MAX_KEY_CHARS))
}

pub fn value_valid(given: &String) -> bool {
    !given.is_empty() && check_control_leading_trailing(given, Some(MAX_VALUE_CHARS))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn key_validation() {
        let valid = [
            String::from("i am key"),
            String::from("i am also key 😈"),
        ];

        for test in valid {
            assert!(key_valid(&test), "valid string failed {:?}", test);
        }

        let invalid = [
            String::new()
        ];

        for test in invalid {
            assert!(!key_valid(&test), "invalid string failed {:?}", test);
        }
    }

    #[test]
    fn value_validation() {
        let valid = [
            String::from("i am value"),
            String::from("i am also value 😈"),
        ];

        for test in valid {
            assert!(key_valid(&test), "valid string failed {:?}", test);
        }

        let invalid = [
            String::new()
        ];

        for test in invalid {
            assert!(!key_valid(&test), "invalid string failed {:?}", test);
        }
    }
}
