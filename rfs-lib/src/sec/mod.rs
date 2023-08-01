pub mod authn {
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

    pub mod totp {
        pub fn digits_valid(given: &u32) -> bool {
            *given <= 12
        }

        pub fn step_valid(given: &u64) -> bool {
            *given <= 120
        }

        pub mod recovery {
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
        }
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
}
