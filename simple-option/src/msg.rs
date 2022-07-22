use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ibc_msg::PacketMsg;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub role: String,
    pub chain_id: u32,
    pub input: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Set { key: String, value: u32 },
    Get { key: String },
    Input { value: String }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// GetValue querys value for given key, GetState returns the current state, GetTx returns tx with tx_id
    GetValue { key: String },
    GetState { },
    GetTx { tx_id: String },
    GetChannels { },
    GetTest { },
    GetHighestReq { },
    GetReceivedSuggest { },
    GetSendAllUpon { },
    GetTestQueue { }
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum ValueResponse {
    KeyFound {
        key: String,
        value: String
    },
    KeyNotFound {
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ChannelsResponse {
    pub port_chan_pair: Vec<(u32,String)>
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct HighestReqResponse {
    pub highest_request: Vec<(u32, u32)>
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ReceivedSuggestResponse {
    pub received_suggest: Vec<(u32, bool)>
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct SendAllUponResponse {
    pub send_all_upon: Vec<(u32, Vec<PacketMsg>)>
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TestQueueResponse {
    pub test_queue: Vec<(u32, (u32, Vec<PacketMsg>))>
}