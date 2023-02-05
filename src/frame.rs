use bytes::Bytes;

pub type Error = Box<dyn std::error::Error + Send + Sync>;

#[derive(Clone, Debug)]
pub enum Frame {
    Error(String),
    Content(ProtocolDataUnit),
}

#[derive(Clone, Debug)]
pub struct ProtocolDataUnit {
    front: Bytes,   // 在主站发送帧信息之前，先发送1—4个字节FEH，以唤醒接收方。
    start: u8,      // 标识一帧信息的开始，其值为68H=01101000B。
    address: Bytes, // 地址域 地址域由 6 个字节构成，每字节 2 位 BCD 码 地址域传输时低字节在前，高字节在后。
    c: u8,          // 控制码 C
    l: u8,          // 数据域长度  L为数据域的字节数。读数据时L≤200，写数据时L≤50，L=0表示无数据域
    data: Bytes, // 数据域 数据域包括数据标识、密码、操作者代码、数据、帧序号等，其结构随控制码的功能而改变。传输时发送方按字节进行加33H处理，接收方按字节进行减33H处理。
    cs: u8, // 校验码 从第一个帧起始符开始到校验码之前的所有各字节的模256的和，即各字节二进制算术和，不计超过256的溢出值
    end: u8, // 标识一帧信息的结束，其值为16H=00010110B。
}
impl ProtocolDataUnit {
    pub fn new() -> Self {
        ProtocolDataUnit {
            front: Bytes::from_static(&[0xfe, 0xfe, 0xfe, 0xfe]),
            start: 0x68,
            address: Bytes::default(),
            c: 0,
            l: 0,
            data: Bytes::default(),
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
        pdu.address = Bytes::from(address);
        match Bytes::from(hex::decode(c)?).get(0) {
            Some(&c) => pdu.c = c,
            None => return Err("c is invalid".into()),
        }
        let data = data
            .iter()
            .map(|t| hex::decode(t))
            .map(|t| t.map(|q| q.iter().map(|v| v + 0x33).rev().collect::<Vec<u8>>()))
            .try_fold(Vec::new(), |mut b, v| match v {
                Ok(mut v) => {
                    b.append(&mut v);
                    Ok(b)
                }
                Err(e) => Err(e),
            })?;
        pdu.data = Bytes::from(data);
        pdu.compute_cs();
        Ok(pdu)
    }
    pub fn compute_cs(&mut self) {
        let r = self.data.iter().map(|t| *t as u32).sum::<u32>() % 256;
        self.l = r as u8;
    }
}
impl Default for ProtocolDataUnit {
    fn default() -> Self {
        Self::new()
    }
}
