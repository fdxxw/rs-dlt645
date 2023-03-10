use crate::error::Error;
use crate::frame::ProtocolDataUnit;
pub trait Packager {
    fn encode(pdu: &ProtocolDataUnit) -> Result<Vec<u8>, Error>;
    fn decode(adu: &Vec<u8>) -> Result<Vec<u8>, Error>;
}
