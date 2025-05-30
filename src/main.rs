use std::fmt::{Debug, Display};
use tokio::task::JoinError;
use zero2prod_my::configuration::get_configuration;
use zero2prod_my::issue_delivery_worker::run_worker_until_stopped;
use zero2prod_my::startup::Application;
use zero2prod_my::telemetry::{get_subscriber, init_subscriber};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let subscriber = get_subscriber("zero2prod".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);
    let configuration = get_configuration().expect("Failed to read configuration");
    // 需注意陷阱\ 下面的方法是 可并发不可并行.有一个任务阻塞了,都会阻塞.
    // let application = Application::build(configuration.clone())
    //     .await?
    //     .run_until_stopped();
    //
    // let worker = run_worker_until_stopped(configuration);


    let application = Application::build(configuration.clone())
        .await?
        .run_until_stopped();

    let application_task = tokio::spawn(application);

    let worker = run_worker_until_stopped(configuration);

    let worker_task = tokio::spawn(worker);
    tokio::select! {
        o = application_task =>report_exit("API", o),
        o = worker_task=>report_exit("Background worker", o),
    }
    Ok(())
}

fn report_exit(
    task_name: &str,
    outcome: Result<Result<(), impl Debug + Display>, JoinError>,
) {
    match outcome {
        Ok(Ok(())) => {
            tracing::info!("{} has exited", task_name)
        }
        Ok(Err(e)) => {
            tracing::error!(
                error.cause_chain = ?e,
                error.message = %e,
                "{} failed",
                task_name
            )
        }
        Err(e) => {
            tracing::error!(
                error.cause_chain = ?e,
                error.message = %e,
                "{}' task failed to complete",
                task_name
            )
        }
    }
}
