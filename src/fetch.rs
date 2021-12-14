use lazy_static::lazy_static;

use log::debug;

use prometheus::register_histogram_vec;
use prometheus::register_int_counter_vec;
use prometheus::HistogramVec;
use prometheus::IntCounterVec;

use reqwest::Client;

use serde_json::Value;

lazy_static! {
    static ref REQUESTS: IntCounterVec = register_int_counter_vec!(
        "adsb_http_requests_total",
        "Number of HTTP requests made to fetch metrics",
        &["uri"],
    )
    .unwrap();
    static ref ERRORS: IntCounterVec = register_int_counter_vec!(
        "adsb_http_request_errors_total",
        "Number of HTTP request errors returned from fetching metrics",
        &["uri", "error_type"],
    )
    .unwrap();
    static ref DURATIONS: HistogramVec = register_histogram_vec!(
        "adsb_http_request_duration_seconds",
        "HTTP request durations",
        &["uri"],
    )
    .unwrap();
}

pub async fn fetch(client: &Client, url: &String) -> Option<Value> {
    debug!("Fetching {}", url);
    REQUESTS.with_label_values(&[&url]).inc();
    let timer = DURATIONS.with_label_values(&[&url]).start_timer();

    let response = client.get(url).send().await;

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
