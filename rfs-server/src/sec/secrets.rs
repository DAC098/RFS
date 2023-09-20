use rust_kms_local::fs::encrypted;
use rust_kms_local::local::VersionedKey as KmsVersionedKey;

pub use encrypted::Options;

pub type Key = rust_kms_local::Key<[u8; 32]>;
pub type VersionedKey = KmsVersionedKey<Key>;
pub type Manager = encrypted::Encrypted<Key>;

pub fn empty_key() -> Key {
    let mut builder = Key::builder([0; 32]);
    builder.set_created(0);

    builder.build().unwrap()
}

pub fn empty_versioned_key() -> VersionedKey {
    KmsVersionedKey(0, empty_key())
}
