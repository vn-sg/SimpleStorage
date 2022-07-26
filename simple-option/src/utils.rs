use cosmwasm_std::{
    StdResult, DepsMut, Order, IbcTimeout, Env, IbcOrder, StdError, IbcChannelOpenMsg
};

use crate::state::{
    CHANNELS,
};

/// Setting the lifetime of packets to be one hour
pub const PACKET_LIFETIME: u64 = 60 * 60;
/// Setting up constant
pub const F: u32 = 1;
pub const IBC_APP_VERSION: &str = "simple_storage";


pub fn get_timeout(env: Env) -> IbcTimeout {
    env.block.time.plus_seconds(PACKET_LIFETIME).into()
}


pub fn get_id_channel_pair(deps: &DepsMut) -> StdResult<Vec<(u32, String)>> {
    let channels: StdResult<Vec<_>> = CHANNELS
        .range(deps.storage, None, None, Order::Ascending)
        .collect();
    channels
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
