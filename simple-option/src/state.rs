use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cw_storage_plus::{Item, Map};

use crate::msg::ExecuteMsg;

pub const NODE_COUNT: u32 = 4;
pub const FAILURE_COUNT: u32 = 1;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub role: String,
    pub n: u32,
    pub chain_id: u32,
    pub channel_ids: Vec<String>,
    pub current_tx_id: u32,
    pub view: u32,
    pub cur_view: u32,
    pub primary: u32,
    pub key1: u32,
    pub key2: u32,
    pub key3: u32,
    pub lock: u32,
    pub key1_val: String,
    pub key2_val: String,
    pub key3_val: String,
    pub lock_val: String,

    pub prev_key1: i32,
    pub prev_key2: i32,

    pub suggestions: Vec<(u32, String)>,
    pub key2_proofs: Vec<(u32, String, i32)>,
    pub proofs: Vec<(u32, String, i32)>,
    pub is_first_propose: bool
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Tx {
    pub msg: ExecuteMsg,
    pub no_of_votes: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Test {
    pub src_port: String,
    pub src_chan_id: String,
    pub dest_port: String,
    pub dest_chan_id: String,
}

pub const STATE: Item<State> = Item::new("state");

pub const VARS: Map<&str, String> = Map::new("vars");
pub const TXS: Map<u32, Tx> = Map::new("txs");
pub const CHANNELS: Map<u32, String> = Map::new("channels");
// pub const TEST: Map<u32, Test> = Map::new("test");
pub const HIGHEST_REQ: Map<u32, u32> = Map::new("highest_req");
pub const HIGHEST_ABORT: Map<u32, u32> = Map::new("highest_abort");
pub const RECEIVED_SUGGEST: Map<u32, bool> = Map::new("received_suggest");
pub const RECEIVED_PROOF: Map<u32, bool> = Map::new("received_proof");
// pub const RECEIVED_PROPOSE: Map<u32, bool> = Map::new("received_propose");


// <value, count>> can be changed to ID later on
pub const CLIENT_REQ_COUNT: Map<String, u32> = Map::new("request_id_map");

// Received from this channel yet? prevent bad guys from trigerring dual vote
pub const CLIENT_REQ_CHANNEL_REQ_DEDUPE: Map<String, bool> = Map::new("CLIENT_REQ_IS_BROADCASTED");
pub const CLIENT_REQ_SENT: Item<bool> = Item::new("CLIENT_REQ_SENT");


pub const DEBUG: Map<u32, String> = Map::new("debug");

