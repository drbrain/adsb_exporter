use anyhow::Context;
use anyhow::Result;

use log::info;

use prometheus_hyper::Server;

use std::net::SocketAddr;
use std::sync::Arc;

use tokio::sync::mpsc;
use tokio::sync::Notify;

type ErrorSender = mpsc::Sender<anyhow::Error>;

pub struct ADSBExporter {
    bind_address: SocketAddr,
    shutdown: Arc<Notify>,
}

impl ADSBExporter {
    pub fn new(bind_address: SocketAddr) -> Result<Self> {
        let shutdown = Arc::new(Notify::new());

        let exporter = ADSBExporter {
            bind_address,
            shutdown,
        };

        Ok(exporter)
    }

    async fn run(&self, error_tx: ErrorSender) {
        info!("Starting server on {}", self.bind_address);

        let result = Server::run(
            Arc::new(prometheus::default_registry().clone()),
            self.bind_address,
            self.shutdown.notified(),
        )
        .await
        .with_context(|| format!("Failed to start server on {}", self.bind_address));

        if let Err(e) = result {
            error_tx
                .send(e)
                .await
                .expect("Error channel failed unexpectedly, bug?");
        }
    }

    pub async fn start(self, error_tx: ErrorSender) {
        crate::spawn_named(
            async move {
                self.run(error_tx).await;
            },
            "adsb_exporter",
        );
    }
}
