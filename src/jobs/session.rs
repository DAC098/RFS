use futures::stream::TryStreamExt;

use crate::state;
use crate::error::{self, Context};
use crate::sec::authn::session::token::SessionToken;
use crate::sec::secrets::Key;
use crate::time;

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
    let wrapper = state.sec().session_info().keys();
    let data = Key::rand_key_data()?;
    let created = time::utc_now()
        .context("timestamp error for session key")?;

    let key = Key::new(data, created);

    {
        let Ok(mut writer) = wrapper.inner().write() else {
            return Err(error::Error::new().source("session keys rwlock poisoned"));
        };

        writer.push(key);
    }

    wrapper.save().context("failed to save session secret")?;

    Ok(())
}
