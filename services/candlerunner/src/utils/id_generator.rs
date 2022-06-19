#[derive(Default)]
pub struct IdGenerator {
    bytes: Vec<u8>,
}

impl IdGenerator {
    pub fn add<K: AsRef<str>, V: AsRef<[u8]>>(&mut self, key: K, value: V) {
        self.bytes.extend_from_slice(key.as_ref().as_bytes());
        self.bytes.push(b':');
        self.bytes.extend_from_slice(value.as_ref());
        self.bytes.push(0);
    }

    pub fn add_opt<K: AsRef<str>, V: AsRef<[u8]>>(&mut self, key: K, value: Option<V>) {
        match value {
            Some(bytes) => self.add(key, bytes),
            None => self.add(key, b"<None>"),
        }
    }

    pub fn generate(self, ns: &uuid::Uuid) -> uuid::Uuid {
        uuid::Uuid::new_v5(ns, &self.bytes)
    }
}
