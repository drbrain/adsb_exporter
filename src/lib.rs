mod adsb_exporter;
mod aircraft_json;
pub mod beast;
mod configuration;
mod dump_watcher;
mod fetch;
mod receiver_json;
mod stats_json;

pub use crate::adsb_exporter::ADSBExporter;
pub use crate::configuration::Configuration;
pub use crate::dump_watcher::DumpWatcher;

#[track_caller]
pub(crate) fn spawn_named<T>(
    task: impl std::future::Future<Output = T> + Send + 'static,
    _name: &str,
) -> tokio::task::JoinHandle<T>
where
    T: Send + 'static,
{
    #[cfg(tokio_unstable)]
    return tokio::task::Builder::new().name(_name).spawn(task);

    #[cfg(not(tokio_unstable))]
    tokio::spawn(task)
}
