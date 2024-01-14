pub use rfs_lib::sec::authn::totp::Algo;

pub fn rust_otp_algo(algo: &Algo) -> rust_otp::Algo {
    match algo {
        Algo::SHA1 => rust_otp::Algo::SHA1,
        Algo::SHA256 => rust_otp::Algo::SHA256,
        Algo::SHA512 => rust_otp::Algo::SHA512,
    }
}
