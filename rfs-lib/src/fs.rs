use crate::validation::check_control_leading_trailing;

pub mod storage;

pub const MAX_BASENAME_CHARS: usize = 512;
pub const MIN_BASENAME_CHARS: usize = 1;

pub const MAX_COMMENT_CHARS: usize = 1024;

fn valid_pathname_char(ch: &char) -> bool {
    (match ch {
        '/' | '\\' => false,
        _ => true
    }) && !ch.is_control()
}

fn valid_pathname(given: &str, min_chars: usize, max_chars: usize, allow_whitespace: bool) -> bool {
    let mut count = 0;
    let mut iter = given.chars();

    if let Some(ch) = iter.next() {
        if ch.is_whitespace() || !valid_pathname_char(&ch) {
            return false;
        }

        count += 1;
    }

    if let Some(ch) = iter.next_back() {
        if ch.is_whitespace() || !valid_pathname_char(&ch) {
            return false;
        }

        count += 1;
    }

    for ch in iter {
        if (!allow_whitespace && ch.is_whitespace()) || !valid_pathname_char(&ch) {
            return false;
        }

        count += 1;

        if count > max_chars {
            return false;
        }
    }

    if count < min_chars {
        false
    } else {
        true
    }
}

pub fn basename_valid(given: &str) -> bool {
    valid_pathname(given, MIN_BASENAME_CHARS, MAX_BASENAME_CHARS, true)
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
            "file_name.txt",
            "a",
        ];

        for test in valid {
            assert!(basename_valid(&test), "valid string failed {:?}", test);
        }

        let max_len = crate::string_to_len(MAX_BASENAME_CHARS + 1);

        let invalid = [
            "",
            "/leading_slash",
            "trailing_slash/",
            "middle/slash",
            "\\leading_back_slash",
            "trailing_back_slash\\",
            "middle\\back_slask",
            max_len.as_str()
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

