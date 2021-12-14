use serde::Deserialize;

use std::fs;
use std::path::Path;
use std::time::Duration;

#[derive(Default, Deserialize)]
pub struct Configuration {
    bind_address: Option<String>,
    dump1090_url: Option<String>,
    dump978_url: Option<String>,

    aircraft_refresh_interval: Option<u64>,
    receiver_refresh_interval: Option<u64>,
    stats_refresh_interval: Option<u64>,

    refresh_timeout: Option<u64>,
}

impl Configuration {
    // Load a configuration file from `path`.
    pub fn load<P: AsRef<Path>>(path: P) -> Self {
        let source = fs::read_to_string(path).unwrap();

        toml::from_str(&source).unwrap()
    }

    // Load configuration from the next argument in the environment.
    pub fn load_from_next_arg() -> Self {
        let file = match std::env::args().nth(1) {
            None => {
                return Configuration::default();
            }
            Some(f) => f,
        };

        Configuration::load(file)
    }

    // Bind address for Prometheus metric server
    pub fn bind_address(&self) -> String {
        self.bind_address
            .as_ref()
            .unwrap_or(&"0.0.0.0:9190".to_string())
            .to_string()
    }

    pub fn dump1090_url(&self) -> Option<String> {
        self.dump1090_url.clone()
    }

    pub fn dump978_url(&self) -> Option<String> {
        self.dump978_url.clone()
    }

    // Timeout for HTTP requests of json sources.  Defaults to 150 milliseconds.
    pub fn refresh_timeout(&self) -> Duration {
        Duration::from_millis(self.refresh_timeout.unwrap_or(150))
    }

    // Interval between refreshes of aircraft.json.  Defaults to 30 seconds.
    pub fn aircraft_refresh_interval(&self) -> Duration {
        let interval = self.aircraft_refresh_interval.unwrap_or(30_000);

        Duration::from_millis(interval)
    }

    // Interval between refreshes of receiver.json.  Defaults to 5 minutes.
    pub fn receiver_refresh_interval(&self) -> Duration {
        let interval = self.receiver_refresh_interval.unwrap_or(300_000);

        Duration::from_millis(interval)
    }

    // Interval between refreshes of stats.json.  Defaults to 1 minute.
    pub fn stats_refresh_interval(&self) -> Duration {
        let interval = self.stats_refresh_interval.unwrap_or(60_000);

        Duration::from_millis(interval)
    }
}
