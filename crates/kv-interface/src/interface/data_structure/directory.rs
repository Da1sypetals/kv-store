use bytes::Bytes;

use super::key_type::KeyType;

#[derive(Debug)]
pub struct Directory {
    pub(crate) dirs: Vec<String>,
}

impl Directory {
    pub fn encode_wrapped(&self) -> Bytes {
        KeyType::Data.wrap_key(&self.encode())
    }

    pub fn level(&self) -> usize {
        self.dirs.len()
    }

    pub(crate) fn encode(&self) -> Vec<u8> {
        let mut res: Vec<u8> = vec![KeyType::Data.into()];
        res.extend_from_slice(
            self.dirs
                .iter()
                .map(|dir| dir.to_owned() + ".")
                .collect::<Vec<_>>()
                .join("")
                .as_bytes(),
        );
        res
    }

    pub(crate) fn decode(bin: &Bytes) -> Self {
        if bin[0] != KeyType::Data.into() {
            panic!(
                "Internal error: invalid keytype id: expected 0, found {}",
                bin[0]
            );
        }
        let content = &bin[1..];
        let dirs = String::from_utf8_lossy(content)
            .split('.')
            .filter_map(|x| {
                if x.len() == 0 {
                    None
                } else {
                    Some(x.to_string())
                }
            })
            .collect();

        Self { dirs }
    }
}

impl ToString for Directory {
    fn to_string(&self) -> String {
        self.dirs
            .iter()
            .map(|dir| format!("{}.", dir))
            .collect::<Vec<_>>()
            .join("")
    }
}
