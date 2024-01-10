use crate::error::{self, Context};
use crate::input;
use crate::state::AppState;

fn submit_user(
    state: &mut AppState
) -> error::Result<Option<rfs_api::auth::session::RequestedAuth>> {
    loop {
        let username = input::read_stdin_trimmed("username: ")?;
        let body = rfs_api::auth::session::RequestUser {
            username: username.clone()
        };

        let url = state.server.url.join("/auth/session/request")?;
        let res = state.client.post(url)
            .json(&body)
            .send()
            .context("failed to request session")?;

        state.save()?;

        match res.status() {
            reqwest::StatusCode::NO_CONTENT => {
                return Ok(None);
            },
            reqwest::StatusCode::OK => {
                let json: rfs_api::Payload<rfs_api::auth::session::RequestedAuth> = res.json()?;

                return Ok(Some(json.into_payload()));
            },
            reqwest::StatusCode::BAD_REQUEST => {
                let error: rfs_api::ApiError = res.json()?;

                if *error.kind() == rfs_api::ApiErrorKind::AlreadyAuthenticated {
                    return Ok(None);
                }

                return Err(error::Error::new().source(error));
            },
            reqwest::StatusCode::NOT_FOUND => {
                let _error: rfs_api::ApiError = res.json()?;

                println!("requested username was not found");
                continue;
            },
            _ => {
                let error: rfs_api::ApiError = res.json()?;

                return Err(error::Error::new().source(error));
            }
        }
    }
}

fn submit_auth(
    state: &mut AppState,
    auth_method: rfs_api::auth::session::RequestedAuth
) -> error::Result<Option<rfs_api::auth::session::RequestedVerify>> {
    match auth_method {
        rfs_api::auth::session::RequestedAuth::Password => {
            let prompt = "password: ";

            loop {
                let password = rpassword::prompt_password(&prompt)?;
                let auth_method = rfs_api::auth::session::SubmittedAuth::Password(password);

                let url = state.server.url.join("/auth/session/submit")?;
                let res = state.client.post(url)
                    .json(&auth_method)
                    .send()?;

                match res.status() {
                    reqwest::StatusCode::NO_CONTENT => {
                        return Ok(None);
                    },
                    reqwest::StatusCode::OK => {
                        return Ok(Some(res.json()?));
                    },
                    reqwest::StatusCode::BAD_REQUEST => {
                        let err: rfs_api::ApiError = res.json()?;

                        if *err.kind() == rfs_api::ApiErrorKind::AlreadyAuthenticated {
                            return Ok(None);
                        }

                        return Err(error::Error::new().source(err));
                    },
                    reqwest::StatusCode::FORBIDDEN => {
                        let err: rfs_api::ApiError = res.json()?;

                        if *err.kind() == rfs_api::ApiErrorKind::InvalidPassword {
                            println!("invalid password provided");
                            continue;
                        }

                        return Err(error::Error::new().source(err));
                    },
                    _ => {
                        let err: rfs_api::ApiError = res.json()?;

                        return Err(error::Error::new().source(err));
                    }
                }
            }
        }
    }
}

fn submit_verify(
    state: &mut AppState,
    verify_method: rfs_api::auth::session::RequestedVerify
) -> error::Result<()> {
    match verify_method {
        rfs_api::auth::session::RequestedVerify::Totp { digits } => {
            let prompt = format!("totp({}) code: ", digits);

            'input_loop: loop {
                let otp = input::read_stdin_trimmed(&prompt)?;

                if otp.len() != digits as usize {
                    println!("invalid totp code length");
                    continue;
                }

                for ch in otp.chars() {
                    if !ch.is_ascii_digit() {
                        println!("invalid totp characters provided");
                        continue 'input_loop;
                    }
                }

                let url = state.server.url.join("/auth/session/verify")?;
                let res = state.client.post(url)
                    .json(&verify_method)
                    .send()?;

                match res.status() {
                    reqwest::StatusCode::NO_CONTENT => {
                        return Ok(());
                    },
                    reqwest::StatusCode::BAD_REQUEST => {
                        let err: rfs_api::ApiError = res.json()?;

                        if *err.kind() == rfs_api::ApiErrorKind::AlreadyAuthenticated {
                            return Ok(());
                        }

                        return Err(error::Error::new().source(err));
                    },
                    reqwest::StatusCode::FORBIDDEN => {
                        let err: rfs_api::ApiError = res.json()?;

                        match err.kind() {
                            rfs_api::ApiErrorKind::InvalidTotp => {
                                println!("invalid totp provided");
                                continue 'input_loop;
                            },
                            rfs_api::ApiErrorKind::InvalidTotpHash => {
                                println!("invalid totp hash provided");
                                continue 'input_loop;
                            },
                            _ => {
                                return Err(error::Error::new().source(err));
                            }
                        }
                    },
                    _ => {
                        let err: rfs_api::ApiError = res.json()?;

                        return Err(error::Error::new().source(err));
                    }
                }
            }
        }
    }
}

pub fn connect(state: &mut AppState) -> error::Result<()> {
    let Some(auth_method) = submit_user(state)? else {
        return Ok(());
    };

    let Some(verify_method) = submit_auth(state, auth_method)? else {
        return Ok(());
    };

    submit_verify(state, verify_method)?;

    Ok(())
}

pub fn disconnect(state: &mut AppState) -> error::Result<()> {
    let url = state.server.url.join("/auth/session/drop")?;
    let res = state.client.delete(url)
        .send()?;

    let status = res.status();

    if status != reqwest::StatusCode::NO_CONTENT {
        let err: rfs_api::ApiError = res.json()?;

        return Err(error::Error::new().source(err));
    }

    Ok(())
}
