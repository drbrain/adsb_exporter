use crate::aircraft_json::AircraftJson;
use crate::configuration::Configuration;

use log::info;

use reqwest::Client;

use std::time::Duration;

#[derive(Clone)]
pub struct DumpWatcher {
    frequency: u32,
    base_uri: String,

    client: Client,

    aircraft_interval: Duration,
    receiver_interval: Duration,
    stats_interval: Duration,
}

impl DumpWatcher {
    pub fn new(configuration: &Configuration, frequency: u32, base_uri: String) -> Self {
        let timeout = configuration.refresh_timeout();

        let client = Client::builder()
            .connect_timeout(timeout)
            .http1_only()
            .timeout(timeout)
            .build()
            .expect("Could not build HTTP client");

        let aircraft_interval = configuration.aircraft_refresh_interval();
        let receiver_interval = configuration.receiver_refresh_interval();
        let stats_interval = configuration.stats_refresh_interval();

        DumpWatcher {
            frequency,
            base_uri,
            client,
            aircraft_interval,
            receiver_interval,
            stats_interval,
        }
    }

    pub async fn run(self) {
        info!("Watching dump{} at {}", self.frequency, self.base_uri);

        let aircraft_url = format!("{}/data/{}", self.base_uri, "aircraft.json");
        let aircraft_json =
            AircraftJson::new(self.client.clone(), self.frequency, aircraft_url, self.aircraft_interval);

        tokio::spawn(async move {
            aircraft_json.run().await;
        });
    }

    pub async fn start(self) {
        tokio::spawn(async move {
            self.run().await;
        });
    }
}
