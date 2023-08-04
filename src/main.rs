use std::fmt::{Debug, Display};
use tokio::task::JoinError;
use zero2prod::configuration::get_configuration;
use zero2prod::issue_delivery_worker::run_worker_until_stopped;
use zero2prod::startup::Application;
use zero2prod::telemetry::{get_subscriber, subscriber_init};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Setting up log env
    let subscriber = get_subscriber("zero2prod".into(), "trace".into(), std::io::stdout);
    subscriber_init(subscriber);

    // Panic if for some resson we can't read the configuration file
    let configuration = get_configuration().expect("Failed to read configuration.");

    let application = Application::build(configuration.clone()).await?;
    let application_task = tokio::spawn(application.run_until_stopped());
    let worker_task = tokio::spawn(run_worker_until_stopped(configuration));

    // wait on multiple concurrent futrues
    // pitfal with select
    //
    // By running all async expressions on the current task, the expressions are able to run concurrently but
    // not in parallel. This means all expressions are run on the same thread and if one branch blocks the
    // thread, all other expressions will be unable to continue. If parallelism is required, spawn each async
    // expression using tokio::spawn and pass the join handle to select!.

    tokio::select! {
        o = application_task => report_exit("API", o),
        o = worker_task => report_exit("Newsletter delivery background worker", o),
    };

    Ok(())
}

fn report_exit(task_name: &str, outcome: Result<Result<(), impl Debug + Display>, JoinError>) {
    match outcome {
        Ok(Ok(())) => tracing::info!("{} has exited", task_name),
        Ok(Err(e)) => {
            tracing::error!(error.cause_chain = ?e, error.message = %e, "{} failed", task_name)
        }
        Err(e) => {
            tracing::error!(error.cause_chain = ?e, error.message = %e, "{} task failed to complete", task_name)
        }
    }
}
