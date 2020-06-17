use holochain_serialized_bytes::prelude::*;

#[derive(PartialEq, Clone, Deserialize, Serialize, SerializedBytes, Debug)]
pub struct SomeStruct {
    inner: String,
}

impl SomeStruct {
    pub fn new(inner: String) -> Self {
        Self { inner }
    }

    pub fn process(&mut self) {
        self.inner = format!("processed: {}", self.inner);
    }
}

#[derive(Serialize, Deserialize, SerializedBytes, Clone)]
pub struct StringType(String);

impl From<String> for StringType {
    fn from(s: String) -> StringType {
        StringType(s)
    }
}

impl From<StringType> for String {
    fn from(s: StringType) -> String {
        s.0.clone()
    }
}

#[derive(Debug, Serialize, Deserialize, SerializedBytes, Clone)]
pub struct BytesType(#[serde(with = "serde_bytes")] Vec<u8>);

impl From<Vec<u8>> for BytesType {
    fn from(b: Vec<u8>) -> Self {
        Self(b)
    }
}

impl BytesType {
    pub fn inner(&self) -> Vec<u8> {
        self.0.clone()
    }
}

#[derive(Serialize, Deserialize, SerializedBytes)]
pub struct IntegerType(u32);

impl From<u32> for IntegerType {
    fn from(u: u32) -> Self {
        Self(u)
    }
}

impl From<IntegerType> for u32 {
    fn from(u: IntegerType) -> Self {
        u.0
    }
}
