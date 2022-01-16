use anyhow::Result;

use adsb_exporter::beast::Codec;

use clap::Parser;

use futures_util::StreamExt;

use tokio::fs::File;

use tokio_util::codec::Framed;

/// Dump messages from a BEAST server
#[derive(Parser)]
#[clap(about, version)]
struct Args {
    /// Process a file containing BEAST data
    #[clap(long)]
    pub file: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    console_subscriber::init();

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args = Args::parse();

    let stream = File::open(args.file).await?;

    let mut reader = Framed::new(stream, Codec::new());

    while let Some(message) = reader.next().await {
        eprintln!("{:#?}", message);
    }

    Ok(())
}
