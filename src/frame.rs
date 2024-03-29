use std::{io::Cursor, vec};

use bytes::{Buf, Bytes};

use crate::error::Error;

#[derive(Clone, Debug)]
pub enum Frame {}

#[derive(Debug)]
pub enum FrameError {
    Incomplete,
    Other(Error),
}

impl Into<Error> for FrameError {
    fn into(self) -> Error {
        match self {
            Self::Incomplete => "Incomplete".into(),
            Self::Other(e) => e,
        }
    }
}

impl Frame {
    pub fn check(src: &mut Cursor<&[u8]>) -> Result<(), FrameError> {
        // 跳过 0xfe
        loop {
            let v = get_u8(src)?;
            if v != 0xfe {
                // 0x68
                if v == 0x68 {
                    break;
                } else {
                    return Err(FrameError::Other(
                        format!("protocol error; invalid frame type byte `{}`", v).into(),
                    ));
                }
            }
        }
        // 地址 6 个字节
        skip(src, 6)?;
        // 0x68
        get_u8_expect(src, 0x68)?;
        // 控制码 C
        skip(src, 1)?;
        // 数据长度
        let len = get_u8(src)?;
        // 数据域
        skip(src, len as usize)?;
        //
        skip(src, 2)?;
        Ok(())
    }

    pub fn parse(src: &mut Cursor<&[u8]>) -> Result<ProtocolDataUnit, FrameError> {
        let mut b = Vec::new();
        // 跳过 0xfe
        loop {
            let v = get_u8(src)?;
            if v != 0xfe {
                // 0x68
                if v == 0x68 {
                    b.push(0x68);
                    break;
                } else {
                    return Err(FrameError::Other(
                        format!("protocol error; invalid frame type byte `{}`", v).into(),
                    ));
                }
            }
            b.push(0xfe);
        }
        // 地址
        b.extend_from_slice(get_u8_of(src, 6)?);
        // 0x68
        get_u8_expect(src, 0x68)?;
        b.push(0x68);

        // C
        b.push(get_u8(src)?);
        // data len
        let len = get_u8(src)?;
        b.push(len);

        b.extend_from_slice(get_u8_of(src, len as usize)?);

        b.extend_from_slice(get_u8_of(src, 2)?);
        match ProtocolDataUnit::try_from(b) {
            Ok(f) => return Ok(f),
            Err(e) => return Err(FrameError::Other(e.into())),
        }
    }
}

fn get_u8(src: &mut Cursor<&[u8]>) -> Result<u8, FrameError> {
    if !src.has_remaining() {
        return Err(FrameError::Incomplete);
    }

    Ok(src.get_u8())
}
fn get_u8_expect(src: &mut Cursor<&[u8]>, expect: u8) -> Result<(), FrameError> {
    let u8 = get_u8(src)?;
    if u8 != expect {
        return Err(FrameError::Other(
            format!("protocol error; invalid frame type byte `{}`", u8).into(),
        ));
    }
    Ok(())
}
fn skip(src: &mut Cursor<&[u8]>, n: usize) -> Result<(), FrameError> {
    if src.remaining() < n {
        return Err(FrameError::Incomplete);
    }

    src.advance(n);
    Ok(())
}
fn get_u8_of<'a>(src: &mut Cursor<&'a [u8]>, n: usize) -> Result<&'a [u8], FrameError> {
    // Scan the bytes directly
    let start = src.position() as usize;
    // Scan to the second to last byte
    let end = start + n;
    if src.remaining() < n {
        return Err(FrameError::Incomplete);
    }
    src.set_position(end as u64);
    Ok(&src.get_ref()[start..end])
}

