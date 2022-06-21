#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};

use cw2::set_contract_version;

use crate::error::ContractError;
use crate::ibc::{PACKET_LIFETIME, broadcast_response};
use crate::ibc_msg::PacketMsg;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, ValueResponse};
use crate::state::{State, STATE, VARS, Tx, TXS};

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
        channel_ids: Vec::new(),
        current_tx_id: 0
        // votes_count: 0,
        // variables: HashMap::new(),
    };
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    STATE.save(deps.storage, &state)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender))
    // .add_attribute("count", msg.count.to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {

    // construct a packet to send
    let mut state = STATE.load(deps.storage)?;
    let tx_id = state.current_tx_id.clone();
    let channel_ids = state.channel_ids.clone();

    // Initialize tx info and store in local state(TXS)
    let action = |_| -> StdResult<Tx> {
        Ok(Tx { 
            msg: msg.clone(), 
            no_of_votes: 1
        })
    };
    TXS.update(deps.storage, tx_id.clone(), action)?;
    let packet = PacketMsg::Propose { msg: msg.clone(), tx_id };
    // Update the tx_id to assign and save current state
    state.current_tx_id += 1;
    STATE.save(deps.storage, &state)?;

    // broadcast Propose message
    let timeout = env.block.time.plus_seconds(PACKET_LIFETIME).into();
    broadcast_response(timeout, channel_ids, packet, "broadcast_propose".to_string())

    // let msg0 = IbcMsg::SendPacket {
    //     channel_id: channel_ids[0].clone(),
    //     data: to_binary(&packet)?,
    //     timeout: env.block.time.plus_seconds(PACKET_LIFETIME).into(),
    // };
    // let msg1 = IbcMsg::SendPacket {
    //     channel_id: channel_ids[1].clone(),
    //     data: to_binary(&packet)?,
    //     timeout: env.block.time.plus_seconds(PACKET_LIFETIME).into(),
    // };

    // let res = Response::new()
    //     .add_message(msg0)
    //     .add_message(msg1)
    //     .add_attribute("action", "broadcast_propose");
    // Ok(res)

    // match msg {
    //     ExecuteMsg::Set { key, value } => try_set(deps, key, value),
    //     ExecuteMsg::Get { key } => try_get(deps, key),
    // }
}




// pub fn try_increment(deps: DepsMut) -> Result<Response, ContractError> {
//     STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
//         state.count += 1;
//         Ok(state)
//     })?;

//     Ok(Response::new().add_attribute("method", "try_increment"))
// }

// pub fn try_reset(deps: DepsMut, info: MessageInfo, count: i32) -> Result<Response, ContractError> {
//     STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
//         if info.sender != state.owner {
//             return Err(ContractError::Unauthorized {});
//         }
//         state.count = count;
//         Ok(state)
//     })?;
//     Ok(Response::new().add_attribute("method", "reset"))
// }

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetValue{ key } => to_binary(&query_value(deps,key)?),
        QueryMsg::GetState {  } => to_binary(&STATE.load(deps.storage)?), 
        QueryMsg::GetTx { tx_id } => to_binary(&query_tx(deps, tx_id)?), 
    }
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

        // let msg = InstantiateMsg { role: "leader".to_string() };
        // let info = mock_info("creator_V", &coins(100, "BTC"));

        // // we can just call .unwrap() to assert this was a success
        // let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        // assert_eq!(0, res.messages.len());

        // query the state and verify if successful
        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetState {}).unwrap();
        let value: State = from_binary(&res).unwrap();
        assert_eq!("leader", value.role);
        assert_eq!(0, value.current_tx_id);
        assert!(value.channel_ids.is_empty());
    }

    #[test]
    fn test_execute() {
        // let mut deps = mock_dependencies_with_balance(&coins(2, "token"));
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg { role: "leader".to_string() };
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

        // // should increase counter by 1
        // let res = query(deps.as_ref(), mock_env(), QueryMsg::GetCount {}).unwrap();
        // let value: CountResponse = from_binary(&res).unwrap();
        // assert_eq!(18, value.count);
    }

    fn instantiate_then_get_deps() -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg { role: "leader".to_string() };
        let info = mock_info("creator_V", &coins(100, "BTC"));
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
        deps
    }

//     #[test]
//     fn reset() {
//         let mut deps = mock_dependencies_with_balance(&coins(2, "token"));

//         let msg = InstantiateMsg { role: todo!(), channel_ids: todo!() };
//         let info = mock_info("creator", &coins(2, "token"));
//         let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

//         // beneficiary can release it
//         let unauth_info = mock_info("anyone", &coins(2, "token"));
//         let msg = ExecuteMsg::Reset { count: 5 };
//         let res = execute(deps.as_mut(), mock_env(), unauth_info, msg);
//         match res {
//             Err(ContractError::Unauthorized {}) => {}
//             _ => panic!("Must return unauthorized error"),
//         }

//         // only the original creator can reset the counter
//         let auth_info = mock_info("creator", &coins(2, "token"));
//         let msg = ExecuteMsg::Reset { count: 5 };
//         let _res = execute(deps.as_mut(), mock_env(), auth_info, msg).unwrap();

//         // should now be 5
//         let res = query(deps.as_ref(), mock_env(), QueryMsg::GetCount {}).unwrap();
//         let value: CountResponse = from_binary(&res).unwrap();
//         assert_eq!(5, value.count);
//     }
}
