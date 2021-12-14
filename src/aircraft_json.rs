use crate::DURATIONS;
use crate::ERRORS;
use crate::REQUESTS;

use anyhow::Context;
use anyhow::Result;

use lazy_static::lazy_static;

use log::debug;

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

pub struct AircraftJson {
    client: Client,
    url: String,
    interval: Duration,
}

impl AircraftJson {
    pub fn new(client: Client, url: String, interval: Duration) -> AircraftJson {
        AircraftJson {
            url,
            client,
            interval,
        }
    }

    async fn fetch(&self) -> Option<Value> {
        debug!("Fetching {}", self.url);
        REQUESTS.with_label_values(&[&self.url]).inc();
        let timer = DURATIONS.with_label_values(&[&self.url]).start_timer();

        let response = self.client.get(&self.url).send().await;

        timer.observe_duration();

        let response = match response {
            Ok(r) => r,
            Err(e) => {
                debug!("request error: {:?}", e);
                ERRORS.with_label_values(&[&self.url, "request"]).inc();
                return None;
            }
        };

        let body = match response.text().await {
            Ok(t) => t,
            Err(e) => {
                debug!("Response body error from {}: {:?}", self.url, e);
                ERRORS.with_label_values(&[&self.url, "text"]).inc();
                return None;
            }
        };

        match serde_json::from_str(&body) {
            Ok(j) => Some(j),
            Err(e) => {
                debug!("JSON parsing error from {}: {:?}", self.url, e);
                ERRORS.with_label_values(&[&self.url, "json"]).inc();
                None
            }
        }
    }

    pub async fn run(&self) {
        loop {
            if let Some(data) = self.fetch().await {
                match self.update_aircraft(data) {
                    Ok(_) => (),
                    Err(e) => {
                        debug!("error updating aircraft {:?}", e);
                    }
                };
            }

            sleep(self.interval).await;
        }
    }

    fn update_aircraft(&self, data: Value) -> Result<()> {
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

        let lat = json!("lat");
        let empty_vec = vec![];

        let mlat = aircrafts
            .iter()
            .filter(|a| {
                a.get("seen_pos")
                    .unwrap_or(&json!(()))
                    .as_f64()
                    .unwrap_or(f64::INFINITY)
                    < 60.0
                    && a.get("mlat")
                        .unwrap_or(&json!([]))
                        .as_array()
                        .unwrap_or(&empty_vec)
                        .contains(&lat)
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
}
