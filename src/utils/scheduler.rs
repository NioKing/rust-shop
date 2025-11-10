use tokio_cron_scheduler::{Job, JobScheduler};

pub async fn spawn_job<Fut: Future<Output = Result<(), String>> + Send + 'static>(
    sheduler: &JobScheduler,
    schedule: &str,
    handler: impl Fn() -> Fut + Send + Sync + 'static + Copy,
) -> Result<(), String> {
    sheduler
        .add(
            Job::new_async(schedule, move |uuid, mut l| {
                Box::pin(async move {
                    println!("Hello from spawned job");
                    handler().await;
                })
            })
            .map_err(|_| "Cron task has failed".to_owned())?,
        )
        .await
        .map_err(|_| "Failed to run a scheduler task".to_owned())?;

    Ok(())
}
