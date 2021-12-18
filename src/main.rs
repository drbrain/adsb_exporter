mod adsb_exporter;
mod aircraft_json;
mod beast;
mod configuration;
mod dump_watcher;
mod fetch;
mod receiver_json;
mod stats_json;

use crate::adsb_exporter::ADSBExporter;
use crate::beast::Client;
use crate::configuration::Configuration;
use crate::dump_watcher::DumpWatcher;

use anyhow::anyhow;
use anyhow::Result;

use env_logger::Builder;
use env_logger::Env;

use log::error;

use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<()> {
    console_subscriber::init();

    Builder::from_env(Env::default().default_filter_or("info")).init();

    let configuration = Configuration::load_from_next_arg();

    let mut client = Client::new("pitime:30005".to_string()).await?;
    client.run().await;

    std::process::exit(5);

    if let Some(base_uri) = configuration.dump1090_url() {
        DumpWatcher::new(&configuration, 1090, base_uri)
            .start()
            .await;
    };

    if let Some(base_uri) = configuration.dump978_url() {
        DumpWatcher::new(&configuration, 978, base_uri)
            .start()
            .await;
    };

    let (error_tx, error_rx) = mpsc::channel(1);

    ADSBExporter::new(configuration.bind_address())?
        .start(error_tx.clone())
        .await;

    let exit_code = wait_for_error(error_rx).await;

    std::process::exit(exit_code);
}

async fn wait_for_error(mut error_rx: mpsc::Receiver<anyhow::Error>) -> i32 {
    let error = match error_rx.recv().await {
        Some(e) => e,
        None => anyhow!("Error reporting channel closed unexpectedly, bug?"),
    };

    error!("{:#}", error);

    1
}

#[track_caller]
pub(crate) fn spawn_named<T>(
    task: impl std::future::Future<Output = T> + Send + 'static,
    _name: &str,
) -> tokio::task::JoinHandle<T>
where
    T: Send + 'static,
{
    #[cfg(tokio_unstable)]
    return tokio::task::Builder::new().name(_name).spawn(task);

    #[cfg(not(tokio_unstable))]
    tokio::spawn(task)
}
