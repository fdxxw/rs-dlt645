use crate::frame::Error;

pub trait Transporter {
    fn send(adu: &Vec<u8>) -> Result<Vec<u8>, Error>;
    fn open() -> Result<(), Error>;
    fn close() -> Result<(), Error>;
}