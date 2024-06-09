pub const MAX_NAME_CHARS: usize = 128;

pub fn name_valid(given: &str) -> bool {
    crate::fs::valid_pathname(given, 1, MAX_NAME_CHARS, false)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::string_to_len;

    #[test]
    fn name_validation() {
        let valid = [
            "i_am_a_valid_name",
            "i_am_also_a_valid_name_😈",
        ];

        for test in valid {
            assert!(name_valid(&test), "valid string failed {:?}", test);
        }

        let max_len = string_to_len(MAX_NAME_CHARS + 1);

        let invalid = [
            "",
            max_len.as_str(),
            "i have spaces",
        ];

        for test in invalid {
            assert!(!name_valid(&test), "invalid string failed {:?}", test);
        }
    }
}
