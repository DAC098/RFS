pub mod ids;
pub mod serde;
pub mod json;
pub mod validation;
pub mod history;

pub mod schema;
pub mod actions;

pub mod user;
pub mod sec;
pub mod tags;
pub mod storage;
pub mod fs;

pub fn string_to_len_char(length: usize, ch: char) -> String {
    let mut rtn = String::with_capacity(length);

    for _ in 0..length {
        rtn.push(ch);
    }

    rtn
}

pub fn string_to_len(length: usize) -> String {
    string_to_len_char(length, 'a')
}
