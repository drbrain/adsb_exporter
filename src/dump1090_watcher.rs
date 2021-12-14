use crate::configuration::Configuration;
use crate::DURATIONS;
use crate::ERRORS;
use crate::REQUESTS;

use anyhow::Context;
use anyhow::Result;

use lazy_static::lazy_static;

use log::debug;
use log::info;

use prometheus::register_gauge_vec;
use prometheus::GaugeVec;

use reqwest::Client;

use serde_json::json;
use serde_json::Value;

use std::time::Duration;

use tokio::time::sleep;

lazy_static! {
    pub static ref AIRCRAFT_RECENT_OBSERVED: GaugeVec = register_gauge_vec!(
        "adsb_aircraft_recent_observed_total",
        "Number of aircraft recently observed",
        &[],
    )
    .unwrap();
    pub static ref AIRCRAFT_RECENT_POSITIONS: GaugeVec = register_gauge_vec!(
        "adsb_aircraft_recent_positions_total",
        "Number of aircraft recently observed with a position",
        &[],
    )
    .unwrap();
    pub static ref AIRCRAFT_RECENT_MLAT: GaugeVec = register_gauge_vec!(
        "adsb_aircraft_recent_mlat_total",
        "Number of aircraft recently observed with a position determined by multilateration",
        &[],
    )
    .unwrap();
}

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

        let client = self.client.clone();
        let aircraft_url = format!("{}/data/{}", self.base_uri, "aircraft.json");

        tokio::spawn(async move {
            run_aircraft(client, aircraft_url, self.aircraft_interval).await;
        });
    }

    pub async fn start(self) {
        tokio::spawn(async move {
            self.run().await;
        });
    }
}

async fn fetch(client: &Client, url: String) -> Option<Value> {
    debug!("Fetching {}", url);
    REQUESTS.with_label_values(&[&url]).inc();
    let timer = DURATIONS.with_label_values(&[&url]).start_timer();

    let response = client.get(&url).send().await;

    timer.observe_duration();

    let response = match response {
        Ok(r) => r,
        Err(e) => {
            debug!("request error: {:?}", e);
            ERRORS.with_label_values(&[&url, "request"]).inc();
            return None;
        }
    };

    let body = match response.text().await {
        Ok(t) => t,
        Err(e) => {
            debug!("Response body error from {}: {:?}", url, e);
            ERRORS.with_label_values(&[&url, "text"]).inc();
            return None;
        }
    };

    match serde_json::from_str(&body) {
        Ok(j) => Some(j),
        Err(e) => {
            debug!("JSON parsing error from {}: {:?}", url, e);
            ERRORS.with_label_values(&[&url, "json"]).inc();
            None
        }
    }
}

async fn run_aircraft(client: Client, url: String, interval: Duration) {
    loop {
        if let Some(data) = fetch(&client, url.clone()).await {
            match update_aircraft(data) {
                Ok(_) => (),
                Err(e) => {
                    debug!("error updating aircraft {:?}", e);
                }
            };
        }

        sleep(interval).await;
    }
}

fn contains(vec: &Vec<Value>, target: String) -> bool {
    vec.iter()
        .any(|v| v.as_str().unwrap_or(&"".to_string()) == target)
}

fn update_aircraft(data: Value) -> Result<()> {
    let aircrafts = data
        .get("aircraft")
        .context("missing aircraft data")?
        .as_array()
        .context("aircraft data not an Array")?;

    let observed = aircrafts
        .iter()
        .filter(|a| {
            a.get("seen")
                .unwrap_or(&json!(()))
                .as_f64()
                .unwrap_or(f64::INFINITY)
                < 60.0
        })
        .count();

    let positions = aircrafts
        .iter()
        .filter(|a| {
            a.get("seen_pos")
                .unwrap_or(&json!(()))
                .as_f64()
                .unwrap_or(f64::INFINITY)
                < 60.0
        })
        .count();

    let mlat = aircrafts
        .iter()
        .filter(|a| {
            a.get("seen_pos")
                .unwrap_or(&json!(()))
                .as_f64()
                .unwrap_or(f64::INFINITY)
                < 60.0
                && contains(
                    a.get("mlat").unwrap_or(&json!([])).as_array().unwrap(),
                    "lat".to_string(),
                )
        })
        .count();

    AIRCRAFT_RECENT_OBSERVED
        .with_label_values(&[])
        .set(observed as f64);
    AIRCRAFT_RECENT_POSITIONS
        .with_label_values(&[])
        .set(positions as f64);
    AIRCRAFT_RECENT_MLAT.with_label_values(&[]).set(mlat as f64);

    debug!(
        "aircraft observed: {}, position: {} mlat: {}",
        observed, positions, mlat
    );

    Ok(())
}
