use crate::fetch::fetch;

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
    static ref RECENT_OBSERVED: GaugeVec = register_gauge_vec!(
        "adsb_aircraft_recent_observed_total",
        "Number of aircraft recently observed",
        &[&"frequency"],
    )
    .unwrap();
    static ref RECENT_POSITIONS: GaugeVec = register_gauge_vec!(
        "adsb_aircraft_recent_positions_total",
        "Number of aircraft recently observed with a position",
        &[&"frequency"],
    )
    .unwrap();
    static ref RECENT_MLAT: GaugeVec = register_gauge_vec!(
        "adsb_aircraft_recent_mlat_total",
        "Number of aircraft recently observed with a position determined by multilateration",
        &[&"frequency"],
    )
    .unwrap();
}

pub struct AircraftJson {
    client: Client,
    frequency: String,
    url: String,
    interval: Duration,
}

impl AircraftJson {
    pub fn new(client: Client, frequency: u32, url: String, interval: Duration) -> AircraftJson {
        let frequency = frequency.to_string();

        AircraftJson {
            client,
            frequency,
            url,
            interval,
        }
    }

    pub async fn run(&self) {
        loop {
            if let Some(data) = fetch(&self.client, &self.url).await {
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
                if let Some(seen) = a.get("seen") {
                    seen.as_f64().unwrap_or(f64::INFINITY) < 60.0
                } else {
                    false
                }
            })
            .count();

        let positions = aircrafts
            .iter()
            .filter(|a| {
                if let Some(seen) = a.get("seen_pos") {
                    seen.as_f64().unwrap_or(f64::INFINITY) < 60.0
                } else {
                    false
                }
            })
            .count();

        let lat = json!("lat");
        let empty_vec = vec![];

        let mlat = aircrafts
            .iter()
            .filter(|a| {
                let located = if let Some(seen) = a.get("seen_pos") {
                    seen.as_f64().unwrap_or(f64::INFINITY) < 60.0
                } else {
                    false
                };

                let mlat = if let Some(mlat) = a.get("mlat") {
                    mlat.as_array().unwrap_or(&empty_vec).contains(&lat)
                } else {
                    false
                };

                located && mlat
            })
            .count();

        RECENT_OBSERVED
            .with_label_values(&[&self.frequency])
            .set(observed as f64);
        RECENT_POSITIONS
            .with_label_values(&[&self.frequency])
            .set(positions as f64);
        RECENT_MLAT
            .with_label_values(&[&self.frequency])
            .set(mlat as f64);

        debug!(
            "aircraft observed: {}, position: {} mlat: {}",
            observed, positions, mlat
        );

        Ok(())
    }
}
