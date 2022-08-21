use cosmwasm_std::{entry_point, StdError};
use cosmwasm_std::{from_binary, to_binary, from_slice, Binary, Deps, DepsMut, Env, MessageInfo, Response, SubMsgResult, WasmMsg, StdResult, SubMsg};

//IBC imports
use cosmwasm_std::{
    IbcBasicResponse, IbcChannelCloseMsg, IbcChannel, Order,
    IbcChannelConnectMsg, IbcChannelOpenMsg, IbcMsg, IbcOrder, IbcPacketAckMsg, 
    IbcPacketReceiveMsg, IbcPacketTimeoutMsg, IbcReceiveResponse, Event, Reply
};


use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{IdResponse, ExecuteMsg, InstantiateMsg, QueryMsg, UncomittedValueResponse, 
    ListChannelsResponse, SimpleStorageIbcPacket, AcknowledgementMsg, ComittedValueResponse, ack_success, ack_fail, SimpleStorageAck};
use crate::state::{State, ChannelInfo, COUNTER_STATE, CHANNEL_INFOS, UNCOMITTED_VALUES, COMITTED_VALUES, ID_COUNTER, VOTE_COUNTS};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:simplestorage";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const SELF_EXEC_ID: u64 = 5000;


#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let state = State {
        count: msg.count,
        owner: info.sender.clone(),
    };
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    COUNTER_STATE.save(deps.storage, &state)?;
    ID_COUNTER.save(deps.storage, &0)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender)
        .add_attribute("count", msg.count.to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Set{value} => try_set(deps,env,info,value),
        ExecuteMsg::Commit{key} => try_set(deps,env,info,"commit".to_string()),
        ExecuteMsg::SelfCallVote { key, value, channel_id } => handle_self_call(deps, env, info, key, value, channel_id),
     }
}

pub fn try_set(deps: DepsMut, env:Env, _info: MessageInfo, value: String) -> Result<Response, ContractError> {

    let id = ID_COUNTER.load(deps.storage)?;
    UNCOMITTED_VALUES.update(deps.storage, id,|mut state| -> Result<_, ContractError> {
        Ok(value.clone())
    })?;

    VOTE_COUNTS.update(deps.storage, id,|mut state| -> Result<_, ContractError> {
        Ok(1)
    })?;

    ID_COUNTER.update(deps.storage, |mut state| -> Result<_, ContractError> {
        Ok(state+1)
    })?;

    // Broadcast
    let mut ibc_proposals = Vec::new(); 
    let channels: Vec<ChannelInfo> = CHANNEL_INFOS
        .range_raw(deps.storage, None, None, Order::Ascending)
        .map(|r| r.map(|(_, v)| v))
        .collect::<StdResult<_>>()?;

    let data = SimpleStorageIbcPacket::IbcProposeRequest{key: id, value: value};
    for channel in channels {
        ibc_proposals.push(IbcMsg::SendPacket { channel_id: channel.id, data: to_binary(&data)?, timeout: env.block.time.plus_seconds(60 * 60).into() });
    };
    
    Ok(Response::new()
        .add_messages(ibc_proposals)
        .add_attribute("action", "handle_send_msgs")
    )

}

pub fn handle_self_call(deps: DepsMut, env:Env, _info: MessageInfo, key: u32, value: String, channel_id: String) -> Result<Response, ContractError> {
    UNCOMITTED_VALUES.update(deps.storage, 3000,|mut state| -> Result<_, ContractError> {
        Ok("SELF_CALL_EXEC_DXH".into())
    })?;

    let data = SimpleStorageIbcPacket::IbcVoteRequest{key: key, value: value};
    let ibc_msg = IbcMsg::SendPacket { channel_id: channel_id, data: to_binary(&data)?, timeout: env.block.time.plus_seconds(60).into()};
    Ok(Response::new()
        .add_message(ibc_msg)
        .add_attribute("action", "handle_send_msgs")
    )

}


