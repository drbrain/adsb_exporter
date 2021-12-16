use crate::fetch::fetch;

use anyhow::Context;
use anyhow::Result;

use lazy_static::lazy_static;

use log::debug;

use prometheus::register_gauge_vec;
use prometheus::GaugeVec;

use reqwest::Client;

use serde_json::Value;

use std::time::Duration;

use tokio::time::sleep;

lazy_static! {
    static ref VERSION: GaugeVec = register_gauge_vec!(
        "adsb_receiver_version_info",
        "Version of the receiver software",
        &[&"frequency", "version"],
    )
    .unwrap();
    static ref POSITION: GaugeVec = register_gauge_vec!(
        "adsb_receiver_position_info",
        "Position of the receiver",
        &[&"frequency", &"latitude", &"longitude"],
    )
    .unwrap();
}

pub struct ReceiverJson {
    client: Client,
    frequency: String,
    url: String,
    interval: Duration,
}

impl ReceiverJson {
    pub fn new(client: Client, frequency: u32, url: String, interval: Duration) -> ReceiverJson {
        let frequency = frequency.to_string();

        ReceiverJson {
            client,
            frequency,
            url,
            interval,
        }
    }

    pub async fn run(&self) {
        loop {
            if let Some(data) = fetch(&self.client, &self.url).await {
                match self.update_receiver(data) {
                    Ok(_) => (),
                    Err(e) => {
                        debug!("error updating receiver {:?}", e);
                    }
                };
            }

            sleep(self.interval).await;
        }
    }

    fn update_receiver(&self, data: Value) -> Result<()> {
        let version = data
            .get("version")
            .context("Missing field version from receiver.json")?
            .as_str()
            .context("Field version from receiver.json is not a string")?
            .to_string();
        VERSION
            .with_label_values(&[&self.frequency, &version])
            .set(1.0);

        let latitude = data
            .get("lat")
            .context("Missing field lat from receiver.json")?
            .as_f64()
            .context("Field lat from receiver.json is not a number")?
            .to_string();
        let longitude = data
            .get("lon")
            .context("Missing field lon from receiver.json")?
            .as_f64()
            .context("Field lon from receiver.json is not a number")?
            .to_string();
        POSITION
            .with_label_values(&[&self.frequency, &latitude, &longitude])
            .set(1.0);

        Ok(())
    }
}
