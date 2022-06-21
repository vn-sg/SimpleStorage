use cosmwasm_std::{
    entry_point, from_slice, to_binary, DepsMut, Env, StdError, StdResult, Event, Binary, Response, IbcTimeout,
};
use cosmwasm_std::{
    IbcBasicResponse, IbcChannelCloseMsg,
    IbcChannelConnectMsg, IbcChannelOpenMsg, IbcMsg, IbcOrder, IbcPacketAckMsg,
    IbcPacketReceiveMsg, IbcPacketTimeoutMsg, IbcReceiveResponse
};

use crate::ContractError;
use crate::ibc_msg::{
    AcknowledgementMsg, PacketMsg, WhoAmIResponse, CommitResponse, ProposeResponse,
};
use crate::msg::ExecuteMsg;
use crate::state::{STATE, State, VARS, Tx, TXS};

pub const IBC_APP_VERSION: &str = "simple_storage";


/// Setting the lifetime of packets to be one hour
pub const PACKET_LIFETIME: u64 = 60 * 60;

pub fn broadcast_response(
    timeout:IbcTimeout,
    channel_ids: Vec<String>, 
    packet_to_broadcast: PacketMsg,
    attrib: String
) -> Result<Response, ContractError> {
    // broadcast Propose message
    let mut msgs: Vec<IbcMsg> = Vec::new();
    for channel_id in channel_ids {
        let msg = IbcMsg::SendPacket {
            channel_id: channel_id.clone(),
            data: to_binary(&packet_to_broadcast)?,
            timeout: timeout.clone()
        };
        msgs.push(msg);
    }

    let res = Response::new()
        .add_messages(msgs)
        .add_attribute("action", attrib);
    Ok(res)
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

#[entry_point]
/// enforces ordering and versioing constraints
pub fn ibc_channel_open(_deps: DepsMut, _env: Env, _msg: IbcChannelOpenMsg) -> StdResult<()> {
    // verify_channel(msg)?;
    Ok(())
}

#[entry_point]
/// once it's established, we send a WhoAmI message
pub fn ibc_channel_connect(
    deps: DepsMut,
    env: Env,
    msg: IbcChannelConnectMsg,
) -> StdResult<IbcBasicResponse> {
    let channel = msg.channel();
    // Retrieve the connecting channel_id
    let channel_id = &channel.endpoint.channel_id;
    
    let action = | mut state: State | -> StdResult<_> {
        state.channel_ids.push(channel_id.to_string());
        Ok(state)
    };
    // Storing channel_id info to state 
    STATE.update(deps.storage, action)?;
    
    // retrieve current State
    // let mut state = STATE.load(deps.storage)?;
    // state.channel_ids.push(channel_id.to_string());
    // // store connected channel_id in local state
    // STATE.save(deps.storage, &state)?;
    // construct a packet to send
    let packet = PacketMsg::WhoAmI {};
    let msg = IbcMsg::SendPacket {
        channel_id: channel_id.clone(),
        data: to_binary(&packet)?,
        timeout: env.block.time.plus_seconds(PACKET_LIFETIME).into(),
    };

    Ok(IbcBasicResponse::new()
        .add_message(msg)
        .add_attribute("action", "ibc_connect")
        .add_attribute("channel_id", channel_id))
}

#[entry_point]
/// On closed channel, simply delete the channel_id local state
pub fn ibc_channel_close(
    deps: DepsMut,
    _env: Env,
    msg: IbcChannelCloseMsg,
) -> StdResult<IbcBasicResponse> {
    // fetch the connected channel_id
    let channel = msg.channel();
    let channel_id = &channel.endpoint.channel_id;

    let action = | mut state: State | -> StdResult<_> {
        state.channel_ids.retain(|e| !(e==channel_id));
        Ok(state)
    };
    STATE.update(deps.storage, action)?;

    // remove the channel
    // let mut state = STATE.load(deps.storage)?;
    // state.channel_ids.retain(|e| !(e==channel_id));
    // STATE.save(deps.storage, &state)?;
    // accounts(deps.storage).remove(channel_id.as_bytes());

    Ok(IbcBasicResponse::new()
        .add_attribute("action", "ibc_close")
        .add_attribute("channel_id", channel_id))
}

// This encode an error or error message into a proper acknowledgement to the recevier
fn encode_ibc_error(msg: impl Into<String>) -> Binary {
    // this cannot error, unwrap to keep the interface simple
    to_binary(&AcknowledgementMsg::<()>::Err(msg.into())).unwrap()
}

#[entry_point]
pub fn ibc_packet_receive(
    deps: DepsMut,
    _env: Env,
    msg: IbcPacketReceiveMsg,
) -> StdResult<IbcReceiveResponse> {
    (|| {
        let packet = msg.packet;
        // which local channel did this packet come on
        let caller = packet.dest.channel_id;
        let msg: PacketMsg = from_slice(&packet.data)?;
        match msg {
            PacketMsg::Propose{ msg, tx_id }=>receive_propose(deps,caller,msg,tx_id),
            PacketMsg::WhoAmI{}=>receive_who_am_i(deps,caller),
            PacketMsg::Commit{ msg, tx_id }=>receive_commit(deps, caller, msg, tx_id)
        }
    })()
    .or_else(|e| {
        // we try to capture all app-level errors and convert them into
        // acknowledgement packets that contain an error code.
        let acknowledgement = encode_ibc_error(format!("invalid packet: {}", e));
        Ok(IbcReceiveResponse::new()
            .set_ack(acknowledgement)
            .add_event(Event::new("ibc").add_attribute("packet", "receive")))
    })

    // Ok(IbcReceiveResponse::new()
    //     .set_ack(b"{}")
    //     .add_attribute("action", "ibc_packet_ack"))
}

pub fn receive_propose (
    _deps: DepsMut,
    _caller: String,
    _msg: ExecuteMsg,
    tx_id: u32
) -> StdResult<IbcReceiveResponse> {
    let _response = ProposeResponse { tx_id };
    // to_binary(&AcknowledgementMsg::<DispatchResponse>::Ok(()))?;

    // specify the type of AcknowledgementMsg to be ProposeResponse
    let acknowledgement = to_binary(&AcknowledgementMsg::Ok(ProposeResponse { tx_id: tx_id.clone() } ))?;
    // send back acknowledgement, containing the response info
    Ok(IbcReceiveResponse::new()
        .set_ack(acknowledgement)
        .add_attribute("action", "receive_propose"))
}
pub fn receive_commit (
    deps: DepsMut,
    _caller: String,
    msg: ExecuteMsg,
    tx_id: u32
) -> StdResult<IbcReceiveResponse> {
        match msg {
            ExecuteMsg::Set { key, value } => try_set(deps, key, value, tx_id),
            ExecuteMsg::Get { key } => try_get(deps, key, tx_id),
        }

}
pub fn try_get(
    deps: DepsMut, 
    key: String,
    tx_id: u32
) -> StdResult<IbcReceiveResponse>  {
    // let value = state.variables[&key].clone();
    let value = VARS.may_load(deps.storage, &key)?.unwrap(); 
    
    let response = CommitResponse { tx_id };
    let acknowledgement = to_binary(&AcknowledgementMsg::Ok(response))?;

    // send response along with the key and value of the variable
    Ok(IbcReceiveResponse::new()
        .set_ack(acknowledgement)
        .add_attribute("action", "commit")
        .add_attribute("msg", "Get")
        .add_attribute("key", key)
        .add_attribute("value", value ))

}

pub fn try_set( 
    deps: DepsMut, 
    key: String, 
    value: String,
    tx_id: u32
) -> StdResult<IbcReceiveResponse> {
    // let mut state = STATE.load(deps.storage)?;
    let action = |_| -> StdResult<String> {
        Ok(value.clone())
    };
    VARS.update(deps.storage, &key, action)?;
    // STATE.save(deps.storage, &state)?;

    let response = CommitResponse { tx_id: tx_id.clone() };
    let acknowledgement = to_binary(&AcknowledgementMsg::Ok(response))?;

    Ok(IbcReceiveResponse::new()
        .set_ack(acknowledgement)
        .add_attribute("action", "commit")
        .add_attribute("msg", "Set")
        .add_attribute("tx_id", tx_id.to_string())
        .add_attribute("key", key)
        .add_attribute("value", value ))

}

// processes PacketMsg::WhoAmI
fn receive_who_am_i(_deps: DepsMut, _caller: String) -> StdResult<IbcReceiveResponse> {
    let response = WhoAmIResponse {};
    let acknowledgement = to_binary(&AcknowledgementMsg::Ok(response))?;
    // and we are golden
    Ok(IbcReceiveResponse::new()
        .set_ack(acknowledgement)
        .add_attribute("action", "receive_who_am_i"))
}

#[entry_point]
pub fn ibc_packet_ack(
    deps: DepsMut,
    env: Env,
    msg: IbcPacketAckMsg,
) -> StdResult<IbcBasicResponse> {

    let packet: PacketMsg = from_slice(&msg.original_packet.data)?;
    match packet {
        PacketMsg::Propose { tx_id: _, msg: _ } => {
            let res: AcknowledgementMsg<ProposeResponse> = from_slice(&msg.acknowledgement.data)?;
            acknowledge_propose(deps, env, res)
        },
        PacketMsg::Commit { msg: _, tx_id: _ } => Ok(IbcBasicResponse::new()),
        PacketMsg::WhoAmI {  } => Ok(IbcBasicResponse::new()),
}


}

fn acknowledge_propose(
    deps: DepsMut,
    env: Env,
    ack: AcknowledgementMsg<ProposeResponse>,
) -> StdResult<IbcBasicResponse> {
    // retrive tx_id from acknowledge message
    let ProposeResponse { tx_id } = match ack {
        AcknowledgementMsg::Ok(res) => res,
        AcknowledgementMsg::Err(e) => {
            return Ok(IbcBasicResponse::new()
                .add_attribute("action", "acknowledge_propose")
                .add_attribute("error", e))
        }
    };
    let action = 
    |tx: Option<Tx>| -> StdResult<Tx> {
        let mut tx = tx.unwrap();
        tx.no_of_votes += 1;
        Ok( tx )
    };

    let tx = TXS.update(deps.storage, tx_id.clone(), action)?; 
    // broadcast Commit message
    if tx.no_of_votes >= 2 {
        let state: State = STATE.load(deps.storage)?;
        let channel_ids = state.channel_ids.clone();
        let packet = PacketMsg::Commit { msg: tx.msg.clone(), tx_id: tx_id.clone() };
        let timeout: IbcTimeout= env.block.time.plus_seconds(PACKET_LIFETIME).into();

        receive_commit(deps, "self".to_string(), tx.msg.clone(), tx_id.clone())?;
        
        let msg0 = IbcMsg::SendPacket {
            channel_id: channel_ids[0].clone(),
            data: to_binary(&packet)?,
            timeout: timeout.clone()
        };
        let msg1 = IbcMsg::SendPacket {
            channel_id: channel_ids[1].clone(),
            data: to_binary(&packet)?,
            timeout: timeout.clone()
        };
        
        Ok(IbcBasicResponse::new()
        .add_message(msg0)
        .add_message(msg1)
        .add_attribute("action", "acknowledge_propose_response")
        .add_attribute("commit", "true"))
    }
    else {
        Ok(IbcBasicResponse::new()
        .add_attribute("action", "acknowledge_propose_response")
        .add_attribute("commit", "false"))
    }
            
    
}


#[entry_point]
/// This will never be called since 
pub fn ibc_packet_timeout(
    _deps: DepsMut,
    _env: Env,
    _msg: IbcPacketTimeoutMsg,
) -> StdResult<IbcBasicResponse> {
    Ok(IbcBasicResponse::new().add_attribute("action", "ibc_packet_timeout"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::{execute, instantiate, query};
    use crate::msg::{InstantiateMsg, QueryMsg, ExecuteMsg};

    use cosmwasm_std::testing::{
        mock_dependencies, mock_env, mock_ibc_channel_connect_ack, mock_ibc_channel_open_init,
        mock_ibc_channel_open_try, mock_ibc_packet_ack, mock_info, MockApi, MockQuerier,
        MockStorage,
    };
    use cosmwasm_std::{coins, CosmosMsg, OwnedDeps};

    fn setup() -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg { role: "leader".to_string() };
        let info = mock_info("creator_V", &coins(100, "BTC"));
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
        deps
    }

    // connect will run through the entire handshake to set up a proper connect and
    // save the account (tested in detail in `proper_handshake_flow`)
    fn connect(mut deps: DepsMut, channel_id: &str) {
        let handshake_open =
            mock_ibc_channel_open_init(channel_id, IbcOrder::Ordered, IBC_APP_VERSION);
        // first we try to open with a valid handshake
        ibc_channel_open(deps.branch(), mock_env(), handshake_open).unwrap();

        // then we connect (with counter-party version set)
        let handshake_connect =
            mock_ibc_channel_connect_ack(channel_id, IbcOrder::Ordered, IBC_APP_VERSION);
        let res = ibc_channel_connect(deps.branch(), mock_env(), handshake_connect).unwrap();

        // this should send a WhoAmI request, which is received some blocks later
        assert_eq!(1, res.messages.len());
        match &res.messages[0].msg {
            CosmosMsg::Ibc(IbcMsg::SendPacket {
                channel_id: packet_channel,
                ..
            }) => assert_eq!(packet_channel.as_str(), channel_id),
            o => panic!("Unexpected message: {:?}", o),
        };
    }

}
