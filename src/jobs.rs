use std::path::PathBuf;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use std::future::Future;
use std::io::ErrorKind;

use chrono::{DateTime, Local, TimeDelta};
use futures::future::{Abortable, AbortHandle, Aborted};
use futures::stream::FuturesUnordered;
use tracing::Instrument;
use tokio::task::JoinHandle;
use tokio::io::AsyncWriteExt;
use tokio::fs::File;
use serde::{Serialize, Deserialize};

use crate::state::ArcShared;
use crate::error::{self, Context};

mod session;
mod password;

#[derive(Debug, Serialize, Deserialize)]
struct JobInfo {
    last_run: Option<DateTime<Local>>
}

impl JobInfo {
    fn load(job_file: &PathBuf) -> error::Result<Self> {
        let result = std::fs::OpenOptions::new()
            .read(true)
            .open(job_file);

        match result {
            Ok(file) => serde_json::from_reader(&file)
                .context("failed to read jobs file"),
            Err(err) => match err.kind() {
                ErrorKind::NotFound => Ok(JobInfo::default()),
                _ => Err(err.into()),
            }
        }
    }

    async fn save(&self, job_file: &PathBuf) -> error::Result<()> {
        let json_buffer = serde_json::to_vec(self)
            .context("failed to create json job info")?;

        let mut file = File::options()
            .write(true)
            .create(true)
            .truncate(true)
            .open(job_file)
            .await
            .context("failed to open job file")?;

        file.write_all(&json_buffer)
            .await
            .context("failed to write job info to file")?;

        Ok(())
    }
}
impl std::default::Default for JobInfo {
    fn default() -> Self {
        JobInfo {
            last_run: None
        }
    }
}

// sec  min   hour    day of month   month   day of week   year
// 0    30    9,12,15     1,15       May-Aug  Mon,Wed,Fri  2018/2

async fn job_task<F, T>(
    state: ArcShared,
    mut upcoming: cron::OwnedScheduleIterator<Local>,
    mut job_info: JobInfo,
    job_file: PathBuf,
    runner: F
) -> error::Result<()>
where
    T: Future<Output = error::Result<()>>,
    F: Fn(ArcShared) -> T,
{
    let zero_delta = TimeDelta::zero();

    if job_info.last_run.is_none() {
        tracing::info!("job has never run. running job");

        if let Err(err) = runner(Arc::clone(&state)).await {
            tracing::error!("job failed with error: {err}");
        } else {
            let local_now = Local::now();

            tracing::debug!("job finished {local_now}");

            job_info.last_run = Some(local_now);

            job_info.save(&job_file).await?;
        }
    } else {
        let Some(current) = upcoming.next() else {
            tracing::info!("job finished");

            return Ok(());
        };

        let now = chrono::Local::now();
        let delta = current - now;

        if delta < zero_delta {
            tracing::info!("missed previous job. running job");

            if let Err(err) = runner(Arc::clone(&state)).await {
                tracing::error!("job failed with error: {err}");
            } else {
                let local_now = Local::now();

                tracing::debug!("job finished {local_now}");

                job_info.last_run = Some(local_now);

                job_info.save(&job_file).await?;
            }
        } else {
            tracing::debug!("moving upcoming iterator back one");

            upcoming.next_back();
        }
    }

    while let Some(next) = upcoming.next() {
        let now = Local::now();
        let delta = next - now;

        if delta < zero_delta {
            continue;
        }

        tracing::debug!("waiting for {delta}");

        tokio::time::sleep(delta.to_std().unwrap()).await;

        tracing::info!("running job");

        if let Err(err) = runner(Arc::clone(&state)).await {
            tracing::error!("failed with error: {err}");
        } else {
            let local_now = Local::now();

            tracing::debug!("job finished {local_now}");

            job_info.last_run = Some(local_now);

            job_info.save(&job_file).await?;
        }
    }

    tracing::info!("job finished");

    Ok(())
}

fn get_jobs_dir(data: PathBuf) -> error::Result<PathBuf> {
    let jobs_dir = data.join("jobs");

    let metadata = match jobs_dir.metadata() {
        Ok(m) => m,
        Err(err) => match err.kind() {
            ErrorKind::NotFound => {
                std::fs::create_dir(&jobs_dir)?;

                return Ok(jobs_dir);
            },
            _ => {
                return Err(err.into());
            }
        }
    };

    if !metadata.is_dir() {
        Err(error::Error::new()
            .message("jobs data directory is not a directory"))
    } else {
        Ok(jobs_dir)
    }
}

fn spawn_job<F, T>(
    jobs_dir: &PathBuf,
    state: &ArcShared,
    name: &'static str,
    crontab: &'static str,
    runner: F
) -> error::Result<JoinHandle<()>>
where
    T: Future<Output = error::Result<()>> + Send,
    F: Fn(ArcShared) -> T + Send + 'static,
{
    let local_state = Arc::clone(state);
    let job_file = jobs_dir.join(format!("{name}.json"));
    let job_info = JobInfo::load(&job_file)?;

    let schedule = cron::Schedule::from_str(crontab)
        .context("failed to parse crontab")?;

    let upcoming = if let Some(last_run) = job_info.last_run {
        schedule.after_owned(last_run)
    } else {
        schedule.upcoming_owned(Local)
    };

    Ok(tokio::spawn(async move {
        let job_span = tracing::span!(
            tracing::Level::INFO,
            "job",
            name = name
        );

        let result = job_task(local_state, upcoming, job_info, job_file, runner)
            .instrument(job_span)
            .await;

        if let Err(err) = result {
            tracing::error!("job {name} failed with error {err}");
        }
    }))
}

pub fn background(state: &ArcShared, data: PathBuf) -> error::Result<FuturesUnordered<JoinHandle<()>>> {
    let jobs_dir = get_jobs_dir(data)?;
    let mut waiter = FuturesUnordered::new();

    waiter.push(spawn_job(&jobs_dir, state, "session_cleanup", "0 0 0,12 * * * *", session::cleanup)?);
    waiter.push(spawn_job(&jobs_dir, state, "session_rotate", "0 0 0 1,15 * * *", session::rotate)?);
    waiter.push(spawn_job(&jobs_dir, state, "password_rotate", "0 0 0 1,15 * * *", password::rotate)?);

    Ok(waiter)
}
