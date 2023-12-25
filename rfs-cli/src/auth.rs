use crate::error;
use crate::input;
use crate::state::AppState;

fn submit_user(
    state: &mut AppState
) -> error::Result<Option<rfs_lib::schema::auth::AuthMethod>> {
    loop {
        let username = input::read_stdin_trimmed("username: ")?;
        let body = rfs_lib::actions::auth::RequestUser {
            username: username.clone()
        };

        let url = state.server.url.join("/auth/session/request")?;
        let res = state.client.post(url)
            .json(&body)
            .send()?;

        let status = res.status();

        if status != reqwest::StatusCode::OK {
            let json = res.json::<rfs_lib::json::Error>()?;

            if json.kind() == "UserNotFound" {
                println!("requested username was not found");
                continue;
            }

            if json.kind() == "AlreadyAuthenticated" {
                return Ok(None);
            }

            return Err(error::Error::new()
                .kind("FailedUserRequest")
                .message("failed to submit requested username")
                .source(format!("{:?}", json)));
        }

        state.save()?;

        let json = res.json::<rfs_lib::json::Wrapper<Option<rfs_lib::schema::auth::AuthMethod>>>()?;

        return Ok(json.into_payload());
    }
}

fn submit_auth(
    state: &mut AppState,
    auth_method: rfs_lib::schema::auth::AuthMethod
) -> error::Result<Option<rfs_lib::schema::auth::VerifyMethod>> {
    match auth_method {
        rfs_lib::schema::auth::AuthMethod::None => Ok(None),
        rfs_lib::schema::auth::AuthMethod::Password => {
            let prompt = "password: ";

            loop {
                let password = rpassword::prompt_password(&prompt)?;
                let auth_method = rfs_lib::actions::auth::SubmitAuth::Password(password);

                let url = state.server.url.join("/auth/session/submit")?;
                let res = state.client.post(url)
                    .json(&auth_method)
                    .send()?;

                let status = res.status();

                if status != reqwest::StatusCode::OK {
                    let json = res.json::<rfs_lib::json::Error>()?;

                    if json.kind() == "InvalidPassword" {
                        println!("invalid password provided");
                        continue;
                    }

                    if json.kind() == "AlreadyAuthenticated" {
                        return Ok(None);
                    }

                    return Err(error::Error::new()
                        .kind("FailedAuthentication")
                        .message("failed to submit requested auth method")
                        .source(format!("{:?}", json)));
                }

                let json = res.json::<rfs_lib::json::Wrapper<Option<rfs_lib::schema::auth::VerifyMethod>>>()?;

                return Ok(json.into_payload());
            }
        }
    }
}

fn submit_verify(
    state: &mut AppState,
    verify_method: rfs_lib::schema::auth::VerifyMethod
) -> error::Result<()> {
    match verify_method {
        rfs_lib::schema::auth::VerifyMethod::None => {},
        rfs_lib::schema::auth::VerifyMethod::Totp{ digits } => {
            let prompt = format!("totp({}) code: ", digits);

            'input_loop: loop {
                let otp = input::read_stdin_trimmed(&prompt)?;

                if otp.len() != digits as usize {
                    println!("invalid totp code length");
                    continue;
                }

                for ch in otp.chars() {
                    if !ch.is_ascii_digit() {
                        println!("invalid totp characters providied");
                        continue 'input_loop;
                    }
                }

                let url = state.server.url.join("/auth/session/verify")?;
                let res = state.client.post(url)
                    .json(&verify_method)
                    .send()?;

                let status = res.status();

                if status != reqwest::StatusCode::OK {
                    let json = res.json::<rfs_lib::json::Error>()?;

                    if json.kind() == "AlreadyAuthenticated" {
                        break;
                    }

                    return Err(error::Error::new()
                        .kind("FailedVerification")
                        .message("failed to submit requested verification method")
                        .source(format!("{:?}", json)));
                }

                break;
            }
        }
    }

    Ok(())
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

    if status != reqwest::StatusCode::OK {
        let json = res.json::<rfs_lib::json::Error>()?;

        return Err(error::Error::new()
            .kind("FailedSessionDrop")
            .message("failed to drop the current client session")
            .source(json));
    }

    Ok(())
}
