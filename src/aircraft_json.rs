use crate::fetch::fetch;

use anyhow::Context;
use anyhow::Result;

use geo::algorithm::bearing::Bearing;
use geo::prelude::*;
use geo::Coordinate;
use geo::Line;
use geo::Point;

use lazy_static::lazy_static;

use log::debug;

use prometheus::register_gauge_vec;
use prometheus::register_int_gauge_vec;
use prometheus::GaugeVec;
use prometheus::IntGaugeVec;

use reqwest::Client;

use serde_json::json;
use serde_json::Value;

use std::collections::HashMap;
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
    static ref OBSERVATIONS: IntGaugeVec = register_int_gauge_vec!(
        "adsb_aircraft_observations",
        "Number of aircraft positions observed by range and bearing",
        &[&"frequency", &"bearing", &"distance"],
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

        let receiver_position = Coordinate {
            x: 47.59,
            y: -122.30,
        };
        let receiver_point: Point<f64> = receiver_position.into();

        let mut observations = HashMap::with_capacity(positions);

        aircrafts
            .iter()
            .filter(|a| {
                if let Some(seen) = a.get("seen_pos") {
                    seen.as_f64().unwrap_or(f64::INFINITY) < 60.0
                } else {
                    false
                }
            })
            .for_each(|a| {
                let aircraft_lat = a.get("lat").unwrap().as_f64().unwrap();
                let aircraft_lon = a.get("lon").unwrap().as_f64().unwrap();

                let aircraft_position = Coordinate {
                    x: aircraft_lat,
                    y: aircraft_lon,
                };

                let distance = Line::new(aircraft_position, receiver_position).haversine_length();
                let distance_bucket = (1 + (distance / 80_000.0) as u32) * 80_000;
                let distance_bucket = match distance_bucket {
                    0..=400_000 => distance_bucket.to_string(),
                    _ => "> 400000".to_string(),
                };

                let bearing = receiver_point.bearing(aircraft_position.into());
                // The documentation says "North is 0° and East is 90°" but this doesn't seem to
                // match the results, so we need to rotate by 90, then add positive bearing.
                let bearing = (450.0 + bearing) % 360.0;

                let bearing_bucket = (((bearing + 11.25) / 22.5).floor() * 22.5) % 360.0;
                let bearing_bucket = bearing_bucket.to_string();

                let key = (distance_bucket, bearing_bucket);
                let previous = observations.get(&key).unwrap_or(&0);
                let count = *previous + 1;

                observations.insert(key, count);
            });

        RECENT_OBSERVED
            .with_label_values(&[&self.frequency])
            .set(observed as f64);
        RECENT_POSITIONS
            .with_label_values(&[&self.frequency])
            .set(positions as f64);
        RECENT_MLAT
            .with_label_values(&[&self.frequency])
            .set(mlat as f64);
        observations
            .iter()
            .for_each(|((distance, bearing), count)| {
                OBSERVATIONS
                    .with_label_values(&[&self.frequency, &bearing, &distance])
                    .set(*count)
            });

        Ok(())
    }
}
