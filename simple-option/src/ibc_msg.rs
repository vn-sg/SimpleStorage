use cosmwasm_std::{ContractResult};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Messages that will be sent over the IBC channel
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PacketMsg {
    MsgQueue {
        q: Vec<PacketMsg>
    },
    Propose { 
        chain_id: u32,
        k: u32, 
        v: String,
        view: u32 
    },
    Echo {
        // chain_id: u32,
        val: String,
        view: u32
    },
    Key1 {
        val: String,
        view: u32
    },
    Key2 {
        val: String,
        view: u32
    },
    Key3 {
        val: String,
        view: u32
    },
    Lock {
        val: String,
        view: u32
    },
    Done {
        val: String
    },
    WhoAmI { 
        chain_id: u32,
        
    },
    // TimeoutMsg{
    //     time_view: u32
    // },
    Request { 
        view: u32, 
        chain_id: u32 
    },
    Suggest { 
        chain_id: u32,
        view: u32,
        key2: u32,
        key2_val: String,
        prev_key2: i32,
        key3: u32,
        key3_val: String
    },
    Proof {
        key1: u32,
        key1_val: String,
        prev_key1: i32,
        view: u32
    }
}
// #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
// #[serde(rename_all = "snake_case")]
// pub struct MsgQueue {
//     q: Vec<PacketMsg>
// }

/// All IBC acknowledgements are wrapped in `ContractResult`.
/// The success value depends on the PacketMsg variant.
pub type AcknowledgementMsg<T> = ContractResult<T>;

/// This is the success response we send on ack for PacketMsg::Dispatch.
/// Just acknowledge success or error
// pub type DispatchResponse = ();

/// This is the success response we send on ack for PacketMsg::WhoAmI.
/// Return the caller's account address on the remote chain
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct WhoAmIResponse {
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ProposeResponse { 
    // pub tx_id: u32
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CommitResponse {
    pub tx_id: u32
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct RequestResponse {
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct SuggestResponse {
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ProofResponse {
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct EchoResponse {
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Key1Response {
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Key2Response {
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Key3Response {
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct LockResponse {
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct DoneResponse {
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MsgQueueResponse {
}
