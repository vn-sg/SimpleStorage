use cosmwasm_std::IbcMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cw_storage_plus::{Item, Map};

use crate::{msg::ExecuteMsg, ibc_msg::{PacketMsg, Msg}};


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    // pub role: String,
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
    pub is_first_propose: bool,
    pub is_first_req_ack: bool,
    pub sent_suggest: bool,
    pub sent_done: bool,
    pub done: Option<String>
}

impl State {

    // Another associated function, taking two arguments:
    pub(crate) fn new(chain_id: u32, input: String) -> State {
        State {
            n: 1,
            chain_id: chain_id,
            channel_ids: Vec::new(),
            current_tx_id: 0,
            view: 0,
            cur_view: 0,
            primary: 1,
            key1: 0,
            key2: 0,
            key3: 0,
            lock: 0,
            key1_val: input.clone(),
            key2_val: input.clone(),
            key3_val: input.clone(),
            lock_val: input.clone(),
            prev_key1: -1,
            prev_key2: -1,
            suggestions: Vec::new(),
            key2_proofs: Vec::new(),
            proofs: Vec::new(),
            is_first_propose: true,
            is_first_req_ack: true,
            sent_suggest: false,
            done: None,
            sent_done: false,
        }
    }
}

// #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
// pub struct State {
//     pub role: String,
//     pub n: u32,
//     pub chain_id: u32,
//     pub channel_ids: Vec<String>,
//     pub current_tx_id: u32,
//     pub view: u32,
//     pub cur_view: u32,
//     pub primary: u32,
//     pub key1: u32,
//     pub key2: u32,
//     pub key3: u32,
//     pub lock: u32,
//     pub key1_val: String,
//     pub key2_val: String,
//     pub key3_val: String,
//     pub lock_val: String,

//     pub prev_key1: i32,
//     pub prev_key2: i32,

//     pub suggestions: Vec<(u32, String)>,
//     pub key2_proofs: Vec<(u32, String, i32)>,
//     pub proofs: Vec<(u32, String, i32)>,
//     pub is_first_propose: bool,
//     pub is_first_req_ack: bool,
//     pub sent_suggest: bool
// }
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

// pub const VARS: Map<&str, String> = Map::new("vars");
pub const TXS: Map<u32, Tx> = Map::new("txs");
pub const CHANNELS: Map<u32, String> = Map::new("channels");

pub const HIGHEST_REQ: Map<u32, u32> = Map::new("highest_req");
pub const HIGHEST_ABORT: Map<u32, i32> = Map::new("highest_abort");
pub const RECEIVED_SUGGEST: Map<u32, bool> = Map::new("received_suggest");
pub const RECEIVED_PROOF: Map<u32, bool> = Map::new("received_proof");
// pub const RECEIVED_PROPOSE: Map<u32, bool> = Map::new("received_propose");

pub const TEST: Map<u32, Vec<IbcMsg>> = Map::new("test");
pub const TEST_QUEUE: Map<u32, (u32, Vec<Msg>)> = Map::new("test_queue");
pub const UPON_QUEUE: Map<String, Vec<PacketMsg>> = Map::new("upon_queue");
pub const SEND_ALL_UPON: Map<u32, Vec<Msg>> = Map::new("send_all_upon");

// Message types to indicate the amount of which received on the same val
// MessageType: Map<VAL, COUNT>
pub const ECHO: Map<String, u32> = Map::new("echo");
pub const KEY1: Map<String, u32> = Map::new("key1");
pub const KEY2: Map<String, u32> = Map::new("key2");
pub const KEY3: Map<String, u32> = Map::new("key3");
pub const LOCK: Map<String, u32> = Map::new("lock");
pub const DONE: Map<String, u32> = Map::new("done");


