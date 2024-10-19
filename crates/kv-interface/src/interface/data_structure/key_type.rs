use bytes::Bytes;

pub enum KeyType {
    Metadata,
    Data,
}

impl KeyType {
    pub fn wrap_key(self, key: &[u8]) -> Bytes {
        let mut res: Vec<u8> = vec![self.into()];
        res.extend(key);
        res.into()
    }
}

impl From<u8> for KeyType {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Metadata,
            1 => Self::Data,
            _ => panic!("Internal error: unknown key type {}", value),
        }
    }
}

impl From<KeyType> for u8 {
    fn from(value: KeyType) -> Self {
        match value {
            KeyType::Metadata => 0,
            KeyType::Data => 1,
        }
    }
}
