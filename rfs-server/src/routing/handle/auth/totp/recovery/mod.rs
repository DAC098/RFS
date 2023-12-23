use rfs_lib::actions;
use axum::http::StatusCode;
use axum::extract::State;
use axum::response::IntoResponse;
use futures::TryStreamExt;

use crate::net::{self, error};
use crate::state::ArcShared;
use crate::sec::authn::initiator::Initiator;
use crate::sec::authn::totp;

pub mod key_id;

pub async fn get(
    State(state): State<ArcShared>,
    initiator: Initiator,
) -> error::Result<impl IntoResponse> {
    let conn = state.pool().get().await?;

    let result = conn.query_raw(
        "\
        select auth_totp_hash.user_id, \
               auth_totp_hash.key, \
               auth_totp_hash.hash, \
               auth_totp_hash.used \
        from auth_totp_hash \
        where auth_totp_hash.user_id = $1",
        &[initiator.user().id()]
    ).await?;

    futures::pin_mut!(result);

    let mut rtn = Vec::with_capacity(3);

    while let Some(row) = result.try_next().await? {
        if rtn.len() == rtn.capacity() {
            rtn.reserve(3);
        }

        rtn.push(rfs_lib::schema::auth::TotpRecovery {
            user_id: row.get(0),
            key: row.get(1),
            hash: row.get(2),
            used: row.get(3),
        });
    }

    let wrapper = rfs_lib::json::ListWrapper::with_vec(rtn);

    Ok(net::Json::new(wrapper))
}

pub async fn post(
    State(state): State<ArcShared>,
    initiator: Initiator,
    axum::Json(json): axum::Json<actions::auth::CreateTotpHash>,
) -> error::Result<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    if !rfs_lib::sec::authn::totp::recovery::key_valid(&json.key) {
        return Err(error::Error::api((
            error::GeneralKind::InvalidData,
            error::Detail::with_key("key")
        )));
    };

    if totp::recovery::key_exists(&conn, initiator.user().id(), &json.key).await? {
        return Err(error::Error::api(error::GeneralKind::AlreadyExists));
    }

    let hash = totp::recovery::create_hash()?;

    let transaction = conn.transaction().await?;

    transaction.execute(
        "\
        insert into auth_totp_hash (user_id, key, hash, used) values \
        ($1, $2, $3, false)",
        &[initiator.user().id(), &json.key, &hash]
    ).await?;

    transaction.commit().await?;

    let rtn = rfs_lib::json::Wrapper::new(rfs_lib::schema::auth::TotpRecovery {
        user_id: initiator.user().id().clone(),
        key: json.key,
        hash,
        used: false
    });

    Ok(net::Json::new(rtn)
        .with_status(StatusCode::CREATED))
}

