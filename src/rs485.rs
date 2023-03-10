use std::io::{Write, Read};
use std::time::Duration;

use async_trait::async_trait;
use bytes::BytesMut;
use tokio_serial::{self, SerialPortBuilder, SerialPortBuilderExt, SerialStream, SerialPort};

use crate::error::Error;
use crate::transporter::Transporter;
pub struct RS485Transporter {
    builder: SerialPortBuilder,
    stream: Option<SerialStream>
    buffer: BytesMut,
}

impl RS485Transporter {
    pub fn new(builder: SerialPortBuilder) -> Self {
        Self { builder, stream: None, buffer: BytesMut::with_capacity(256), }
    }
}

#[async_trait]
impl Transporter for RS485Transporter {
    async fn send(&mut self, adu: &[u8]) -> Result<Vec<u8>, Error> {
      if let Some(stream) = self.stream {
        stream.set_timeout(Duration::from_millis(500));
        stream.write(adu)?;
        loop {
          // if let Some(frame) = self.parse_frame()? {

          // }
          
          if 0 == stream.read(&mut self.buffer)? {
            if self.buffer.is_empty() {
              return Err("data is empty".into());
            } else {
                return Err("connection reset by peer".into());
            }
          }
        }        
      }
      Err("serial is not opend".into())
    }
    async fn open(&mut self) -> Result<(), Error> {
      let r = self.builder.open_native_async()?;
      self.stream = Some(r);
      Ok(())
    }
    async fn close(&mut self) -> Result<(), Error> {
      if let Some(stream) = self.stream {
        self.stream = None;
      }
      Ok(())
    }
}
