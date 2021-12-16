mod adsb_exporter;
mod aircraft_json;
mod configuration;
mod dump_watcher;
mod fetch;
mod receiver_json;
mod stats_json;

use anyhow::anyhow;
use anyhow::Result;

use adsb_exporter::ADSBExporter;
use configuration::Configuration;
use dump_watcher::DumpWatcher;

use env_logger::Builder;
use env_logger::Env;

use log::error;

use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<()> {
    Builder::from_env(Env::default().default_filter_or("info")).init();

    let configuration = Configuration::load_from_next_arg();

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
