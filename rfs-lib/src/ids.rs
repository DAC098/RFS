pub const START_TIME: u64 = 168586200000;
pub const UID_SIZE: usize = 16;
pub const UID_ALPHABET: [char; 63] = [
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9',
    'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z',
    'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z',
    '_'
];

pub type UserId = i64;
pub type UserUid = String;
pub type GroupId = i64;
pub type GroupUid = String;
pub type RoleId = i64;
pub type RoleUid = String;
pub type FSId = i64;
pub type FSUid = String;
pub type StorageId = i64;
pub type StorageUid = String;

pub fn create_uid() -> String {
    nanoid::format(nanoid::rngs::default, &UID_ALPHABET, UID_SIZE)
}
