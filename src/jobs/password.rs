use crate::state;
use crate::error;

pub async fn rotate(state: state::ArcShared) -> error::Result<()> {
    tracing::info!("rotating password keys");

    Ok(())
}
