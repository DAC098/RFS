use crate::state;
use crate::error;

pub async fn cleanup(state: state::ArcShared) -> error::Result<()> {
    tracing::info!("cleaning up old sessions");

    Ok(())
}

pub async fn rotate(state: state::ArcShared) -> error::Result<()> {
    tracing::info!("rotating session keys");

    Ok(())
}
