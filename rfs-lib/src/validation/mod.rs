pub fn check_control_leading_trailing<G>(
    given: G,
    max_chars: Option<usize>
) -> bool
where
    G: AsRef<str>
{
    let given_ref = given.as_ref();
    let mut iter = given_ref.chars();
    let mut char_count = 1;

    if let Some(ch) = iter.next() {
        char_count += 1;

        if ch.is_control() || ch.is_whitespace() {
            return false
        }
    }

    // check for trailing whitespace/control
    if let Some(ch) = iter.next_back() {
        char_count += 1;

        if ch.is_control() || ch.is_whitespace() {
            return false
        }
    }

    if let Some(max_chars) = max_chars {
        while let Some(ch) = iter.next() {
            if ch.is_control() {
                return false;
            }

            char_count += 1;

            if char_count > max_chars {
                return false;
            }
        }
    } else {
        while let Some(ch) = iter.next() {
            if ch.is_control() {
                return false;
            }
        }
    }

    true
}

pub fn check_control_whitespace<G>(
    given: G,
    max_chars: Option<usize>
) -> bool
where
    G: AsRef<str>
{
    let given_ref = given.as_ref();
    let mut iter = given_ref.chars();
    let mut char_count = 0;

    if let Some(max_chars) = max_chars {
        while let Some(ch) = iter.next() {
            if ch.is_control() || ch.is_whitespace() {
                return false;
            }

            char_count += 1;

            if char_count > max_chars {
                return false;
            }
        }
    } else {
        while let Some(ch) = iter.next() {
            if ch.is_control() || ch.is_whitespace() {
                return false;
            }
        }
    }

    true
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    pub fn check_control_leading_trailing_whitespace_chars() {
        let leading = String::from(" test");
        let trailing = String::from("test ");

        assert!(!check_control_leading_trailing(leading, None), "leading whitespace characters");
        assert!(!check_control_leading_trailing(trailing, None), "trailing whitespace characters");
    }

    #[test]
    pub fn check_control_leading_trailing_control_chars() {
        let trailing = String::from("test\u{0000}");
        let leading = String::from("\u{0000}test");
        let contains = String::from("test\u{0000}test");

        assert!(!check_control_leading_trailing(trailing, None), "trailing control characters");
        assert!(!check_control_leading_trailing(leading, None), "leading control characters");
        assert!(!check_control_leading_trailing(contains, None), "contains control characters");
    }

    #[test]
    pub fn check_control_leading_trailing_max_length() {
        let k = String::from("abcdefghijklmnopqrstuvwxyzA");
        let count = k.chars().count();
        let max = count - 1;

        assert!(!check_control_leading_trailing(k, Some(max)), "max {} total {}", max, count);
    }

    #[test]
    pub fn check_control_whitespace_whitespace_chars() {
        let leading = String::from(" test");
        let trailing = String::from("test ");
        let contains = String::from("test test");

        assert!(!check_control_whitespace(leading, None), "leading whitespace characters");
        assert!(!check_control_whitespace(trailing, None), "trailing whitespace characters");
        assert!(!check_control_whitespace(contains, None), "contains whitespace characters");
    }

    #[test]
    pub fn check_control_whitespace_control_chars() {
        let trailing = String::from("test\u{0000}");
        let leading = String::from("\u{0000}test");
        let contains = String::from("test\u{0000}test");

        assert!(!check_control_whitespace(trailing, None), "trailing control characters");
        assert!(!check_control_whitespace(leading, None), "leading control characters");
        assert!(!check_control_whitespace(contains, None), "contains control characters");
    }

    #[test]
    pub fn check_control_whitespace_max_length() {
        let k = String::from("abcdefghijklmnopqrstuvwxyzA");
        let count = k.chars().count();
        let max = count - 1;

        assert!(!check_control_whitespace(k, Some(max)), "max {} total {}", max, count);
    }
}
