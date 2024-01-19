use crate::validation::check_control_whitespace;

pub const MAX_GROUP_NAME_CHARS: usize = 128;

pub fn name_valid(given: &str) -> bool {
    !given.is_empty() && check_control_whitespace(given, Some(MAX_GROUP_NAME_CHARS))
}
