use crate::aircraft_json::AircraftJson;
use crate::configuration::Configuration;

use log::info;

use reqwest::Client;

use std::time::Duration;

#[derive(Clone)]
pub struct Dump1090Watcher {
    client: Client,

    base_uri: String,

    aircraft_interval: Duration,
    receiver_interval: Duration,
    stats_interval: Duration,
}

impl Dump1090Watcher {
    pub fn new(configuration: &Configuration) -> Option<Self> {
        let aircraft_interval = configuration.aircraft_refresh_interval();
        let base_uri = match configuration.dump1090_url() {
            Some(uri) => uri,
            None => return None,
        };
        let receiver_interval = configuration.receiver_refresh_interval();
        let stats_interval = configuration.stats_refresh_interval();
        let timeout = configuration.refresh_timeout();

        let client = Client::builder()
            .connect_timeout(timeout)
            .http1_only()
            .timeout(timeout)
            .build()
            .expect("Could not build client");

        Some(Dump1090Watcher {
            client,
            base_uri,
            aircraft_interval,
            receiver_interval,
            stats_interval,
        })
    }

    pub async fn run(self) {
        info!("Watching dump1090 at {}", self.base_uri);

        let aircraft_url = format!("{}/data/{}", self.base_uri, "aircraft.json");
        let aircraft_json =
            AircraftJson::new(self.client.clone(), aircraft_url, self.aircraft_interval);

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
