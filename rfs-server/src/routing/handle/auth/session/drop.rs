use axum::http::{StatusCode, HeaderMap};
use axum::body::Body;
use axum::extract::State;
use axum::response::{Response, IntoResponse};

use crate::net::error;
use crate::state::ArcShared;
use crate::sec::authn::session::expire_session_cookie;
use crate::sec::authn::initiator::{
    lookup_header_map,
    Mechanism,
};

pub async fn delete(
    State(state): State<ArcShared>,
    headers: HeaderMap,
) -> error::Result<impl IntoResponse> {
    let mut conn = state.pool().get().await?;

    let initiator = lookup_header_map(state.auth(), &conn, &headers).await?;
    let transaction = conn.transaction().await?;

    match initiator.mechanism {
        Mechanism::Session(session) => {
            session.delete(&transaction).await?;

            transaction.commit().await?;

            Ok(Response::builder()
                .status(StatusCode::OK)
                .header("set-cookie", expire_session_cookie(state.auth()))
                .body(Body::empty())
                .unwrap())
        }
    }
}