#[entry_point]
pub fn reply(deps: DepsMut, _env: Env, reply: Reply) -> StdResult<Response> {
    match (reply.id, reply.result) {
        (SELF_EXEC_ID, SubMsgResult::Err(err)) => {
            UNCOMITTED_VALUES.update(deps.storage, 5000,|mut state| -> StdResult<_> {
                Ok("REPLY FAILURE".into())
            })?;        
            UNCOMITTED_VALUES.update(deps.storage, 5001,|mut state| -> StdResult<_> {
                Ok(err.into())
            })?;        

            Ok(Response::new())
        }    
        _ => {    
            Ok(Response::new())        
        }
    }
}


#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetUncomittedValue {key} => to_binary(&get_uncomitted_value(deps, key)?),
        QueryMsg::GetLatestId {} => to_binary(&get_latest_id(deps)?),
        QueryMsg::GetComittedValue {key} => to_binary(&get_comitted_value(deps, key)?),
        QueryMsg::ListChannels {} => to_binary(&list_channels(deps)?),        
    }
}

fn get_latest_id(deps: Deps) -> StdResult<IdResponse> {
    let state = ID_COUNTER.load(deps.storage)?;
    Ok(IdResponse { id: state })
}

fn get_uncomitted_value(deps: Deps, key: u32) -> StdResult<UncomittedValueResponse> {
    if UNCOMITTED_VALUES.has(deps.storage, key) {
        let val = UNCOMITTED_VALUES.load(deps.storage, key)?;
        Ok( UncomittedValueResponse{value: Some(val), error: None})
    } else {
        Ok( UncomittedValueResponse{value: None, error: Some("Key doesnt exist".to_string())})
    }
}


fn get_comitted_value(deps: Deps, key: u32) -> StdResult<ComittedValueResponse> {
    if COMITTED_VALUES.has(deps.storage, key) {
        let val = COMITTED_VALUES.load(deps.storage, key)?;
        Ok( ComittedValueResponse{value: Some(val), error: None})
    } else {
        Ok( ComittedValueResponse{value: None, error: Some("Key is not comitted".to_string())})
    }
}


fn list_channels(deps: Deps) -> StdResult<ListChannelsResponse> {
    let channels = CHANNEL_INFOS
        .range_raw(deps.storage, None, None, Order::Ascending)
        .map(|r| r.map(|(_, v)| v))
        .collect::<StdResult<_>>()?;

    Ok(ListChannelsResponse{channels})
}


// ------------------------------------------------------- TODO move to ibc.rs ------------------------------------------------------


#[entry_point]
/// enforces ordering and versioning constraints
pub fn ibc_channel_open(deps: DepsMut, env: Env, msg: IbcChannelOpenMsg) -> Result<(), ContractError> { 
    enforce_order_and_version(msg.channel(), msg.counterparty_version())?;
    Ok(())
}

#[entry_point]
/// Store channel information for send vs reply
pub fn ibc_channel_connect(deps: DepsMut, env: Env, msg: IbcChannelConnectMsg) -> Result<IbcBasicResponse, ContractError> {
    enforce_order_and_version(msg.channel(), msg.counterparty_version())?;

    let channel: IbcChannel = msg.into();
    let info = ChannelInfo {
        id: channel.endpoint.channel_id,
        counterparty_endpoint: channel.counterparty_endpoint,
        connection_id: channel.connection_id,
    };
    CHANNEL_INFOS.save(deps.storage, &info.id, &info)?;

    Ok(IbcBasicResponse::new())
}

#[entry_point]
/// On closed channel, simply delete the account from our local store
pub fn ibc_channel_close(deps: DepsMut,env: Env, msg: IbcChannelCloseMsg) -> Result<IbcBasicResponse, ContractError> {
    UNCOMITTED_VALUES.update(deps.storage, 20000,|mut state| -> Result<_, ContractError> {
        Ok("IBC CHANNEL CLOSED".to_string())
    })?;
    Ok(IbcBasicResponse::new())
}


