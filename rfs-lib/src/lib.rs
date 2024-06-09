pub mod error;

pub mod ids;
pub mod serde;
pub mod validation;
pub mod history;

pub mod users;
pub mod sec;
pub mod tags;
pub mod fs;

pub mod query;

#[cfg(test)]
pub fn string_to_len_char(length: usize, ch: char) -> String {
    let mut rtn = String::with_capacity(length);

    for _ in 0..length {
        rtn.push(ch);
    }

    rtn
}

#[cfg(test)]
pub fn string_to_len(length: usize) -> String {
    string_to_len_char(length, 'a')
}
