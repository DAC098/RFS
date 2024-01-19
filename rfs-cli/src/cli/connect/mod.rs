use rfs_api::ApiErrorKind;
use rfs_api::client::ApiClient;
use rfs_api::client::auth::session::{
    RequestAuth,
    SubmitAuth,
    SubmitVerify,
};

use crate::input;
use crate::error::{self, Context};

pub fn submit_user(
    client: &mut ApiClient
) -> error::Result<Option<rfs_api::auth::session::RequestedAuth>> {
    loop {
        let username = input::read_stdin_trimmed("username: ")?;

        match RequestAuth::new(username.clone()).send(&client) {
            Ok(result) => {
                client.save_session().context("failed saving session data")?;

                return Ok(result.map(|v| v.into_payload()))
            },
            Err(err) => {
                let api = err.as_api()
                    .context("failed handling server request")?;

                match api.kind() {
                    ApiErrorKind::AlreadyAuthenticated => {
                        return Ok(None)
                    },
                    ApiErrorKind::UserNotFound => {
                        println!("requested username was not found");
                        continue;
                    }
                    _ => {
                        return Err(error::Error::from(api));
                    }
                }
            }
        }
    }
}

fn submit_password(client: &ApiClient) -> error::Result<Option<rfs_api::auth::session::RequestedVerify>> {
    let prompt = "password: ";

    loop {
        let password = rpassword::prompt_password(&prompt)?;

        let result = SubmitAuth::password(password).send(client);

        match result {
            Ok(rtn) => {
                return Ok(rtn.map(|v| v.into_payload()))
            }
            Err(err) => {
                let api = err.as_api().context("error server request")?;

                match api.kind() {
                    ApiErrorKind::AlreadyAuthenticated => {
                        return Ok(None);
                    },
                    ApiErrorKind::InvalidPassword => {
                        println!("invalid password provided");
                        continue;
                    },
                    _ => {
                        return Err(error::Error::from(api));
                    }
                }
            }
        }
    }
}

pub fn submit_auth(
    client: &ApiClient,
    auth_method: rfs_api::auth::session::RequestedAuth
) -> error::Result<Option<rfs_api::auth::session::RequestedVerify>> {
    match auth_method {
        rfs_api::auth::session::RequestedAuth::Password => submit_password(client)
    }
}

fn submit_totp(client: &ApiClient, digits: u32) -> error::Result {
    let prompt = format!("totp 2FA\nnote: prefix with # for recovery code\n{} digit code: ", digits);

    loop {
        let otp = input::read_stdin_trimmed(&prompt)?;

        if let Some((_, code)) = otp.split_once('#') {
            let Err(err) = SubmitVerify::totp_hash(code).send(client) else {
                return Ok(());
            };

            let api = err.as_api().context("error server request")?;

            match api.kind() {
                ApiErrorKind::InvalidTotpHash => {
                    println!("invalid totp hash provided");
                    continue;
                }
                _ => {
                    return Err(error::Error::from(api))
                }
            }
        } else {
            let Err(err) = SubmitVerify::totp(otp).send(client) else {
                return Ok(());
            };

            let api = err.as_api().context("error server request")?;

            match api.kind() {
                ApiErrorKind::InvalidTotp => {
                    println!("invalid totp provided");
                    continue;
                }
                _ => {
                    return Err(error::Error::from(api));
                }
            }
        }
    }
}

pub fn submit_verify(
    client: &ApiClient,
    verify_method: rfs_api::auth::session::RequestedVerify
) -> error::Result {
    match verify_method {
        rfs_api::auth::session::RequestedVerify::Totp { digits } => {
            submit_totp(client, digits)
        }
    }
}
