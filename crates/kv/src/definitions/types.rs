use bytes::Bytes;

pub type ByteVec = Vec<u8>;
pub type KvSizeType = u32;

#[derive(Debug)]
pub struct KvBytes {
    pub key: Bytes,
    pub value: Bytes,
}
