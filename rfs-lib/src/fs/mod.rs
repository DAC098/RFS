use crate::validation::check_control_leading_trailing;

pub const MAX_BASENAME_CHARS: usize = 512;
pub const MAX_COMMENT_CHARS: usize = 1024;

pub fn basename_valid(given: &String) -> bool {
    !given.is_empty() && check_control_leading_trailing(given, Some(MAX_BASENAME_CHARS))
}

pub fn comment_valid(given: &String) -> bool {
    !given.is_empty() && check_control_leading_trailing(given, Some(MAX_COMMENT_CHARS))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn basename_validation() {
        let valid = [
            String::from("file_name.txt"),
            String::from("a"),
        ];

        for test in valid {
            assert!(basename_valid(&test), "valid string failed {:?}", test);
        }

        let invalid = [
            String::new(),
            crate::string_to_len(MAX_BASENAME_CHARS + 1),
        ];

        for test in invalid {
            assert!(!basename_valid(&test), "invalid string failed {:?}", test);
        }
    }

    #[test]
    fn comment_validation() {
        let valid = [
            String::from("I am a comment that will describe what this thing is in greater detail"),
            String::from("I am also a comment but with other utf-8 characters 😌😲Ì😣ÿưõǿuYŖ¤1")
        ];

        for test in valid {
            assert!(comment_valid(&test), "valid string failed {:?}", test);
        }

        let invalid = [
            String::new(),
            crate::string_to_len(MAX_COMMENT_CHARS + 1),
        ];

        for test in invalid {
            assert!(!comment_valid(&test), "invalid string failed {:?}", test);
        }
    }
}