#[entry_point]
/// never should be called as the other side never sends packets
pub fn ibc_packet_receive(
    deps: DepsMut,
    env: Env,
    msg: IbcPacketReceiveMsg,
) -> Result<IbcReceiveResponse, ContractError> {
    //(|| {
        let packet = msg.packet;
        let dest_channel_id = packet.dest.channel_id.to_string();
        let channel_id = packet.src.channel_id.to_string();

        UNCOMITTED_VALUES.update(deps.storage, 101,|mut state| -> StdResult<_> {
            Ok(channel_id)
        })?;

        let packet: SimpleStorageIbcPacket = from_binary(&packet.data)?;
        match packet {
            SimpleStorageIbcPacket::IbcProposeRequest { key, value } => handle_propose_request(deps, env, key, value, dest_channel_id.to_string()),
            SimpleStorageIbcPacket::IbcCommitRequest {key} => handle_commit_request(deps, key, dest_channel_id.to_string()),
            SimpleStorageIbcPacket::IbcVoteRequest {key, value} => handle_vote_ibc_packet(deps, env, key, value, dest_channel_id.to_string()),
            SimpleStorageIbcPacket::TestRequest { } => Ok(IbcReceiveResponse::new().set_ack(b"{}").add_attribute("action", "ibc_packet_ack"))
        }
    //})
    /* 
    ()
    .or_else( |e| {
        let acknowledgement = encode_ibc_error(format!("invalid packet: {}", e));


        Ok(IbcReceiveResponse::new()
            .set_ack(acknowledgement)
            .add_event(Event::new("ibc").add_attribute("packet", "receive")))
            .add_event(Event::new("CRASH RECEIVE PACKET").add_attribute()
    })
    */
}

pub fn handle_propose_request(deps: DepsMut, env: Env, key: u32, value: String, source_channel_id: String) -> Result<IbcReceiveResponse, ContractError> {
    
    UNCOMITTED_VALUES.update(deps.storage, key,|mut state| -> Result<_, ContractError> {
        Ok(value.clone())
    })?;

    let self_exec_msg = SubMsg::reply_on_error(WasmMsg::Execute { 
        contract_addr: env.contract.address.into(), 
        msg: to_binary(&ExecuteMsg::SelfCallVote { key: key, value: value, channel_id: source_channel_id })?, 
        funds: vec![],
    }, SELF_EXEC_ID);


    //let data = SimpleStorageIbcPacket::IbcVoteRequest{key: key, value: value};
    Ok(IbcReceiveResponse::new()
        .set_ack(ack_success())
        .add_submessage(self_exec_msg)
        //.add_message(IbcMsg::SendPacket { channel_id: source_channel_id, data: to_binary(&data)?, timeout: env.block.time.plus_seconds(60).into() })
        .add_attribute("action", "handle_propose_request"))
}


pub fn handle_vote_ibc_packet(deps: DepsMut, env: Env, key: u32, value: String, source_channel_id: String) -> Result<IbcReceiveResponse, ContractError> {
    //move to comitted
    COMITTED_VALUES.update(deps.storage, key,|mut state| -> StdResult<_> {
        Ok(value.clone())
    })?;    

    // Broadcast Cp,,ots
    let mut ibc_commits = Vec::new(); 
    let channels: Vec<ChannelInfo> = CHANNEL_INFOS
        .range_raw(deps.storage, None, None, Order::Ascending)
        .map(|r| r.map(|(_, v)| v))
        .collect::<StdResult<_>>()?;

    let data = SimpleStorageIbcPacket::IbcCommitRequest{key: key};
    for channel in channels {
        ibc_commits.push(IbcMsg::SendPacket { channel_id: channel.id, data: to_binary(&data)?, timeout: env.block.time.plus_seconds(60).into() });
    };
    

    UNCOMITTED_VALUES.update(deps.storage, 350,|mut state| -> StdResult<_> {
        Ok("SEND IBC COMMIT REQUESTS".to_string())
    })?;

    Ok(IbcReceiveResponse::new()
        .set_ack(ack_success())
        .add_messages(ibc_commits)
        .add_attribute("action", "handle_vote_ibc_packet"))
}


