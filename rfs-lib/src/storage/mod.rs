use crate::validation::check_control_whitespace;

pub const MAX_NAME_CHARS: usize = 128;

pub fn name_valid(given: &String) -> bool {
    !given.is_empty() && check_control_whitespace(&given, Some(MAX_NAME_CHARS))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::string_to_len;

    #[test]
    fn name_validation() {
        let valid = [
            String::from("i_am_a_valid_name"),
            String::from("i_am_also_a_valid_name_😈"),
        ];

        for test in valid {
            assert!(name_valid(&test), "valid string failed {:?}", test);
        }

        let invalid = [
            String::new(),
            string_to_len(MAX_NAME_CHARS + 1)
        ];

        for test in invalid {
            assert!(!name_valid(&test), "invalid string failed {:?}", test);
        }
    }
}
