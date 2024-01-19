use email_address::EmailAddress;

use crate::validation::check_control_whitespace;

pub mod groups;

pub const MAX_USERNAME_CHARS: usize = 128;

pub fn username_valid(given: &String) -> bool {
    !given.is_empty() && check_control_whitespace(given, Some(MAX_USERNAME_CHARS))
}

pub fn email_valid(given: &String) -> bool {
    EmailAddress::is_valid(given)
}
