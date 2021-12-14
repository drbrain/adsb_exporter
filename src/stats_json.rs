use crate::fetch::fetch;

use anyhow::Context;
use anyhow::Result;

use lazy_static::lazy_static;

use log::debug;

use prometheus::register_gauge_vec;
use prometheus::register_int_counter_vec;
use prometheus::GaugeVec;
use prometheus::IntCounterVec;

use reqwest::Client;

use serde_json::Value;

use std::time::Duration;

use tokio::time::sleep;

macro_rules! set_counter {
    ( $metric:ident, $labels:expr, $source:ident, $field:literal, $conversion:ident ) => {
        if let Some(value) = $source.get($field) {
            if let Some(value) = value.$conversion() {
                let increment = value - $metric.with_label_values($labels).get();

                $metric.with_label_values($labels).inc_by(increment);
            }
        }
    };
}

macro_rules! set_gauge {
    ( $metric:ident, $labels:expr, $source:ident, $field:literal, $conversion:ident ) => {
        if let Some(value) = $source.get($field) {
            if let Some(value) = value.$conversion() {
                $metric.with_label_values($labels).set(value);
            }
        }
    };
}

lazy_static! {
    static ref LOCAL_SAMPLES_PROCESSED: IntCounterVec = register_int_counter_vec!(
        "adsb_stats_local_samples_processed_total",
        "Number of samples processed",
        &[&"frequency"],
    )
    .unwrap();
    static ref LOCAL_SAMPLES_DROPPED: IntCounterVec = register_int_counter_vec!(
        "adsb_stats_local_samples_dropped_total",
        "Number of samples dropped before processing, a nonzero value means CPU overload",
        &[&"frequency"],
    )
    .unwrap();
    static ref LOCAL_MODEAC: IntCounterVec = register_int_counter_vec!(
        "adsb_stats_local_modeac_decoded_total",
        "Number of mode A/C messages decoded",
        &[&"frequency"],
    )
    .unwrap();
    static ref LOCAL_MODES: IntCounterVec = register_int_counter_vec!(
        "adsb_stats_local_modes_preambles_total",
        "Number of mode S preambles recieved",
        &[&"frequency"],
    )
    .unwrap();
    static ref LOCAL_MODES_BAD: IntCounterVec = register_int_counter_vec!(
        "adsb_stats_local_modes_bad_total",
        "Number of Mode S preambles that didn't result in a valid message",
        &[&"frequency"],
    )
    .unwrap();
    static ref LOCAL_UNKNOWN_ICAO: IntCounterVec = register_int_counter_vec!(
        "adsb_stats_local_modes_unknown_icao_total",
        "Number of Mode S preambles with an unknown ICAO address",
        &[&"frequency"],
    )
    .unwrap();
    static ref LOCAL_UNKNOWN_ACCEPTED: IntCounterVec = register_int_counter_vec!(
        "adsb_stats_local_modes_accepted_total",
        "Number of valid Mode S messages labeled by N-bit error corrections",
        &[&"frequency", "corrections"],
    )
    .unwrap();
    static ref LOCAL_SIGNAL: GaugeVec = register_gauge_vec!(
        "adsb_stats_local_signal_dbfs_mean",
        "Mean signal power of received messages in dbFS",
        &[&"frequency"],
    )
    .unwrap();
    static ref LOCAL_SIGNAL_PEAK: GaugeVec = register_gauge_vec!(
        "adsb_stats_local_signal_dbfs_peak",
        "Peak signal power of received messages in dBFS",
        &[&"frequency"],
    )
    .unwrap();
    static ref LOCAL_NOISE: GaugeVec = register_gauge_vec!(
        "adsb_stats_local_noise_dbfs_mean",
        "Mean signal noise of received messages in dBFS",
        &[&"frequency"],
    )
    .unwrap();
    static ref LOCAL_STRONG_SIGNALS: IntCounterVec = register_int_counter_vec!(
        "adsb_stats_local_strong_signals_total",
        "Number of messages received with a signal power above -3dBFS",
        &[&"frequency", "corrections"],
    )
    .unwrap();
}

pub struct StatsJson {
    client: Client,
    frequency: String,
    url: String,
    interval: Duration,
}

impl StatsJson {
    pub fn new(client: Client, frequency: u32, url: String, interval: Duration) -> StatsJson {
        let frequency = frequency.to_string();

        StatsJson {
            client,
            frequency,
            url,
            interval,
        }
    }

    pub async fn run(&self) {
        debug!(
            "Watching stats dump{} at {} every {:?}",
            self.frequency, self.url, self.interval
        );

        loop {
            if let Some(data) = fetch(&self.client, &self.url).await {
                match self.update_stats(data) {
                    Ok(_) => (),
                    Err(e) => {
                        debug!("error updating stats {:?}", e);
                    }
                };
            }

            sleep(self.interval).await;
        }
    }

    fn update_stats(&self, data: Value) -> Result<()> {
        let total = data.get("total").context("missing total data")?;

        let local = total
            .get("local")
            .context("Missing local data in \"total\" object")?;

        set_counter!(
            LOCAL_SAMPLES_PROCESSED,
            &[&self.frequency],
            local,
            "samples_processed",
            as_u64
        );
        set_counter!(
            LOCAL_SAMPLES_DROPPED,
            &[&self.frequency],
            local,
            "samples_dropped",
            as_u64
        );
        set_counter!(LOCAL_MODEAC, &[&self.frequency], local, "modeac", as_u64);
        set_counter!(LOCAL_MODES, &[&self.frequency], local, "modes", as_u64);
        set_counter!(LOCAL_MODES_BAD, &[&self.frequency], local, "bad", as_u64);
        set_counter!(
            LOCAL_UNKNOWN_ICAO,
            &[&self.frequency],
            local,
            "unknown_icao",
            as_u64
        );

        Ok(())
    }
}
