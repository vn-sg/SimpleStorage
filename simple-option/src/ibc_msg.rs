use cosmwasm_std::{ContractResult};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::msg::ExecuteMsg;

/// Messages that will be sent over the IBC channel
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PacketMsg {
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
    Commit { 
        msg: ExecuteMsg, 
        tx_id: u32
    },
    WhoAmI { chain_id: u32 },
    ClientRequestBroadcast {
        value: String,
    },
    ViewRequest { 
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
pub struct ClientRequestBroadcastResponse {

}