use futures::stream::TryStreamExt;

use crate::state;
use crate::error;
use crate::sec::authn::session::token::SessionToken;

pub async fn cleanup(state: state::ArcShared) -> error::Result<()> {
    let today = chrono::Utc::now();
    let cache = state.auth().session_info().cache();
    let mut conn = state.pool().get().await?;

    let transaction = conn.transaction().await?;

    let found_session_tokens = transaction.query_raw(
        "\
        delete from auth_session \
        where expires <= $1 \
        returning token",
        [&today]
    ).await?;

    futures::pin_mut!(found_session_tokens);

    let mut count = 0;

    while let Some(record) = found_session_tokens.try_next().await? {
        let token = SessionToken::from_vec(record.get(0));

        cache.remove(&token);

        count += 1;
    }

    tracing::info!("dropped {count} sessions");

    transaction.commit().await?;

    Ok(())
}

pub async fn rotate(state: state::ArcShared) -> error::Result<()> {
    tracing::info!("rotating session keys");

    Ok(())
}
