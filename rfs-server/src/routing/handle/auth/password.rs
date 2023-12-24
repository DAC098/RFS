use rfs_lib::actions;

use axum::extract::State;
use axum::response::IntoResponse;

use crate::net::{self, error};
use crate::state::ArcShared;
use crate::sec::authn::initiator::Initiator;
use crate::sec::authn::password::{self, Password};

pub async fn post(
    State(state): State<ArcShared>,
    initiator: Initiator,
    axum::Json(json): axum::Json<actions::auth::CreatePassword>,
) -> error::Result<impl IntoResponse> {
    let mut conn = state.pool().get().await?;
    let peppers = state.sec().peppers().inner();

    if !rfs_lib::sec::authn::password_valid(&json.updated) {
        return Err(error::Error::api((
            error::GeneralKind::ValidationFailed,
            error::Detail::Keys(vec![String::from("password")]),
        )));
    };

    if json.updated != json.confirm {
        return Err(error::Error::api((
            error::GeneralKind::InvalidData,
            error::Detail::Keys(vec![String::from("confirm")])
        )));
    }

    if let Some(current) = Password::retrieve(&conn, initiator.user().id()).await? {
        let Some(given) = json.current else {
            return Err(error::Error::api((
                error::GeneralKind::MissingData,
                error::Detail::Keys(vec![String::from("current")])
            )));
        };

        if !rfs_lib::sec::authn::password_valid(&given) {
            return Err(error::Error::api((
                error::GeneralKind::ValidationFailed,
                error::Detail::Keys(vec![String::from("password")])
            )));
        };

        let salt = password::gen_salt()?;
        let hash;
        let version;

        {
            let reader = peppers.read()
                .map_err(|_| error::Error::new().source("secrets rwlock poisoned"))?;

            let secret = if current.version == 0 {
                &[]
            } else {
                let Some(secret) = reader.get(&current.version) else {
                    return Err(error::Error::new()
                        .source("password secret version not found. unable to verify user password"));
                };

                secret.data().as_slice()
            };

            if !current.verify(given, secret)? {
                return Err(error::Error::api((
                    error::GeneralKind::InvalidData,
                    error::Detail::Keys(vec![String::from("password")])
                )));
            }

            if let Some((ver, pepper)) = reader.latest_version() {
                hash = password::gen_hash(&json.updated, &salt, pepper.data())?;
                version = *ver;
            } else {
                hash = password::gen_hash(&json.updated, &salt, &[])?;
                version = 0;
            }
        }

        let transaction = conn.transaction().await?;

        let _ = transaction.execute(
            "update auth_password set hash = $2, version = $3 where user_id = $1",
            &[initiator.user().id(), &hash, &(version as i64)]
        );

        transaction.commit().await?;
    } else {
        let salt = password::gen_salt()?;
        let version;
        let hash;

        {
            let reader = peppers.read()
                .map_err(|_| error::Error::new().source("peppers rwlock poisoned"))?;

            if let Some((ver, pepper)) = reader.latest_version() {
                hash = password::gen_hash(&json.updated, &salt, pepper.data())?;
                version = *ver;
            } else {
                hash = password::gen_hash(&json.updated, &salt, &[])?;
                version = 0;
            }
        }

        let transaction = conn.transaction().await?;

        let _ = transaction.execute(
            "\
            insert into auth_password (user_id, version, hash) values
            ($1, $2, $3)",
            &[&initiator.user().id(), &(version as i64), &hash]
        ).await?;

        transaction.commit().await?;
    }

    Ok(net::Json::empty()
        .with_message("password updated successfully"))
}

pub async fn delete(
    State(state): State<ArcShared>,
    initiator: Initiator,
    axum::Json(json): axum::Json<actions::auth::DeletePassword>,
) -> error::Result<impl IntoResponse> {
    let mut conn = state.pool().get().await?;
    let peppers = state.sec().peppers().inner();

    if let Some(current) = Password::retrieve(
        &conn,
        initiator.user().id()
    ).await? {
        let Some(given) = json.current else {
            return Err(error::Error::api((
                error::GeneralKind::MissingData,
                error::Detail::Keys(vec![String::from("current")]),
            )));
        };

        {
            let reader = peppers.read()
                .map_err(|_| error::Error::new().source("peppers rwlock poisoned"))?;

            let secret = if current.version == 0 {
                &[]
            } else {
                let Some(secret) = reader.get(&current.version) else {
                    return Err(error::Error::new()
                        .source("password secret version not found. unable to verify user password"));
                };

                secret.data().as_slice()
            };

            if !current.verify(given, secret)? {
                return Err(error::Error::api((
                    error::GeneralKind::InvalidData,
                    error::Detail::Keys(vec![String::from("current")])
                )));
            }
        }

        let transaction = conn.transaction().await?;

        let _ = transaction.execute(
            "delete from auth_password where user_id = $1",
            &[initiator.user().id()]
        ).await?;

        transaction.commit().await?;

        Ok(net::Json::empty()
           .with_message("password deleted successfuly"))
    } else {
        Ok(net::Json::empty()
           .with_message("no password found"))
    }
}
