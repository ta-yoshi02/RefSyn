use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Operation {
    pub name: String,
}
