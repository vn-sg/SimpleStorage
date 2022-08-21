use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, IbcEndpoint};
use cw_storage_plus::{Item, Map};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub count: i32,
    pub owner: Addr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct StorageValue {
    pub value: String,
}

pub const COUNTER_STATE: Item<State> = Item::new("state");

pub const ID_COUNTER: Item<u32> = Item::new("id_counter");

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct ChannelInfo {
    /// id of this channel
    pub id: String,
    /// the remote channel/port we connect to
    pub counterparty_endpoint: IbcEndpoint,
    /// the connection this exists on (you can use to query client/consensus info)
    pub connection_id: String,
}

/// static info on one channel that doesn't change
pub const CHANNEL_INFOS: Map<&str, ChannelInfo> = Map::new("channel_info");

/// uncomitted values
pub const UNCOMITTED_VALUES: Map<u32, String> = Map::new("uncomitted_values");

/// comitted values
pub const COMITTED_VALUES: Map<u32, String> = Map::new("comitted_values");

/// uncomitted values
pub const VOTE_COUNTS: Map<u32, u32> = Map::new("vote_counts");

