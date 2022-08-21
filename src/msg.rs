use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use crate::state::ChannelInfo;
use cosmwasm_std::{ContractResult, Binary, to_binary};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub count: i32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Set { value: String },
    Commit {key: u32},
    SelfCallVote {key: u32, value: String, channel_id: String},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    // GetCount returns the current count as a json-encoded number
    GetUncomittedValue {key: u32},
    GetComittedValue {key : u32},
    ListChannels(),
    GetLatestId {},
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct IdResponse {
    pub id: u32,
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct UncomittedValueResponse {
    pub value: Option<String>,
    pub error: Option<String>,
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ComittedValueResponse {
    pub value: Option<String>,
    pub error: Option<String>,
}


// ----------------------- IBC messages -------------------------------


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SimpleStorageIbcPacket {
    IbcProposeRequest {key: u32, value: String},
    IbcVoteRequest {key: u32, value: String},
    IbcCommitRequest {key: u32},
    TestRequest {},
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug, Default)]
pub struct ListChannelsResponse {
    pub channels: Vec<ChannelInfo>
}

/// All acknowledgements are wrapped in `ContractResult`.
/// The success value depends on the PacketMsg variant.
/// #[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub type AcknowledgementMsg<T> = ContractResult<T>;


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SimpleStorageAckPacket {
    AckVoteRequest {key: u32, value: String},
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum SimpleStorageAck {
    Result(Binary),
    Error(String),
}

pub fn ack_success() -> Binary {
    let res = SimpleStorageAck::Result(b"1".into());
    to_binary(&res).unwrap()
}

pub fn ack_fail(err: String) -> Binary {
    let res = SimpleStorageAck::Error(err);
    to_binary(&res).unwrap()
}
