use std::{collections::HashSet, fmt};

use cosmwasm_std::{Timestamp, to_binary, Binary, Addr};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize, ser::SerializeStruct};

use crate::{ibc_msg::Msg, state::{State, InputType}};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub chain_id: u32,
    pub input: InputType,
    pub contract_addr: String,
    // pub msg: ContractExecuteMsg
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Input { value: InputType },
    PreInput { value: InputType},
    ContractCall { value: InputType, contract: Addr },
    Abort {},
    Trigger { behavior: String }
}


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ContractExecuteMsg {
    Register { name: String },
    Transfer { name: String, to: String },
}

impl fmt::Display for ContractExecuteMsg {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ContractExecuteMsg::Register { name } => write!(f, "r,{}", name),
            ContractExecuteMsg::Transfer { name, to } => write!(f, "t,{},{}", name, to),
        }
        
    }
}
impl ContractExecuteMsg {
    pub(crate) fn generate(name: String) -> Self {
        ContractExecuteMsg::Register { name }
    }
//     pub(crate) fn as_bytes(self) -> &'static [u8] {
//         &to_binary(&self).unwrap()
//     }
}

// pub enum Trigger {
//     MultiPropose {},
//     SendMsgToAll ( String )
// }

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// GetValue querys value for given key, GetState returns the current state, GetTx returns tx with tx_id
    GetState { },
    GetStateProgress { },
    GetChannels { },
    GetTest { },
    GetHighestReq { },
    GetHighestAbort { },
    GetReceivedSuggest { },
    GetSendAllUpon { },
    GetTestQueue { },
    GetEcho { },
    GetKey1 { },
    GetKey2 { },
    GetKey3 { },
    GetLock { },
    GetDone { },
    GetAbortInfo { },
    GetDebug { },
    GetIbcDebug {},
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
        decided_val: InputType
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
    pub received_suggest: HashSet<u32>
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct SendAllUponResponse {
    pub send_all_upon: Vec<(u32, Vec<Msg>)>
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TestQueueResponse {
    pub test_queue: Vec<(u32, Vec<(u32, Vec<Msg>)>)>
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct EchoQueryResponse { 
    pub echo: Vec<(String, HashSet<u32>)>
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Key1QueryResponse { 
    pub key1: Vec<(String, HashSet<u32>)>
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Key2QueryResponse { 
    pub key2: Vec<(String, HashSet<u32>)>
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Key3QueryResponse { 
    pub key3: Vec<(String, HashSet<u32>)>
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct LockQueryResponse { 
    pub lock: Vec<(String, HashSet<u32>)>
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct DoneQueryResponse { 
    pub done: Vec<(String, HashSet<u32>)>
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AbortResponse { 
    pub start_time: Timestamp,
    pub end_time: Timestamp,
    pub current_time: Timestamp,
    pub is_timeout: bool,
    pub done: bool,
    pub should_abort: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct DebugResponse { 
    pub debug: Vec<(u32, String)>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct HighestAbortResponse {
    pub highest_abort: Vec<(u32, i32)>
}


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Register {
    pub(crate) name: String,
}

pub struct temp {
    pub value: String,
}


pub struct MyStruct {
    pub value: String,
}

impl Serialize for MyStruct {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
            let mut state = serializer.serialize_struct("Color", 3)?;
            state.serialize_field("r", "testR")?;
            state.serialize_field("g", "testG")?;
            state.serialize_field("b", "testB")?;
            state.end()
    }
}

pub struct RawSerializedBytes {
    pub value : Vec<u8>
}

impl Serialize for RawSerializedBytes {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
        todo!()
    }
}