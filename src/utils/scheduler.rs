use crate::error::{AppError, AppErrorKind};
use anyhow::Context;
use tokio_cron_scheduler::{Job, JobScheduler};

pub async fn spawn_job<Fut: Future<Output = Result<(), AppError>> + Send + 'static>(
    sheduler: &JobScheduler,
    schedule: &str,
    handler: impl Fn() -> Fut + Send + Sync + 'static + Copy,
) -> Result<(), AppError> {
    sheduler
        .add(
            Job::new_async(schedule, move |uuid, mut l| {
                Box::pin(async move {
                    println!("Hello from spawned job");
                    handler().await;
                })
            })
            .context("Cron task failed")?,
        )
        .await
        .context("Failed to run scheduler task")?;

    Ok(())
}
