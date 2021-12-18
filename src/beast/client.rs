use anyhow::Context;
use anyhow::Result;

use crate::beast::codec::Codec;

use futures_util::StreamExt;

use log::debug;

use tokio::net::TcpStream;

use tokio_util::codec::Framed;

pub struct Client {
    address: String,
    reader: Framed<TcpStream, Codec>,
}

impl Client {
    pub async fn new(address: String) -> Result<Client> {
        let stream = TcpStream::connect(address.clone())
            .await
            .with_context(|| format!("Unable to connect to {}", address.clone()))?;

        let reader = Framed::new(stream, Codec::new());

        let client = Client { address, reader };

        Ok(client)
    }

    pub async fn run(&mut self) {
        while let Some(result) = self.reader.next().await {
            debug!("result: {:?}", result);
            panic!();
        }
    }
}

impl std::fmt::Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client")
            .field("address", &self.address)
            .finish()
    }
}
