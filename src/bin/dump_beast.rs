use adsb_exporter::beast::Codec;
use anyhow::Result;
use clap::ArgGroup;
use clap::ErrorKind;
use clap::IntoApp;
use clap::Parser;
use futures_util::Stream;
use futures_util::StreamExt;
use std::fmt::Debug;
use std::marker::Unpin;
use tokio::fs::File;
use tokio_util::codec::Framed;

/// Dump messages from a BEAST server
#[derive(Parser)]
#[clap(about, version, group(ArgGroup::new("source").required(true).args(&["file", "server"])))]
struct Args {
    /// Process a file containing BEAST data
    #[clap(long)]
    pub file: Option<String>,

    /// Process messages from a BEAST server
    ///
    /// Server should be a host and port:
    /// * localhost:30005
    /// * 192.0.2.1:30005
    #[clap(long)]
    pub server: Option<String>,

    /// Enable console-subscriber
    #[clap(long)]
    pub enable_console_subscriber: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args = Args::parse();

    if args.enable_console_subscriber {
        console_subscriber::init();
    }

    if let Some(file) = args.file {
        read_file(file).await?
    } else if let Some(server) = args.server {
        read_socket(server).await?
    } else {
        let mut app = Args::into_app();
        app.error(
            ErrorKind::MissingRequiredArgument,
            "You must provide at least one BEAST source",
        )
        .exit();
    };

    Ok(())
}

async fn read_file(file: String) -> Result<()> {
    let stream = File::open(file).await?;

    let reader = Framed::new(stream, Codec::new());

    read(reader).await;

    Ok(())
}

async fn read_socket(server: String) -> Result<()> {
    let std_socket = std::net::TcpStream::connect(server)?;
    let stream = tokio::net::TcpStream::from_std(std_socket)?;

    let reader = Framed::new(stream, Codec::new());

    read(reader).await;

    Ok(())
}

async fn read<T>(mut reader: T)
where
    T: Stream + Unpin,
    <T as Stream>::Item: Debug,
{
    while let Some(message) = reader.next().await {
        println!("{:#?}", message);
    }
}
