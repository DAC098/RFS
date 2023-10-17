use rfs_lib::schema;
use rfs_lib::actions;
use axum::http::{HeaderMap, StatusCode};
use axum::extract::State;
use axum::response::IntoResponse;
use chrono::{Utc, DateTime};
use rust_kms_local::fs::Wrapper;

use crate::net::{self, error};
use crate::state::ArcShared;
use crate::sec::secrets::Key;
use crate::time;

pub async fn get(
    State(state): State<ArcShared>,
) -> error::Result<impl IntoResponse> {
    let peppers = state.sec().peppers().inner();
    let mut known_versions;

    {
        let reader = peppers.read()
            .map_err(|_| error::Error::new().source("peppers rwlock poisoned"))?;
        known_versions = Vec::with_capacity(reader.len());

        for (version, key) in reader.iter() {
            known_versions.push(schema::sec::PasswordListItem {
                version: *version,
                created: DateTime::<Utc>::from_timestamp(*key.created() as i64, 0)
                    .ok_or(error::Error::new()
                        .kind("TimestampError")
                        .message("failed to create timestamp for password key"))?
            });
        }
    }

    Ok(net::Json::new(rfs_lib::json::ListWrapper::with_vec(known_versions)))
}

pub async fn post(
    State(state): State<ArcShared>,
) -> error::Result<impl IntoResponse> {
    let wrapper = state.sec().peppers();
    let data = Key::rand_key_data()?;
    let created = match time::utc_now() {
        Some(d) => d.as_secs(),
        None => {
            return Err(error::Error::new()
                .source("failed to create timestamp"));
        }
    };

    let key = Key::new(data, created);

    {
        let mut writer = wrapper.inner().write()
            .map_err(|_| error::Error::new()
                .source("peppers rwlock poisoned"))?;

        writer.update(key);
    }

    wrapper.save()
        .map_err(|e| error::Error::new()
            .kind("failedSavingPeppers")
            .message("failed to save updated peppers to file")
            .source(e))?;

    Ok(net::Json::empty())
}
