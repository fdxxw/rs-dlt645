#![feature(test)]
extern crate test;
pub fn add(left: usize, right: usize) -> usize {
    left + right
}

mod frame;
mod packager;
mod transporter;
mod rs485;
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
