use cosmwasm_std::{
    StdResult, DepsMut, Order, IbcTimeout, Env, IbcOrder, StdError, IbcChannelOpenMsg, Storage,
};

use crate::ibc_msg::{
    PacketMsg
};

use crate::state::{
    CHANNELS, SEND_ALL_UPON, STATE, HIGHEST_REQ
};

/// Setting the lifetime of packets to be one hour
pub const PACKET_LIFETIME: u64 = 60 * 60;
/// Setting up constant
pub const F: u32 = 1;
pub const IBC_APP_VERSION: &str = "simple_storage";


use crate::ContractError;

pub fn get_timeout(env: Env) -> IbcTimeout {
    env.block.time.plus_seconds(PACKET_LIFETIME).into()
}


pub fn get_id_channel_pair(deps: &DepsMut) -> StdResult<Vec<(u32, String)>> {
    let channels: StdResult<Vec<_>> = CHANNELS
        .range(deps.storage, None, None, Order::Ascending)
        .collect();
    channels
}

pub fn get_id_channel_pair_from_storage(storage: &mut dyn Storage) -> StdResult<Vec<(u32, String)>> {
    let channels: StdResult<Vec<_>> = CHANNELS
        .range(storage, None, None, Order::Ascending)
        .collect();
    channels
}

pub fn send_all_upon_join_queue(storage: &mut dyn Storage, packet_to_broadcast: PacketMsg, 
                                queue: &mut Vec<Vec<PacketMsg>>) -> Result<(), ContractError> {
    let channel_ids = get_id_channel_pair_from_storage(storage)?;
    let state = STATE.load(storage)?;
    for (chain_id, _channel_id) in &channel_ids {
        let highest_request = HIGHEST_REQ.load(storage, chain_id.clone())?;
        if highest_request == state.view {
            queue[*chain_id as usize].push(packet_to_broadcast.clone());
        }
        else{
            let action = |packets: Option<Vec<PacketMsg>>| -> StdResult<Vec<PacketMsg>> {
                match packets {
                    Some(_) => todo!(),
                    None => Ok(vec!(packet_to_broadcast.clone())),
                }
                
            };
            SEND_ALL_UPON.update(storage, *chain_id, action)?;
        }
    }
    Ok(())
}



fn _verify_channel(msg: IbcChannelOpenMsg) -> StdResult<()> {
    let channel = msg.channel();

    if channel.order != IbcOrder::Ordered {
        return Err(StdError::generic_err("Only supports ordered channels"));
    }
    if channel.version.as_str() != IBC_APP_VERSION {
        return Err(StdError::generic_err(format!(
            "Must set version to `{}`",
            IBC_APP_VERSION
        )));
    }
    if let Some(counter_version) = msg.counterparty_version() {
        if counter_version != IBC_APP_VERSION {
            return Err(StdError::generic_err(format!(
                "Counterparty version must be `{}`",
                IBC_APP_VERSION
            )));
        }
    }

    Ok(())
}