#[derive(Clone, Debug)]
pub struct ProtocolDataUnit {
    front: Vec<u8>,   // 在主站发送帧信息之前，先发送1—4个字节FEH，以唤醒接收方。
    start: u8,        // 标识一帧信息的开始，其值为68H=01101000B。
    address: Vec<u8>, // 地址域 地址域由 6 个字节构成，每字节 2 位 BCD 码 地址域传输时低字节在前，高字节在后。
    c: u8,            // 控制码 C
    l: u8,            // 数据域长度  L为数据域的字节数。读数据时L≤200，写数据时L≤50，L=0表示无数据域
    data: Vec<u8>, // 数据域 数据域包括数据标识、密码、操作者代码、数据、帧序号等，其结构随控制码的功能而改变。传输时发送方按字节进行加33H处理，接收方按字节进行减33H处理。
    cs: u8, // 校验码 从第一个帧起始符开始到校验码之前的所有各字节的模256的和，即各字节二进制算术和，不计超过256的溢出值
    end: u8, // 标识一帧信息的结束，其值为16H=00010110B。
}
#[derive(Debug)]
pub enum TryFromError {
    INVALID,
}
impl Into<Error> for TryFromError {
    fn into(self) -> Error {
        match self {
            Self::INVALID => "invalid".into(),
        }
    }
}
impl ProtocolDataUnit {
    pub fn new() -> Self {
        ProtocolDataUnit {
            front: vec![0xfe, 0xfe, 0xfe, 0xfe],
            start: 0x68,
            address: vec![],
            c: 0,
            l: 0,
            data: vec![],
            cs: 0,
            end: 0x16,
        }
    }
    /**
     *
     */
    pub fn from_cmd(addr: &str, c: &str, data: &Vec<&str>) -> Result<Self, Error> {
        let mut pdu = Self::default();
        let mut address = hex::decode(addr)?;
        address.reverse();
        pdu.address = address;
        match Bytes::from(hex::decode(c)?).get(0) {
            Some(&c) => pdu.c = c,
            None => return Err("c is invalid".into()),
        }
        let data = data
            .iter()
            .map(|t| hex::decode(t))
            .map(|t| {
                t.map(|q| {
                    q.iter()
                        .map(|v| (*v as u32 + 0x33 as u32) as u8)
                        .rev()
                        .collect::<Vec<u8>>()
                })
            })
            .try_fold(Vec::new(), |mut b, v| match v {
                Ok(mut v) => {
                    b.append(&mut v);
                    Ok(b)
                }
                Err(e) => Err(e),
            })?;
        pdu.data = data;
        pdu.l = pdu.data.len() as u8;
        Ok(pdu)
    }
    pub fn from_cmd_2(addr: Vec<u8>, c: u8, data: &Vec<Vec<u8>>) -> Result<Self, Error> {
        let mut pdu = Self::default();
        let mut addr = addr;
        addr.reverse();
        pdu.address = addr;
        pdu.c = c;
        let data = data
            .iter()
            .map(|t| {
                t.iter()
                    .map(|v| (*v as u32 + 0x33 as u32) as u8)
                    .rev()
                    .collect::<Vec<u8>>()
            })
            .fold(Vec::new(), |mut b, mut v| {
                b.append(&mut v);
                b
            });
        pdu.data = data;
        pdu.l = pdu.data.len() as u8;
        Ok(pdu)
    }
    pub fn compute_cs(data: &Vec<u8>) -> u8 {
        let r = data.iter().map(|t| *t as u32).sum::<u32>() % 256;
        r as u8
    }
    pub fn read_addr() -> Result<Self, Error> {
        Self::from_cmd_2(vec![0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA], 0x13, &vec![])
    }
    pub fn set_addr() -> Result<Self, Error> {
        Self::from_cmd_2(
            vec![0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA],
            0x15,
            &vec![vec![0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA]],
        )
    }
    pub fn address(&self) -> Vec<u8> {
        self.address.clone()
    }
    pub fn address_str(&self) -> String {
        hex::encode(&self.address)
    }
    pub fn address_real_str(&self) -> String {
        let mut addr = self.address.clone();
        addr.reverse();
        hex::encode(addr)
    }
    pub fn data(&self) -> Vec<u8> {
        self.data.clone()
    }
    pub fn c(&self) -> u8 {
        self.c
    }
}
impl Default for ProtocolDataUnit {
    fn default() -> Self {
        Self::new()
    }
}

impl Into<Vec<u8>> for ProtocolDataUnit {
    fn into(mut self) -> Vec<u8> {
        let mut v = vec![];
        self.l = self.data.len() as u8;
        v.push(self.start);
        v.append(&mut self.address);
        v.push(self.start);
        v.push(self.c);
        v.push(self.l);
        v.append(&mut self.data);
        self.cs = Self::compute_cs(&v);
        v.push(self.cs);
        v.push(self.end);
        let mut final_v = vec![];
        final_v.append(&mut self.front);
        final_v.append(&mut v);
        final_v
    }
}
impl Into<String> for ProtocolDataUnit {
    fn into(self) -> String {
        let v: Vec<u8> = self.into();
        hex::encode(v)
    }
}

impl TryFrom<&str> for ProtocolDataUnit {
    type Error = TryFromError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let v = value.replace(" ", "");
        match hex::decode(v) {
            Ok(v) => ProtocolDataUnit::try_from(v),
            Err(_) => Err(TryFromError::INVALID),
        }
    }
}

impl TryFrom<Vec<u8>> for ProtocolDataUnit {
    type Error = TryFromError;
    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        let mut pdu = ProtocolDataUnit::default();
        // 帧起始符
        pdu.front.clear();
        for b in value.iter() {
            if *b == 0xfe_u8 {
                pdu.front.push(*b);
            } else {
                break;
            }
        }
        let mut cursor = Cursor::new(value);
        cursor.advance(pdu.front.len());

