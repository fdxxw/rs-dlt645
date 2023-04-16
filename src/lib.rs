#![feature(test)]
extern crate test;
pub fn add(left: usize, right: usize) -> usize {
    left + right
}

mod error;
mod frame;
mod packager;
mod transporter;
mod rs485;
mod tcp;
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
