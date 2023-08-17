use std::{
    str::FromStr,
    sync::{atomic, Arc},
};

use chrono::{DateTime, Duration, Utc};
use strum::{AsRefStr, Display, EnumIter, EnumString};
use tokio::task::JoinHandle;
use tracing::{debug, info, instrument, warn};

mod jobs;

use crate::{
    configuration::Configuration,
    database::{model::ScheduledJob, RVocAsyncDatabaseConnectionPool},
    error::{RVocError, RVocResult},
    job_queue::jobs::update_witkionary::update_wiktionary,
};

#[instrument(err, skip(database_connection_pool, configuration))]
pub async fn spawn_job_queue_runner(
    database_connection_pool: &RVocAsyncDatabaseConnectionPool,
    shutdown_flag: Arc<atomic::AtomicBool>,
    configuration: &Configuration,
) -> RVocResult<JoinHandle<RVocResult<()>>> {
    initialise_job_queue(database_connection_pool, configuration).await?;

    let database_connection_pool = database_connection_pool.clone();
    let configuration = configuration.clone();

    info!("Spawning job queue runner");
    Ok(tokio::spawn(async move {
        use tokio::time;

        let mut interval = time::interval(time::Duration::from_secs(1));
        interval.set_missed_tick_behavior(time::MissedTickBehavior::Skip);

        while !shutdown_flag.load(atomic::Ordering::Relaxed) {
            interval.tick().await;
            poll_job_queue_and_execute(&database_connection_pool, &configuration).await?;
        }

        Ok(())
    }))
}

#[instrument(err, skip(database_connection_pool, configuration))]
async fn initialise_job_queue(
    database_connection_pool: &RVocAsyncDatabaseConnectionPool,
    configuration: &Configuration,
) -> RVocResult<()> {
    info!("Initialising job queue");

    database_connection_pool
        .execute_transaction_with_retries::<_, RVocError>(
            |database_connection| {
                Box::pin(async move {
                    use crate::database::schema::job_queue::dsl::*;
                    use diesel::{dsl::now, ExpressionMethods};
                    use diesel_async::RunQueryDsl;
                    use strum::IntoEnumIterator;

                    let valid_job_names: Vec<_> = JobName::iter().collect();

                    // Insert missing jobs.
                    diesel::insert_into(job_queue)
                        .values(
                            valid_job_names
                                .iter()
                                .map(|job_name| {
                                    (
                                        scheduled_execution_time.eq(now),
                                        name.eq(job_name.as_ref()),
                                        in_progress.eq(false),
                                    )
                                })
                                .collect::<Vec<_>>(),
                        )
                        .on_conflict_do_nothing()
                        .execute(database_connection)
                        .await?;

                    // Delete unknown jobs.
                    let deleted_job_names = diesel::delete(job_queue)
                        .filter(
                            name.ne_all(
                                valid_job_names
                                    .iter()
                                    .map(AsRef::as_ref)
                                    .collect::<Vec<_>>(),
                            ),
                        )
                        .returning(name)
                        .get_results::<String>(database_connection)
                        .await?;

                    for deleted_job_name in deleted_job_names {
                        warn!("Deleted unknown scheduled job with name: {deleted_job_name:?}");
                    }

                    Ok(())
                })
            },
            configuration.maximum_transaction_retry_count,
        )
        .await?;

    Ok(())
}

#[instrument(err, skip(database_connection_pool, configuration))]
async fn poll_job_queue_and_execute(
    database_connection_pool: &RVocAsyncDatabaseConnectionPool,
    configuration: &Configuration,
) -> RVocResult<()> {
    if let Some(job) = reserve_job(database_connection_pool, configuration).await? {
        debug!("Executing job {job:?}");

        match job.name {
            JobName::UpdateWiktionary => {
                update_wiktionary(database_connection_pool, configuration).await?
            }
        }

        complete_job(job, database_connection_pool, configuration).await
    } else {
        Ok(())
    }
}

