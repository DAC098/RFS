use axum::http::{StatusCode, HeaderMap};
use axum::extract::State;
use axum::response::IntoResponse;

use crate::net::error;
use crate::state::ArcShared;
use crate::sec::authn::session::expire_session_cookie;
use crate::sec::authn::initiator::{
    lookup_header_map,
    Mechanism,
    LookupError,
};

pub async fn delete(
    State(state): State<ArcShared>,
    headers: HeaderMap,
) -> error::Result<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    let session = match lookup_header_map(state.auth(), &conn, &headers).await {
        Ok(initiator) => match initiator.mechanism {
            Mechanism::Session(session) => session,
        }
        Err(err) => match err {
            LookupError::SessionNotFound => {
                return Ok((
                    StatusCode::NO_CONTENT,
                    expire_session_cookie(state.auth())
                ));
            }
            LookupError::SessionExpired(session) |
            LookupError::SessionUnauthenticated(session) |
            LookupError::SessionUnverified(session) => session,
            LookupError::UserNotFound(mechanism) => match mechanism {
                Mechanism::Session(session) => session
            }
            err => {
                return Err(error::Error::new().source(err));
            }
        }
    };

    let transaction = conn.transaction().await?;

    session.delete(&transaction).await?;

    transaction.commit().await?;

    state.auth()
        .session_info()
        .cache()
        .invalidate(&session.token);

    Ok((
        StatusCode::NO_CONTENT,
        expire_session_cookie(state.auth())
    ))
}
