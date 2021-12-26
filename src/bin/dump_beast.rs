use anyhow::Result;

use adsb_exporter::beast::Codec;

use futures_util::StreamExt;

use tokio::fs::File;

use tokio_util::codec::Framed;

#[tokio::main]
async fn main() -> Result<()> {
    console_subscriber::init();

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let file = std::env::args().nth(1).unwrap();
    let stream = File::open(file).await?;

    let mut reader = Framed::new(stream, Codec::new());

    while let Some(message) = reader.next().await {
        eprintln!("{:#?}", message);
    }

    Ok(())
}