/// Check if there is a job to be executed.
/// If yes, then mark it as "in progress" and return it.
#[instrument(err, skip(database_connection_pool, configuration))]
async fn reserve_job(
    database_connection_pool: &RVocAsyncDatabaseConnectionPool,
    configuration: &Configuration,
) -> RVocResult<Option<InProgressJob>> {
    database_connection_pool
        .execute_transaction_with_retries::<_, RVocError>(
            |database_connection| {
                Box::pin(async move {
                    use crate::database::schema::job_queue::dsl::*;
                    use diesel::Identifiable;
                    use diesel::OptionalExtension;
                    use diesel::QueryDsl;
                    use diesel::SelectableHelper;
                    use diesel::{dsl::now, ExpressionMethods};
                    use diesel_async::RunQueryDsl;

                    // See if there is a job available.
                    let queued_job = job_queue
                        .select(ScheduledJob::as_select())
                        .filter(scheduled_execution_time.ge(now))
                        .filter(in_progress.eq(false))
                        .order_by(scheduled_execution_time.asc())
                        .first(database_connection)
                        .await
                        .optional()?;

                    if let Some(mut queued_job) = queued_job {
                        // Convert the job name into JobName.
                        // If it does not exist, then we delete the corresponding job.
                        let job_name = match JobName::from_str(&queued_job.name) {
                            Ok(job_name) => job_name,
                            Err(error) => {
                                warn!(
                                    "Error decoding job name, deleting corresponding job: {error}"
                                );
                                diesel::delete(&queued_job)
                                    .execute(database_connection)
                                    .await?;
                                return Ok(None);
                            }
                        };

                        // Check if job is still running.
                        if let Some(running_job) = job_queue
                            .select(ScheduledJob::as_select())
                            .filter(name.eq(job_name.to_string()))
                            .filter(in_progress.eq(true))
                            .first(database_connection)
                            .await
                            .optional()?
                        {
                            warn!("Job is still running: {:?}", running_job.id());
                            return Ok(None);
                        }

                        // Set the current job as in progress.
                        queued_job.in_progress = true;
                        diesel::update(job_queue)
                            .set(&queued_job)
                            .execute(database_connection)
                            .await?;

                        // let start_time = diesel::select(now).get_result(database_connection).await?;

                        let job = InProgressJob {
                            scheduled_time: queued_job.scheduled_execution_time,
                            start_time: Utc::now(),
                            name: job_name,
                        };

                        if job.start_time - job.scheduled_time
                            > configuration.job_queue_poll_interval + Duration::seconds(10)
                        {
                            warn!(
                            "Job started with a delay larger than the job queue poll interval: {}",
                            configuration.job_queue_poll_interval
                        );
                        }

                        Ok(Some(job))
                    } else {
                        Ok(None)
                    }
                })
            },
            configuration.maximum_transaction_retry_count,
        )
        .await
        .map_err(|error| RVocError::AccessJobQueue {
            source: Box::new(error),
        })
}

/// Check if there is a job to be executed.
/// If yes, then mark it as "in progress" and return it.
#[instrument(err, skip(database_connection_pool, configuration))]
async fn complete_job(
    job: InProgressJob,
    database_connection_pool: &RVocAsyncDatabaseConnectionPool,
    configuration: &Configuration,
) -> RVocResult<()> {
    let finish_time = Utc::now();
    let completed_job = job.finish(finish_time);
    let completed_job = &completed_job;

    debug!(
        "Completed job {} with a duration of {} after a start delayed by {}",
        completed_job.name,
        completed_job.duration(),
        completed_job.delay()
    );

    database_connection_pool
        .execute_transaction_with_retries::<_, RVocError>(
            |database_connection| {
                Box::pin(async move {
                    use crate::database::schema::job_queue::dsl::*;
                    use diesel_async::RunQueryDsl;

                    // Schedule the next execution.
                    let next_scheduled_execution = ScheduledJob {
                        scheduled_execution_time: completed_job
                            .schedule_next_execution(configuration),
                        name: completed_job.name.to_string(),
                        in_progress: false,
                    };
                    if next_scheduled_execution.scheduled_execution_time < Utc::now() {
                        warn!("Scheduled job in the past: {next_scheduled_execution:?}");
                    }

                    diesel::update(job_queue)
                        .set(next_scheduled_execution)
                        .execute(database_connection)
                        .await?;

                    Ok(())
                })
            },
            configuration.maximum_transaction_retry_count,
        )
        .await
        .map_err(|error| RVocError::AccessJobQueue {
            source: Box::new(error),
        })
}

#[derive(Debug, Eq, PartialEq, Clone, Copy, EnumString, Display, AsRefStr, EnumIter)]
pub enum JobName {
    UpdateWiktionary,
}

#[derive(Debug)]
struct InProgressJob {
    scheduled_time: DateTime<Utc>,
    start_time: DateTime<Utc>,
    name: JobName,
}

#[derive(Debug)]
struct CompletedJob {
    scheduled_time: DateTime<Utc>,
    start_time: DateTime<Utc>,
    finish_time: DateTime<Utc>,
    name: JobName,
}

impl InProgressJob {
    fn finish(self, finish_time: DateTime<Utc>) -> CompletedJob {
        CompletedJob {
            scheduled_time: self.scheduled_time,
            start_time: self.start_time,
            finish_time,
            name: self.name,
        }
    }
}

impl CompletedJob {
    fn schedule_next_execution(&self, configuration: &Configuration) -> DateTime<Utc> {
        match self.name {
            JobName::UpdateWiktionary => {
                self.finish_time + configuration.wiktionary_update_interval
            }
        }
    }

    fn delay(&self) -> Duration {
        self.start_time - self.scheduled_time
    }

    fn duration(&self) -> Duration {
        self.finish_time - self.start_time
    }
}
