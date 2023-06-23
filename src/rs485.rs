use std::io::{Cursor, Read, Write};
use std::time::Duration;

use crate::error::Error;
use crate::frame::{Frame, FrameError, ProtocolDataUnit};
use crate::transporter::Transporter;
use async_trait::async_trait;
use bytes::{Buf, BufMut, BytesMut};
use futures::stream::StreamExt;
use futures::SinkExt;
use tokio::time::timeout;
use tokio_serial::{self, SerialPort, SerialPortBuilder, SerialPortBuilderExt, SerialStream};
use tokio_util::codec::{Decoder, Encoder};

pub struct RS485Transporter {
    builder: SerialPortBuilder,
    stream: Option<SerialStream>,
    timeout: Duration,
}

pub struct RS485Codec;

impl RS485Transporter {
    pub fn new(builder: SerialPortBuilder) -> Self {
        Self {
            builder,
            stream: None,
            timeout: Duration::from_millis(1),
        }
    }
}

impl Encoder<&[u8]> for RS485Codec {
    type Error = Error;
    fn encode(&mut self, item: &[u8], dst: &mut BytesMut) -> Result<(), Self::Error> {
        dst.reserve(item.len());
        dst.put(item);
        Ok(())
    }
}
impl Decoder for RS485Codec {
    type Error = Error;
    type Item = ProtocolDataUnit;
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let mut buf = Cursor::new(&src[..]);
        match Frame::check(&mut buf) {
            Ok(_) => {
                let len = buf.position() as usize;
                buf.set_position(0);
                match Frame::parse(&mut buf) {
                    Ok(frame) => {
                        src.advance(len);
                        Ok(Some(frame))
                    }
                    Err(e) => Err(e.into()),
                }
            }
            Err(FrameError::Incomplete) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}

#[async_trait]
impl Transporter for RS485Transporter {
    async fn send(&mut self, adu: &[u8]) -> Result<Option<ProtocolDataUnit>, Error> {
        if let Some(stream) = &mut self.stream {
            stream.set_timeout(self.timeout)?;
            match timeout(self.timeout, RS485Codec.framed(stream).send(adu)).await {
                Ok(_) => {}
                Err(_) => return Err(format!("send timeout after {:?}", self.timeout).into()),
            }
        } else {
            return Err("serial is not opend".into());
        }
        let mut reader = RS485Codec.framed(self.stream.as_mut().unwrap());
        match timeout(self.timeout, reader.next()).await {
            Ok(r) => {
                if let Some(r) = r {
                    match r {
                        Ok(pdu) => {
                            return Ok(Some(pdu));
                        }
                        Err(e) => {
                            return Err(e);
                        }
                    }
                }
            }
            Err(_) => return Err(format!("read timeout after {:?}", self.timeout).into()),
        }
        Ok(None)
    }
    async fn open(&mut self) -> Result<(), Error> {
        let r = self.builder.clone().open_native_async()?;
        self.stream = Some(r);
        Ok(())
    }
    async fn close(&mut self) -> Result<(), Error> {
        if let Some(_) = &mut self.stream {
            self.stream = None;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use tokio_serial::{DataBits, Parity, StopBits};
    use tokio_test::block_on;

    use crate::frame::ProtocolDataUnit;

    use super::*;

    #[test]
    fn test() {
        block_on(async {
            let builder = tokio_serial::new("COM3", 2400)
                .data_bits(DataBits::Eight)
                .parity(Parity::Even)
                .stop_bits(StopBits::One);
            let mut rs485 = RS485Transporter::new(builder);
            rs485.open().await.unwrap();
            let adu: Vec<u8> =
                ProtocolDataUnit::try_from("fe fe fe fe 68 aa aa aa aa aa aa 68 13 00 df 16")
                    .unwrap()
                    .into();
            for _ in 1..=10 {
                let start = Instant::now();

                let r = rs485.send(&adu).await;
                match r {
                    Ok(frame) => {
                        println!("{:?}", Into::<String>::into(frame.unwrap()));
                    }
                    Err(e) => {
                        eprintln!("Err：{}", e)
                    }
                }
                println!("Elapsed time: {:?}", start.elapsed());
            }
        })
    }
}
