use std::path::PathBuf;

use futures::{TryStream, TryStreamExt};

use crate::fs::error::StreamError;
use crate::storage;
use crate::storage::fs::{Storage as FsStorage};

pub async fn new_stream_file<S>(
    prefix: PathBuf,
    storage: &storage::Medium,
    mut stream: S,
) -> Result<(u64, FsStorage), StreamError>
where
    S: TryStream + Unpin,
    S::Ok: AsRef<[u8]>,
    StreamError: From<<S as TryStream>::Error>
{
    match &storage.type_ {
        storage::Type::Local(local) => {
            let full = local.path.join(prefix);
            let mut size: u64 = 0;

            let mut file = tokio::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&full)
                .await?;

            while let Some(slice) = stream.try_next().await? {
                let wrote = tokio::io::AsyncWriteExt::write(
                    &mut file,
                    slice.as_ref()
                ).await?;

                let converted = TryFrom::try_from(wrote)
                    .expect("total bytes written exceeds u64?");

                if let Some(checked) = size.checked_add(converted) {
                    size = checked;
                } else {
                    return Err(StreamError::MaxFileSize);
                }
            }

            Ok((size, FsStorage::Local(storage::fs::Local {
                id: storage.id.clone()
            })))
        }
    }
}