        // 0x68
        if cursor.remaining() < 1 {
            return Err(TryFromError::INVALID);
        }
        cursor.advance(1);
        // 地址域
        if cursor.remaining() < 6 {
            return Err(TryFromError::INVALID);
        }
        let address = &cursor.chunk()[..6];
        pdu.address = Vec::from(address);
        cursor.advance(6);
        // 0x68
        if cursor.remaining() < 1 {
            return Err(TryFromError::INVALID);
        }
        cursor.advance(1);
        // 控制码 C
        if cursor.remaining() < 1 {
            return Err(TryFromError::INVALID);
        }
        pdu.c = cursor.get_u8();
        // 数据域长度
        if cursor.remaining() < 1 {
            return Err(TryFromError::INVALID);
        }
        pdu.l = cursor.get_u8();
        // 数据
        if cursor.remaining() < (pdu.l as usize) {
            return Err(TryFromError::INVALID);
        }
        pdu.data = Vec::from(&cursor.chunk()[..(pdu.l as usize)]);
        cursor.advance(pdu.l as usize);
        // 校验码
        if cursor.remaining() < 1 {
            return Err(TryFromError::INVALID);
        }
        pdu.cs = cursor.get_u8();
        // 结束符
        if cursor.remaining() < 1 {
            return Err(TryFromError::INVALID);
        }
        pdu.end = cursor.get_u8();
        Ok(pdu)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test::Bencher;
    #[test]
    fn from_cmd() {
        let pdu = ProtocolDataUnit::from_cmd("202208310002", "11", &vec!["028022FF"]);
        assert_eq!(pdu.is_ok(), true);
        assert_eq!(
            Into::<String>::into(pdu.unwrap()),
            "fefefefe680200310822206811043255b335d116".to_string()
        );
    }
    #[test]
    fn from_cmd_2() {
        let pdu = ProtocolDataUnit::from_cmd_2(
            vec![0x20, 0x22, 0x08, 0x31, 0x00, 0x02],
            0x11,
            &vec![vec![0x02, 0x80, 0x22, 0xff]],
        );
        assert_eq!(pdu.is_ok(), true);
        assert_eq!(
            Into::<String>::into(pdu.unwrap()),
            "fefefefe680200310822206811043255b335d116".to_string()
        );
    }
    #[test]
    fn try_from_string() {
        let pdu = ProtocolDataUnit::try_from("fe  fefefe680200310822206811043255b335d116");
        assert!(pdu.is_ok());
        assert_eq!(
            Into::<String>::into(pdu.unwrap()),
            "fefefefe680200310822206811043255b335d116".to_string()
        );
    }
    #[test]
    fn try_from_vec() {
        let pdu = ProtocolDataUnit::try_from(vec![
            0xfe, 0xfe, 0xfe, 0xfe, 0x68, 0x02, 0x00, 0x31, 0x08, 0x22, 0x20, 0x68, 0x11, 0x04,
            0x32, 0x55, 0xb3, 0x35, 0xd1, 0x16,
        ]);
        assert!(pdu.is_ok());
        assert_eq!(
            Into::<String>::into(pdu.unwrap()),
            "fefefefe680200310822206811043255b335d116".to_string()
        );
    }
    #[bench]
    fn from_cmd_bench(b: &mut Bencher) {
        b.iter(|| {
            let pdu = ProtocolDataUnit::from_cmd("202208310002", "11", &vec!["028022FF"]).unwrap();
            Into::<String>::into(pdu)
        });
    }

    #[bench]
    fn from_cmd_2_bench(b: &mut Bencher) {
        b.iter(|| {
            let pdu = ProtocolDataUnit::from_cmd_2(
                vec![0x20, 0x22, 0x08, 0x31, 0x00, 0x02],
                0x11,
                &vec![vec![0x02, 0x80, 0x22, 0xff]],
            )
            .unwrap();
            Into::<String>::into(pdu)
        });
    }
    #[bench]
    fn try_from_string_bench(b: &mut Bencher) {
        b.iter(|| ProtocolDataUnit::try_from("fefefefe680200310822206811043255b335d116"));
    }
    #[bench]
    fn try_from_vec_bench(b: &mut Bencher) {
        b.iter(|| {
            ProtocolDataUnit::try_from(vec![
                0xfe, 0xfe, 0xfe, 0xfe, 0x68, 0x02, 0x00, 0x31, 0x08, 0x22, 0x20, 0x68, 0x11, 0x04,
                0x32, 0x55, 0xb3, 0x35, 0xd1, 0x16,
            ])
        });
    }
}
