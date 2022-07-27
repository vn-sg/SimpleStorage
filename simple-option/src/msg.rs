use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{ibc_msg::PacketMsg, state::State};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub chain_id: u32,
    pub input: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Input { value: String },
    ForceAbort {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// GetValue querys value for given key, GetState returns the current state, GetTx returns tx with tx_id
    GetState { },
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
pub enum StateResponse {
    InProgress {
        state: State
    },
    Done {
        decided_val: String
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