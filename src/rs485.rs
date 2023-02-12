use crate::transporter::Transporter;

pub struct RS485Transporter {

}

impl Transporter for RS485Transporter {
  fn send(adu: &Vec<u8>) -> Result<Vec<u8>, crate::frame::Error> {
      
  }
  fn open() -> Result<(), crate::frame::Error> {
      
  }
  fn close() -> Result<(), crate::frame::Error> {
      
  }
}