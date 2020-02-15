extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate holochain_json_derive;

use holochain_json_api::{json::JsonString, error::JsonError};

#[derive(PartialEq, Clone, Deserialize, Serialize, Debug, DefaultJson)]
pub struct SomeStruct {
    inner: String,
}

impl SomeStruct {
    pub fn new(inner: String) -> Self {
        Self{ inner }
    }
}
