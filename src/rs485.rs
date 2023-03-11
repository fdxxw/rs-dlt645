use std::io::{Cursor, Read, Write};
use std::time::Duration;

use crate::error::Error;
use crate::frame::{Frame, FrameError, ProtocolDataUnit};
use crate::transporter::Transporter;
use async_trait::async_trait;
use bytes::{Buf, BufMut, BytesMut};
use futures::stream::StreamExt;
use futures::SinkExt;
use tokio_serial::{self, SerialPort, SerialPortBuilder, SerialPortBuilderExt, SerialStream};
use tokio_util::codec::{Decoder, Encoder};

pub struct RS485Transporter {
    builder: SerialPortBuilder,
    stream: Option<SerialStream>,
}

struct RS485Codec;

impl RS485Transporter {
    pub fn new(builder: SerialPortBuilder) -> Self {
        Self {
            builder,
            stream: None,
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
            stream.set_timeout(Duration::from_millis(500))?;
            RS485Codec.framed(stream).send(adu).await?;
        } else {
            return Err("serial is not opend".into());
        }
        let mut reader = RS485Codec.framed(self.stream.as_mut().unwrap());
        while let Some(r) = reader.next().await {
            match r {
                Ok(pdu) => {
                    return Ok(Some(pdu));
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }
        Ok(None)
        // loop {
        //     if let Some(frame) = self.parse_frame()? {
        //         return Ok(Some(frame));
        //     }
        //     if 0 == self.stream.as_mut().unwrap().read(&mut self.buffer)? {
        //         if self.buffer.is_empty() {
        //             return Err("data is empty".into());
        //         } else {
        //             return Err("connection reset by peer".into());
        //         }
        //     }
        // }
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
impl RS485Transporter {
    // fn parse_frame(&mut self) -> Result<Option<ProtocolDataUnit>, Error> {
    //     let mut buf = Cursor::new(&self.buffer[..]);
    //     match Frame::check(&mut buf) {
    //         Ok(_) => {
    //             let len = buf.position() as usize;
    //             buf.set_position(0);
    //             match Frame::parse(&mut buf) {
    //                 Ok(frame) => {
    //                     self.buffer.advance(len);
    //                     Ok(Some(frame))
    //                 }
    //                 Err(e) => Err(e.into()),
    //             }
    //         }
    //         Err(FrameError::Incomplete) => Ok(None),
    //         Err(e) => Err(e.into()),
    //     }
    // }
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
            let start = Instant::now();

            let r = rs485.send(&adu).await;
            match r {
                Ok(frame) => {
                    println!("{:?}",  Into::<String>::into(frame.unwrap()));
                }
                Err(e) => {
                    eprintln!("Err：{}", e)
                }
            }
            println!("Elapsed time: {:?}", start.elapsed());
        })
    }
}
