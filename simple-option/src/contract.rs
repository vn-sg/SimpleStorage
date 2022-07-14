use std::vec;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Order, IbcTimeout, IbcMsg};

use cw2::set_contract_version;

use crate::error::ContractError;
use crate::ibc::{PACKET_LIFETIME, create_broadcast_msgs};
use crate::ibc_msg::PacketMsg;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, ValueResponse, ChannelsResponse};
use crate::state::{State, STATE, VARS, Tx, TXS, CHANNELS, Test, HIGHEST_REQ, HIGHEST_ABORT};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:simple-storage";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let state = State {
        role: msg.role,
        chain_id: msg.chain_id,
        channel_ids: Vec::new(),
        current_tx_id: 0,
        view: 0,
        cur_view: 0,
        primary: 1,
        key1: 0,
        key2: 0,
        key3: 0,
        lock: 0,
        key1_val: msg.input,
        key2_val: msg.input,
        key3_val: msg.input,
        lock_val: msg.input,
    
        prev_key1: -1,
        prev_key2: -1,
    };
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    STATE.save(deps.storage, &state)?;

    // initialize the highest_request of oneself
    let action = |_| -> StdResult<u32> {
        Ok(0)
    };
    // initialize the highest_request of oneself
    HIGHEST_REQ.update(deps.storage, msg.chain_id, action)?;
    // initialize the highest_abort of oneself
    HIGHEST_ABORT.save(deps.storage, msg.chain_id, &0)?;
    

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    // load the state
    let mut state = STATE.load(deps.storage)?;
    // set timeout for broadcasting
    let timeout: IbcTimeout = env.block.time.plus_seconds(PACKET_LIFETIME).into();
 
    // Set the primary to be (view mod n) + 1
    state.primary = state.view % 4 + 1;

    // Add Request message to packets_to_be_broadcasted
    let packets = vec![PacketMsg::Request{ view: state.view, chain_id: state.chain_id }];
    // Contruct messages to be broadcasted
    let mut msgs: Vec<IbcMsg> = create_broadcast_msgs(timeout.clone(), state.channel_ids.clone(), packets, false)?;

    // Upon highest_request[primary] = view
    let prim_highest = HIGHEST_REQ.load(deps.storage, state.primary)?;
    let packet_to_broadcast =vec![PacketMsg::Suggest { 
        view: state.view, 
        key2: state.key2, 
        key2_val: state.key2_val , 
        prev_key2: state.prev_key2, 
        key3: state.key3, 
        key3_val: state.key3_val }];
    if prim_highest == state.view {
        msgs.extend(create_broadcast_msgs(
            timeout.clone(), 
            vec![CHANNELS.load(deps.storage, state.primary)?], 
            packet_to_broadcast, 
            true)?);
    }

    let tx_id = state.current_tx_id.clone();
    // let channel_ids = state.channel_ids.clone();
    // let channel_ids = state.channel_ids.values().cloned().collect();
    // let channel_ids: StdResult<Vec<_>> = CHANNELS
    //         .range(deps.storage, None, None, Order::Ascending)
    //         .collect();

    // Initialize tx info and store in local state(TXS)
    TXS.save(deps.storage, tx_id.clone(), 
    &Tx { 
        msg: msg.clone(), 
        no_of_votes: 1
    })?; 
    // packet.push(PacketMsg::Propose { msg: msg.clone(), tx_id });
    // Update the tx_id to assign and save current state
    state.current_tx_id += 1;
    
    // if state.primary == -1 {
        // packet.push(PacketMsg::WhoAmI { chain_id: state.chain_id });
        // state.primary = (state.view % 4) as i32;
    // }
    STATE.save(deps.storage, &state)?;

    // broadcast message
    let res = Response::new()
        .add_messages(msgs)
        .add_attribute("action", "broadcast");
    Ok(res)
    
    // broadcast_response(timeout.clone(), channel_ids, packet, "broadcast_propose".to_string())

}


