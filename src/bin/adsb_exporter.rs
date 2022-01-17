use adsb_exporter::ADSBExporter;
use adsb_exporter::Configuration;
use adsb_exporter::DumpWatcher;

use anyhow::anyhow;
use anyhow::Result;

use clap::ErrorKind;
use clap::IntoApp;
use clap::Parser;

use env_logger::Builder;
use env_logger::Env;

use log::error;

use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<()> {
    Builder::from_env(Env::default().default_filter_or("info")).init();

    let configuration = Configuration::parse();

    if configuration.enable_console_subscriber {
        console_subscriber::init();
    }

    if configuration.dump1090_url.is_none() && configuration.dump978_url.is_none() {
        let mut app = Configuration::into_app();
        app.error(
            ErrorKind::MissingRequiredArgument,
            "You must provide at least one dump URL",
        )
        .exit();
    }

    if let Some(ref base_uri) = configuration.dump1090_url {
        DumpWatcher::new(&configuration, 1090, base_uri.to_string())
            .start()
            .await;
    };

    if let Some(ref base_uri) = configuration.dump978_url {
        DumpWatcher::new(&configuration, 978, base_uri.to_string())
            .start()
            .await;
    };

    let (error_tx, error_rx) = mpsc::channel(1);

    ADSBExporter::new(configuration.bind_address)?
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
