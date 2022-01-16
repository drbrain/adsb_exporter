use clap::Parser;

use std::net::SocketAddr;
use std::time::Duration;

/// A Prometheus exporter for ADSB message receivers like dump1090 and dump978
#[derive(Parser)]
#[clap(about, version)]
pub struct Configuration {
    /// Bind address for prometheus exporter
    #[clap(long, default_value = "0.0.0.0:9190")]
    pub bind_address: SocketAddr,

    /// URL of the dump1090 server
    #[clap(long)]
    pub dump1090_url: Option<String>,

    /// URL of the dump978 server
    #[clap(long)]
    pub dump978_url: Option<String>,

    /// Refresh interval in seconds for aircraft.json
    #[clap(long, default_value = "30", parse(try_from_str = secs_to_duration))]
    pub aircraft_refresh_interval: Duration,

    /// Refresh interval in seconds for receiver.json
    #[clap(long, default_value = "300", parse(try_from_str = secs_to_duration))]
    pub receiver_refresh_interval: Duration,

    /// Refresh interval in seconds for stats.json
    #[clap(long, default_value = "60", parse(try_from_str = secs_to_duration))]
    pub stats_refresh_interval: Duration,

    /// Refresh timeout in milliseconds for requests to dump program URLs
    #[clap(long, default_value = "150", parse(try_from_str = millis_to_duration))]
    pub refresh_timeout: Duration,
}

fn millis_to_duration(s: &str) -> Result<Duration, &'static str> {
    match s.parse::<u64>() {
        Ok(secs) => Ok(Duration::from_millis(secs)),
        Err(_) => Err("expected duration milliseconds"),
    }
}

fn secs_to_duration(s: &str) -> Result<Duration, &'static str> {
    match s.parse::<u64>() {
        Ok(secs) => Ok(Duration::from_secs(secs)),
        Err(_) => Err("expected duration seconds"),
    }
}
