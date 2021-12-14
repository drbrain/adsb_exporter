mod adsb_exporter;
mod aircraft_json;
mod configuration;
mod dump_watcher;

use anyhow::anyhow;
use anyhow::Result;

use adsb_exporter::ADSBExporter;
use configuration::Configuration;
use dump_watcher::DumpWatcher;

use env_logger::Builder;
use env_logger::Env;

use lazy_static::lazy_static;

use log::error;

use prometheus::register_histogram_vec;
use prometheus::register_int_counter_vec;
use prometheus::HistogramVec;
use prometheus::IntCounterVec;

use tokio::sync::mpsc;

lazy_static! {
    pub static ref REQUESTS: IntCounterVec = register_int_counter_vec!(
        "adsb_http_requests_total",
        "Number of HTTP requests made to fetch metrics",
        &["uri"],
    )
    .unwrap();
    pub static ref ERRORS: IntCounterVec = register_int_counter_vec!(
        "adsb_http_request_errors_total",
        "Number of HTTP request errors returned from fetching metrics",
        &["uri", "error_type"],
    )
    .unwrap();
    pub static ref DURATIONS: HistogramVec = register_histogram_vec!(
        "adsb_http_request_duration_seconds",
        "HTTP request durations",
        &["uri"],
    )
    .unwrap();
}

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