pub fn handle_commit_request(deps: DepsMut, key: u32, source_channel_id: String) -> Result<IbcReceiveResponse, ContractError> {

    UNCOMITTED_VALUES.update(deps.storage, 500,|mut state| -> Result<_, ContractError> {
        Ok("RECEIVED COMMIT PACKAGE DXH".to_string())
    })?;

    let value = UNCOMITTED_VALUES.load(deps.storage, key)?;
    COMITTED_VALUES.update(deps.storage, key,|mut state| -> Result<_, ContractError> {
        Ok(value.clone())
    })?;

    Ok(IbcReceiveResponse::new().set_ack(ack_success())
        .add_attribute("action", "ibc_packet_commit_ack"))
}

#[entry_point]
pub fn ibc_packet_ack(
    deps: DepsMut,
    env: Env,
    msg: IbcPacketAckMsg,
) -> StdResult<IbcBasicResponse> {

    let _kek = UNCOMITTED_VALUES.update(deps.storage, 200,|mut state| -> Result<_, ContractError> {
        Ok("ACKNOWLEDGED IBC_PACKET DXH".to_string())
    });

    Ok(IbcBasicResponse::new())
    /* 
    // which local channel was this packet send from
    let source_channel_id = msg.original_packet.src.channel_id;
    // we need to parse the original packet and assume that it was a positive vote... No need to parse the actual acknowledgement as we are not doing any
    // error handling
    let packet: SimpleStorageIbcPacket = from_slice(&msg.original_packet.data)?;
    match packet {
        SimpleStorageIbcPacket::IbcProposeRequest { key, value } => Ok(IbcBasicResponse::new()),
        SimpleStorageIbcPacket::IbcCommitRequest {key: _} => Ok(IbcBasicResponse::new()), //Do Nothing
        SimpleStorageIbcPacket::IbcVoteRequest {key: _, value: _} => Ok(IbcBasicResponse::new()), //Do Nothing
        SimpleStorageIbcPacket::TestRequest {} => Ok(IbcBasicResponse::new()),
    }
    */
}

pub fn handle_vote_acknowledgement(deps: DepsMut, env: Env, key: u32, value: String, source_channel_id: String) -> StdResult<IbcBasicResponse> {
    //move to comitted
    COMITTED_VALUES.update(deps.storage, key,|mut state| -> StdResult<_> {
        Ok(value.clone())
    })?;    

    // Broadcast Cp,,ots
    let mut ibc_commits = Vec::new(); 
    let channels: Vec<ChannelInfo> = CHANNEL_INFOS
        .range_raw(deps.storage, None, None, Order::Ascending)
        .map(|r| r.map(|(_, v)| v))
        .collect::<StdResult<_>>()?;

    let data = SimpleStorageIbcPacket::IbcCommitRequest{key: key};
    for channel in channels {
        ibc_commits.push(IbcMsg::SendPacket { channel_id: channel.id, data: to_binary(&data)?, timeout: env.block.time.plus_seconds(60).into() });
    };
    

    UNCOMITTED_VALUES.update(deps.storage, 300,|mut state| -> StdResult<_> {
        Ok("SEND IBC COMMIT REQUESTS".to_string())
    })?;

    Ok(IbcBasicResponse::new()
        //.add_messages(ibc_commits)
    )
}


