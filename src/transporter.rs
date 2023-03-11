use async_trait::async_trait;

use crate::{error::Error, frame::{Frame, ProtocolDataUnit}};

#[async_trait]
pub trait Transporter {
    async fn send(&mut self, adu: &[u8]) -> Result<Option<ProtocolDataUnit>, Error>;
    async fn open(&mut self) -> Result<(), Error>;
    async fn close(&mut self) -> Result<(), Error>;
}