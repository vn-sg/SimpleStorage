use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cw_storage_plus::{Item, Map};

use crate::msg::ExecuteMsg;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub role: String,
    pub channel_ids: Vec<String>,
    pub current_tx_id: u32,
    // pub variables: HashMap<String, String>
    // pub x: String,
    // pub y: String
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Tx {
    pub msg: ExecuteMsg,
    pub no_of_votes: u32,
}

pub const STATE: Item<State> = Item::new("state");

pub const VARS: Map<&str, String> = Map::new("vars");
pub const TXS: Map<u32, Tx> = Map::new("txs");
// const PEOPLE: Map<&str, Data> = Map::new("people");