#[entry_point]
/// never should be called as we do not send packets
pub fn ibc_packet_timeout(
    deps: DepsMut,
    env: Env,
    msg: IbcPacketTimeoutMsg,
) -> StdResult<IbcBasicResponse> {
    UNCOMITTED_VALUES.update(deps.storage, 1000,|mut state| -> StdResult<_> {
        Ok("IBC_TIMEOUT_PACKET".to_string())
    })?;
    Ok(IbcBasicResponse::new())
}

fn enforce_order_and_version(
    channel: &IbcChannel,
    counterparty_version: Option<&str>,
) -> Result<(), ContractError> {
    if channel.version != "trustboost-test" {
        return Err(ContractError::InvalidChannelVersion {
            version: channel.version.clone(),
        });
    }
    if let Some(version) = counterparty_version {
        if version != "trustboost-test" {
            return Err(ContractError::InvalidChannelVersion {
                version: version.to_string(),
            });
        }
    }
    if channel.order != IbcOrder::Ordered {
        return Err(ContractError::OnlyOrderedChannel {});
    }
    Ok(())
}


// this encode an error or error message into a proper acknowledgement to the recevier
fn encode_ibc_error(msg: impl Into<String>) -> Binary {
    // this cannot error, unwrap to keep the interface simple
    to_binary(&AcknowledgementMsg::<()>::Err(msg.into())).unwrap()
}



// ------------------------------------------------------- TODO move to ibc.rs ------------------------------------------------------


#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies_with_balance, mock_env, mock_info};
    use cosmwasm_std::{coins, from_binary};

    // #[test]
    // fn proper_initialization() {
    //     let mut deps = mock_dependencies_with_balance(&coins(2, "token"));

    //     let msg = InstantiateMsg { count: 17 };
    //     let info = mock_info("creator", &coins(1000, "earth"));

    //     // we can just call .unwrap() to assert this was a success
    //     let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
    //     assert_eq!(0, res.messages.len());

    //     // it worked, let's query the state
    //     let res = query(deps.as_ref(), mock_env(), QueryMsg::GetCount {}).unwrap();
    //     let value: CountResponse = from_binary(&res).unwrap();
    //     assert_eq!(17, value.count);
    // }

    // #[test]
    // fn increment() {
    //     let mut deps = mock_dependencies_with_balance(&coins(2, "token"));

    //     let msg = InstantiateMsg { count: 17 };
    //     let info = mock_info("creator", &coins(2, "token"));
    //     let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    //     // beneficiary can release it
    //     let info = mock_info("anyone", &coins(2, "token"));
    //     let msg = ExecuteMsg::Increment {};
    //     let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    //     // should increase counter by 1
    //     let res = query(deps.as_ref(), mock_env(), QueryMsg::GetCount {}).unwrap();
    //     let value: CountResponse = from_binary(&res).unwrap();
    //     assert_eq!(18, value.count);
    // }

    // #[test]
    // fn reset() {
    //     let mut deps = mock_dependencies_with_balance(&coins(2, "token"));

    //     let msg = InstantiateMsg { count: 17 };
    //     let info = mock_info("creator", &coins(2, "token"));
    //     let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    //     // beneficiary can release it
    //     let unauth_info = mock_info("anyone", &coins(2, "token"));
    //     let msg = ExecuteMsg::Reset { count: 5 };
    //     let res = execute(deps.as_mut(), mock_env(), unauth_info, msg);
    //     match res {
    //         Err(ContractError::Unauthorized {}) => {}
    //         _ => panic!("Must return unauthorized error"),
    //     }

    //     // only the original creator can reset the counter
    //     let auth_info = mock_info("creator", &coins(2, "token"));
    //     let msg = ExecuteMsg::Reset { count: 5 };
    //     let _res = execute(deps.as_mut(), mock_env(), auth_info, msg).unwrap();

    //     // should now be 5
    //     let res = query(deps.as_ref(), mock_env(), QueryMsg::GetCount {}).unwrap();
    //     let value: CountResponse = from_binary(&res).unwrap();
    //     assert_eq!(5, value.count);
    //}
}