#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetValue{ key } => to_binary(&query_value(deps,key)?),
        QueryMsg::GetState {  } => to_binary(&STATE.load(deps.storage)?), 
        QueryMsg::GetTx { tx_id } => to_binary(&query_tx(deps, tx_id)?), 
        QueryMsg::GetChannels{  } => to_binary(&query_channels(deps)?),
        QueryMsg::GetTest{  } => to_binary(&query_test(deps)?),
    }
}

fn query_test(_deps: Deps) -> StdResult<Vec<(u32, Test)>> {
    // let test: StdResult<Vec<_>> = TEST
    // .range(deps.storage, None, None, Order::Ascending)
    // .collect();

    Ok(Vec::new())
}


fn query_channels(deps: Deps) -> StdResult<ChannelsResponse> {
    let channels: StdResult<Vec<_>> = CHANNELS
    .range(deps.storage, None, None, Order::Ascending)
    .collect();
    // let channels = channels?;
    Ok(ChannelsResponse {
        port_chan_pair: channels?
    })

}

fn query_tx(deps: Deps, tx_id: String ) -> StdResult<Tx> {
    let tx_id = tx_id.parse::<u32>().unwrap();
    let tx = TXS.may_load(deps.storage, tx_id)?;
    match tx {
        Some(tx) => Ok(tx),
        None => Ok(Tx { 
            msg: ExecuteMsg::Get { key: "-1".to_string() },
            no_of_votes: u32::MAX
        }),
    }

}

fn query_value(deps: Deps, key: String) -> StdResult<ValueResponse> {
    // let state = STATE.load(deps.storage)?;
    let value = VARS.may_load(deps.storage, &key)?;
    match value {
        Some(v) => Ok(ValueResponse::KeyFound { key, value: v }),
        None => Ok(ValueResponse::KeyNotFound { }),
    }
    
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_env, mock_info, mock_dependencies, MockStorage, MockQuerier, MockApi};
    use cosmwasm_std::{coins, from_binary, OwnedDeps};

    #[test]
    fn proper_initialization() {
        let deps = instantiate_then_get_deps();

        // query the state and verify if successful
        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetState {}).unwrap();
        let value: State = from_binary(&res).unwrap();
        assert_eq!("leader", value.role);
        assert_eq!(0, value.current_tx_id);
        // assert!(value.channel_ids.is_empty());
    }

    #[test]
    fn test_execute() {
        // let mut deps = mock_dependencies_with_balance(&coins(2, "token"));
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg { role: "leader".to_string(), chain_id: 0, input: 0 };
        let info = mock_info("creator_V", &coins(100, "BTC"));
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // call the execute function of contract
        let info = mock_info("anyone", &coins(2, "token"));
        let msg = ExecuteMsg::Set { key: "TestKey".to_string(), value: "value_of_TestKey".to_string() };
        let _res = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap();

        // check if map in local state has been updated correctly
        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetTx { tx_id: "0".to_string() }).unwrap();
        let tx: Tx = from_binary(&res).unwrap();
        assert_eq!(1, tx.no_of_votes);
        assert_eq!(msg.clone(), tx.msg);
        
        // CHECK for key/value in VARS
        // let res = query(deps.as_ref(), mock_env(), QueryMsg::GetValue { key: "TestKey".to_string() }).unwrap();
        // let value: String = from_binary(&res).unwrap();
        // assert_eq!("value_of_TestKey", value);

        // should increase counter by 1
        // let res = query(deps.as_ref(), mock_env(), QueryMsg::GetCount {}).unwrap();
        // let value: CountResponse = from_binary(&res).unwrap();
        // assert_eq!(18, value.count);

    }

    fn instantiate_then_get_deps() -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg { role: "leader".to_string(), chain_id: 0, input: 0};
        let info = mock_info("creator_V", &coins(100, "BTC"));
        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
        deps
    }

}
