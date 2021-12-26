use crate::fetch::fetch;

use anyhow::Context;
use anyhow::Result;

use lazy_static::lazy_static;

use log::debug;

use prometheus::register_counter_vec;
use prometheus::register_gauge_vec;
use prometheus::register_int_counter_vec;
use prometheus::CounterVec;
use prometheus::GaugeVec;
use prometheus::IntCounterVec;

use reqwest::Client;

use serde_json::Value;

use std::num::Wrapping;
use std::time::Duration;

use tokio::time::sleep;

macro_rules! update_counter {
    ( $metric:ident, $labels:expr, $value:ident, $conversion:ident ) => {
        if let Some(value) = $value.$conversion() {
            let increment = Wrapping(value) - Wrapping($metric.with_label_values($labels).get());

            $metric.with_label_values($labels).inc_by(increment.0);
        }
    };
}

macro_rules! set_counter {
    ( $metric:ident, $labels:expr, $source:ident, $field:literal, $conversion:ident ) => {
        if let Some(value) = $source.get($field) {
            update_counter!($metric, $labels, value, $conversion);
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
    // adaptive
    static ref ADAPTIVE_GAIN: GaugeVec = register_gauge_vec!(
        "adsb_stats_adaptive_gain_dB",
        "Current adaptive gain setting",
        &["frequency"],
    )
    .unwrap();
    static ref ADAPTIVE_GAIN_LIMIT: GaugeVec = register_gauge_vec!(
        "adsb_stats_adaptive_gain_dynamic_range_limit_dB",
        "Current dynamic range gain upper limit",
        &["frequency"],
    )
    .unwrap();
    static ref ADAPTIVE_NOISE_FLOOR: GaugeVec = register_gauge_vec!(
        "adsb_stats_adaptive_gain_noise_floor_dBFS",
        "Current dynamic range noise floor",
        &["frequency"],
    )
    .unwrap();
    static ref ADAPTIVE_GAIN_CHANGES: IntCounterVec = register_int_counter_vec!(
        "adsb_stats_adaptive_gain_changes_total",
        "Number of dynamic gain changes",
        &["frequency"],
    )
    .unwrap();
    static ref ADAPTIVE_UNDECODED: IntCounterVec = register_int_counter_vec!(
        "adsb_stats_adaptive_loud_undecoded_total",
        "Number of loud undecoded bursts",
        &["frequency"],
    )
    .unwrap();
    static ref ADAPTIVE_DECODED: IntCounterVec = register_int_counter_vec!(
        "adsb_stats_adaptive_loud_decoded_total",
        "Number of loud decoded messages",
        &["frequency"],
    )
    .unwrap();
    static ref ADAPTIVE_GAIN_SECONDS: IntCounterVec = register_int_counter_vec!(
        "adsb_stats_adaptive_gain_seconds_total",
        "Number of seconds spent in a dB gain level",
        &["frequency", "gain_dB"],
    )
    .unwrap();

    // CPR
    static ref CPR_SURFACE: IntCounterVec = register_int_counter_vec!(
        "adsb_stats_cpr_surface_total",
        "Number of surface CPR messages received",
        &["frequency"],
    )
    .unwrap();
    static ref CPR_AIRBORNE: IntCounterVec = register_int_counter_vec!(
        "adsb_stats_cpr_airborne_total",
        "Number of airborne CPR messages received",
        &["frequency"],
    )
    .unwrap();
    static ref CPR_GLOBAL_OK: IntCounterVec = register_int_counter_vec!(
        "adsb_stats_cpr_global_ok_total",
        "Number of global positions derived",
        &["frequency"],
    )
    .unwrap();
    static ref CPR_GLOBAL_BAD: IntCounterVec = register_int_counter_vec!(
        "adsb_stats_cpr_global_bad_total",
        "Number of global positions rejected for inconsistency",
        &["frequency"],
    )
    .unwrap();
    static ref CPR_GLOBAL_RANGE: IntCounterVec = register_int_counter_vec!(
        "adsb_stats_cpr_global_bad_range_total",
        "Number of global bad positions exceeding the receiver maximum range",
        &["frequency"],
    )
    .unwrap();
    static ref CPR_GLOBAL_SPEED: IntCounterVec = register_int_counter_vec!(
        "adsb_stats_cpr_global_bad_speed_total",
        "Number of global bad positions exceeding inter-position speed checks",
        &["frequency"],
    )
    .unwrap();
    static ref CPR_GLOBAL_SKIPPED: IntCounterVec = register_int_counter_vec!(
        "adsb_stats_cpr_global_skipped_total",
        "Number of global position attempts skipped due to missing data",
        &["frequency"],
    )
    .unwrap();
    static ref CPR_LOCAL_OK: IntCounterVec = register_int_counter_vec!(
        "adsb_stats_cpr_local_ok_total",
        "Number of local (relative) positions found",
        &["frequency"],
    )
    .unwrap();
    static ref CPR_LOCAL_AIRCRAFT: IntCounterVec = register_int_counter_vec!(
        "adsb_stats_cpr_local_aircraft_relative_total",
        "Number of local positions relative to a previous aircraft position",
        &["frequency"],
    )
    .unwrap();
    static ref CPR_LOCAL_RECEIVER: IntCounterVec = register_int_counter_vec!(
        "adsb_stats_cpr_local_receiver_relative_total",
        "Number of local positions relative to the receiver position",
        &["frequency"],
    )
    .unwrap();
    static ref CPR_LOCAL_RANGE: IntCounterVec = register_int_counter_vec!(
        "adsb_stats_cpr_local_bad_range_total",
        "Number of local bad positions exceeding the receiver maximum range, or with an ambiguous range",
        &["frequency"],
    )
    .unwrap();
    static ref CPR_LOCAL_SPEED: IntCounterVec = register_int_counter_vec!(
        "adsb_stats_cpr_local_bad_speed_total",
        "Number of local bad positions exceeding inter-position speed checks",
        &["frequency"],
    )
    .unwrap();
    static ref CPR_LOCAL_SKIPPED: IntCounterVec = register_int_counter_vec!(
        "adsb_stats_cpr_local_skipped_total",
        "Number of local position attempts skipped due to missing data",
        &["frequency"],
    )
    .unwrap();
    static ref CPR_FILTERED: IntCounterVec = register_int_counter_vec!(
        "adsb_stats_cpr_filtered_total",
        "Number of CPR messages ignored for matching faulty transponder heuristics",
        &["frequency"],
    )
    .unwrap();

    // cpu
    static ref CPU_DEMOD: CounterVec = register_counter_vec!(
        "adsb_stats_cpu_demodulation_seconds_total",
        "Number CPU seconds spent demodulation and decoding SDR data",
        &["frequency"],
    )
    .unwrap();
    static ref CPU_READER: CounterVec = register_counter_vec!(
        "adsb_stats_cpu_reader_seconds_total",
        "Number CPU seconds spent reading SDR sample data",
        &["frequency"],
    )
    .unwrap();
    static ref CPU_BACKGROUND: CounterVec = register_counter_vec!(
        "adsb_stats_cpu_background_seconds_total",
        "Number CPU seconds spent on network IO and periodic tasks",
        &["frequency"],
    )
    .unwrap();

    // local
    static ref LOCAL_SAMPLES_PROCESSED: IntCounterVec = register_int_counter_vec!(
        "adsb_stats_local_samples_processed_total",
        "Number of local samples processed",
        &["frequency"],
    )
    .unwrap();
    static ref LOCAL_SAMPLES_DROPPED: IntCounterVec = register_int_counter_vec!(
        "adsb_stats_local_samples_dropped_total",
        "Number of local samples dropped before processing, a nonzero value means CPU overload",
        &["frequency"],
    )
    .unwrap();
    static ref LOCAL_MODEAC: IntCounterVec = register_int_counter_vec!(
        "adsb_stats_local_modeac_decoded_total",
        "Number of local mode A/C messages decoded",
        &["frequency"],
    )
    .unwrap();
    static ref LOCAL_MODES: IntCounterVec = register_int_counter_vec!(
        "adsb_stats_local_modes_preambles_total",
        "Number of local mode S preambles received",
        &["frequency"],
    )
    .unwrap();
    static ref LOCAL_MODES_BAD: IntCounterVec = register_int_counter_vec!(
        "adsb_stats_local_modes_bad_total",
        "Number of local mode S preambles that didn't result in a valid message",
        &["frequency"],
    )
    .unwrap();
    static ref LOCAL_UNKNOWN_ICAO: IntCounterVec = register_int_counter_vec!(
        "adsb_stats_local_modes_unknown_icao_total",
        "Number of local mode S preambles with an unknown ICAO address",
        &["frequency"],
    )
    .unwrap();
    static ref LOCAL_ACCEPTED: IntCounterVec = register_int_counter_vec!(
        "adsb_stats_local_modes_accepted_total",
        "Number of local valid mode S messages labeled with N-bit error corrections",
        &["frequency", "corrections"],
    )
    .unwrap();
    static ref LOCAL_SIGNAL: GaugeVec = register_gauge_vec!(
        "adsb_stats_local_signal_dbfs",
        "Mean signal power of local received messages in dBFS",
        &["frequency"],
    )
    .unwrap();
    static ref LOCAL_SIGNAL_PEAK: GaugeVec = register_gauge_vec!(
        "adsb_stats_local_signal_dbfs_peak",
        "Peak signal power of local received messages in dBFS",
        &["frequency"],
    )
    .unwrap();
    static ref LOCAL_NOISE: GaugeVec = register_gauge_vec!(
        "adsb_stats_local_noise_dbfs",
        "Mean signal noise of local received messages in dBFS",
        &["frequency"],
    )
    .unwrap();
    static ref LOCAL_STRONG_SIGNALS: IntCounterVec = register_int_counter_vec!(
        "adsb_stats_local_strong_signals_total",
        "Number of local messages received with a signal power above -3dBFS",
        &["frequency", "corrections"],
    )
    .unwrap();

    // messages
    static ref MESSAGES: IntCounterVec = register_int_counter_vec!(
        "adsb_stats_messages_total",
        "Number of messages received from any source",
        &["frequency"],
    )
    .unwrap();
    static ref MESSAGES_BY_DF: IntCounterVec = register_int_counter_vec!(
        "adsb_stats_messages_by_df_total",
        "Number of messages received per downlink format",
        &["frequency", "downlink_format"],
    )
    .unwrap();

    // remote
    static ref REMOTE_MODEAC: IntCounterVec = register_int_counter_vec!(
        "adsb_stats_remote_modeac_decoded_total",
        "Number of remote mode A/C messages decoded",
        &["frequency"],
    )
    .unwrap();
    static ref REMOTE_MODES: IntCounterVec = register_int_counter_vec!(
        "adsb_stats_remote_modes_preambles_total",
        "Number of remote mode S preambles received",
        &["frequency"],
    )
    .unwrap();
    static ref REMOTE_MODES_BAD: IntCounterVec = register_int_counter_vec!(
        "adsb_stats_remote_modes_bad_total",
        "Number of remote mode S preambles that didn't result in a valid message",
        &["frequency"],
    )
    .unwrap();
    static ref REMOTE_UNKNOWN_ICAO: IntCounterVec = register_int_counter_vec!(
        "adsb_stats_remote_modes_unknown_icao_total",
        "Number of remote mode S preambles with an unknown ICAO address",
        &["frequency"],
    )
    .unwrap();
    static ref REMOTE_ACCEPTED: IntCounterVec = register_int_counter_vec!(
        "adsb_stats_remote_modes_accepted_total",
        "Number of valid remote mode S messages labeled by N-bit error corrections",
        &["frequency", "corrections"],
    )
    .unwrap();

    // tracks
    static ref TRACKS_ALL: IntCounterVec = register_int_counter_vec!(
        "adsb_stats_tracks_total",
        "Number of unique aircraft tracks",
        &["frequency"],
    )
    .unwrap();
    static ref TRACKS_SINGLE: IntCounterVec = register_int_counter_vec!(
        "adsb_stats_tracks_single_message_total",
        "Number of single message aircraft tracks",
        &["frequency"],
    )
    .unwrap();
    static ref TRACKS_UNRELIABLE: IntCounterVec = register_int_counter_vec!(
        "adsb_stats_tracks_unreliable_total",
        "Number of unreliable tracks marked unreliable",
        &["frequency"],
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

        // .total
        set_counter!(MESSAGES, &[&self.frequency], total, "messages", as_u64);

        if let Some(messages_by_df) = total.get("messages_by_df") {
            if let Some(messages_by_df) = messages_by_df.as_array() {
                messages_by_df
                    .iter()
                    .enumerate()
                    .for_each(|(format, count)| {
                        update_counter!(
                            MESSAGES_BY_DF,
                            &[&self.frequency, &format.to_string()],
                            count,
                            as_u64
                        );
                    });
            }
        }

        // .total.adaptive
        let adaptive = total
            .get("adaptive")
            .context("Missing adaptive data in \"total\" object")?;

        set_counter!(
            ADAPTIVE_DECODED,
            &[&self.frequency],
            adaptive,
            "loud_decoded",
            as_u64
        );
        set_counter!(
            ADAPTIVE_GAIN_CHANGES,
            &[&self.frequency],
            adaptive,
            "gain_changes",
            as_u64
        );
        set_counter!(
            ADAPTIVE_UNDECODED,
            &[&self.frequency],
            adaptive,
            "loud_undecoded",
            as_u64
        );

        set_gauge!(
            ADAPTIVE_GAIN,
            &[&self.frequency],
            adaptive,
            "gain_db",
            as_f64
        );
        set_gauge!(
            ADAPTIVE_GAIN_LIMIT,
            &[&self.frequency],
            adaptive,
            "dynamic_range_limit_db",
            as_f64
        );
        set_gauge!(
            ADAPTIVE_NOISE_FLOOR,
            &[&self.frequency],
            adaptive,
            "noise_dbfs",
            as_f64
        );

        if let Some(gain_seconds) = adaptive.get("gain_seconds") {
            if let Some(gain_seconds) = gain_seconds.as_array() {
                gain_seconds.iter().for_each(|pair| {
                    if let Some(pair) = pair.as_array() {
                        if let Some(gain) = pair[0].as_f64() {
                            let seconds = &pair[1];

                            update_counter!(
                                ADAPTIVE_GAIN_SECONDS,
                                &[&self.frequency, &gain.to_string()],
                                seconds,
                                as_u64
                            );
                        }
                    }
                });
            }
        }

        // .total.cpr
        let cpr = total
            .get("cpr")
            .context("Missing cpr data in \"total\" object")?;

        set_counter!(CPR_AIRBORNE, &[&self.frequency], cpr, "airborne", as_u64);
        set_counter!(CPR_FILTERED, &[&self.frequency], cpr, "filtered", as_u64);
        set_counter!(
            CPR_GLOBAL_BAD,
            &[&self.frequency],
            cpr,
            "global_bad",
            as_u64
        );
        set_counter!(CPR_GLOBAL_OK, &[&self.frequency], cpr, "global_ok", as_u64);
        set_counter!(
            CPR_GLOBAL_RANGE,
            &[&self.frequency],
            cpr,
            "global_range",
            as_u64
        );
        set_counter!(
            CPR_GLOBAL_SKIPPED,
            &[&self.frequency],
            cpr,
            "global_bad",
            as_u64
        );
        set_counter!(
            CPR_GLOBAL_SPEED,
            &[&self.frequency],
            cpr,
            "global_speed",
            as_u64
        );
        set_counter!(
            CPR_LOCAL_AIRCRAFT,
            &[&self.frequency],
            cpr,
            "local_aircraft_relative",
            as_u64
        );
        set_counter!(CPR_LOCAL_OK, &[&self.frequency], cpr, "local_ok", as_u64);
        set_counter!(
            CPR_LOCAL_RANGE,
            &[&self.frequency],
            cpr,
            "local_range",
            as_u64
        );
        set_counter!(
            CPR_LOCAL_RECEIVER,
            &[&self.frequency],
            cpr,
            "local_receiver_relative",
            as_u64
        );
        set_counter!(
            CPR_LOCAL_SKIPPED,
            &[&self.frequency],
            cpr,
            "local_skipped",
            as_u64
        );
        set_counter!(
            CPR_LOCAL_SPEED,
            &[&self.frequency],
            cpr,
            "local_speed",
            as_u64
        );
        set_counter!(CPR_SURFACE, &[&self.frequency], cpr, "surface", as_u64);

        // .total.cpu
        let cpu = total
            .get("cpu")
            .context("Missing cpu data in \"total\" object")?;

        if let Some(value) = cpu.get("demod") {
            if let Some(value) = value.as_f64() {
                let value = value / 1000.0; // convert to seconds

                let increment = value - CPU_DEMOD.with_label_values(&[&self.frequency]).get();

                CPU_DEMOD
                    .with_label_values(&[&self.frequency])
                    .inc_by(increment);
            }
        }

        if let Some(value) = cpu.get("reader") {
            if let Some(value) = value.as_f64() {
                let value = value / 1000.0; // convert to seconds

                let increment = value - CPU_READER.with_label_values(&[&self.frequency]).get();

                CPU_READER
                    .with_label_values(&[&self.frequency])
                    .inc_by(increment);
            }
        }

        if let Some(value) = cpu.get("background") {
            if let Some(value) = value.as_f64() {
                let value = value / 1000.0; // convert to seconds

                let increment = value - CPU_BACKGROUND.with_label_values(&[&self.frequency]).get();

                CPU_BACKGROUND
                    .with_label_values(&[&self.frequency])
                    .inc_by(increment);
            }
        }

        // .total.local
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

        if let Some(accepted) = local.get("accepted") {
            if let Some(accepted) = accepted.as_array() {
                accepted
                    .iter()
                    .enumerate()
                    .for_each(|(corrections, count)| {
                        update_counter!(
                            LOCAL_ACCEPTED,
                            &[&self.frequency, &corrections.to_string()],
                            count,
                            as_u64
                        );
                    });
            }
        }

        // .total.remote
        let remote = total
            .get("remote")
            .context("Missing remote data in \"total\" object")?;

        set_counter!(REMOTE_MODEAC, &[&self.frequency], remote, "modeac", as_u64);
        set_counter!(REMOTE_MODES, &[&self.frequency], remote, "modes", as_u64);
        set_counter!(REMOTE_MODES_BAD, &[&self.frequency], remote, "bad", as_u64);
        set_counter!(
            REMOTE_UNKNOWN_ICAO,
            &[&self.frequency],
            remote,
            "unknown_icao",
            as_u64
        );

        if let Some(accepted) = remote.get("accepted") {
            if let Some(accepted) = accepted.as_array() {
                accepted
                    .iter()
                    .enumerate()
                    .for_each(|(corrections, count)| {
                        update_counter!(
                            REMOTE_ACCEPTED,
                            &[&self.frequency, &corrections.to_string()],
                            count,
                            as_u64
                        );
                    });
            }
        }

        // .total.tracks
        let tracks = total
            .get("tracks")
            .context("Missing tracks data in \"total\" object")?;

        set_counter!(TRACKS_ALL, &[&self.frequency], tracks, "all", as_u64);
        set_counter!(
            TRACKS_SINGLE,
            &[&self.frequency],
            tracks,
            "single_message",
            as_u64
        );
        set_counter!(
            TRACKS_UNRELIABLE,
            &[&self.frequency],
            tracks,
            "unreliable",
            as_u64
        );

        //
        let last1min = data.get("last1min").context("missing last1min data")?;

        let local = last1min
            .get("local")
            .context("Missing local data in \"last1min\" object")?;

        set_gauge!(LOCAL_SIGNAL, &[&self.frequency], local, "signal", as_f64);
        set_gauge!(
            LOCAL_SIGNAL_PEAK,
            &[&self.frequency],
            local,
            "peak_signal",
            as_f64
        );
        set_gauge!(LOCAL_NOISE, &[&self.frequency], local, "noise", as_f64);

        Ok(())
    }
}
