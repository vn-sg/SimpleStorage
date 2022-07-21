use std::thread::current;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, IbcTimeout, MessageInfo, Order, Response, StdResult,
    Reply, SubMsgResult
};

use cw2::set_contract_version;

use crate::error::ContractError;
use crate::ibc::{view_change, PACKET_LIFETIME, handle_client_request};
use crate::msg::{ChannelsResponse, ExecuteMsg, InstantiateMsg, QueryMsg, ValueResponse, SuggestionsResponse, ClientReqResponse};
use crate::state::{State, Test, Tx, CHANNELS, HIGHEST_ABORT, HIGHEST_REQ, STATE, TXS, VARS, RECEIVED_SUGGEST, 
    CLIENT_REQ_COUNT, CLIENT_TOTAL_COUNT, NODE_COUNT, FAILURE_COUNT};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:simple-storage";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
const SELF_EXEC_ID_DEBUG: u64 = 1000;



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
        key1_val: msg.input.clone(),
        key2_val: msg.input.clone(),
        key3_val: msg.input.clone(),
        lock_val: msg.input.clone(),

        prev_key1: -1,
        prev_key2: -1,

        suggestions: Vec::new(),
        key2_proofs: Vec::new(),
        proofs: Vec::new(),
        is_first_propose: true

    };
    STATE.save(deps.storage, &state)?;
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // initialize the highest_request of oneself
    let action = |_| -> StdResult<u32> { Ok(u32::MAX) };
    // initialize the highest_request of oneself
    HIGHEST_REQ.update(deps.storage, msg.chain_id, action)?;
    // initialize the highest_abort of oneself
    HIGHEST_ABORT.save(deps.storage, msg.chain_id, &0)?;

    CLIENT_TOTAL_COUNT.save(deps.storage, &0)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender))
}

pub fn handle_execute_input(
    deps: DepsMut,
    env: Env,
    input: String,
) -> Result<Response, ContractError> {
    // set timeout for broadcasting
    let timeout: IbcTimeout = env.block.time.plus_seconds(PACKET_LIFETIME).into();

    // Initialize highest_request (all to the max of u32 to differentiate between the initial state)
    let all_chain_ids: StdResult<Vec<_>> = CHANNELS
        .keys(deps.storage, None, None, Order::Ascending)
        .collect();
    let all_chain_ids = all_chain_ids?;
    for chain_id in all_chain_ids {
        HIGHEST_REQ.save(deps.storage, chain_id, &u32::MAX)?;
        // Resetting highest_abort
        HIGHEST_ABORT.save(deps.storage, chain_id, &0)?;
        RECEIVED_SUGGEST.save(deps.storage, chain_id, &false)?;
    }
    /* a better way?
    CHANNELS
        .keys(deps.storage, None, None, Order::Ascending)
        .map(|id| HIGHEST_REQ.save(deps.storage, id?, &0)? );
    */
    let mut state = STATE.load(deps.storage)?;
    // Initialization
    state.view = 0;
    state.cur_view = 0;
    state.primary = 1;
    state.key1 = 0;
    state.key2 = 0;
    state.key3 = 0;
    state.lock = 0;
    state.prev_key1 = -1;
    state.prev_key2 = -1;
    state.key1_val = input.clone();
    state.key2_val = input.clone();
    state.key3_val = input.clone();
    state.lock_val = input.clone();
    // Set suggestions and key2_proofs to empty set
    state.suggestions = Vec::new();
    state.key2_proofs = Vec::new();

    // Set the primary to be (view mod n) + 1
    state.primary = state.view % NODE_COUNT + 1;

    //// process_messages() part ////
    // initialize proofs to an empty set
    state.proofs = Vec::new();

    // reset values
    state.is_first_propose = true;

    // Store values to state
    STATE.save(deps.storage, &state)?;

    let msgs = view_change(deps, timeout.clone())?;

    // broadcast message
    let res = Response::new()
        .add_messages(msgs)
        .add_attribute("action", "broadcast");

    Ok(res)
}

// execute entry_point is used for beginning new instance of IT-HS consensus
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Set { key: _, value: _ } => {
            // Plain response
            let res = Response::new()
                .add_attribute("action", "execute")
                .add_attribute("msg_type", "get");
            Ok(res)
        },
        ExecuteMsg::Get { key: _ } => {
            // Plain response
            let res = Response::new()
                .add_attribute("action", "execute")
                .add_attribute("msg_type", "get");
            Ok(res)
        },
        ExecuteMsg::Input { value } => handle_execute_input(deps, env, value),
        ExecuteMsg::InputTest { val, val2 } => {
            handle_execute_input(deps, env, val)
        },
        ExecuteMsg::ClientRequest { value } => {
            let timeout: IbcTimeout = env.block.time.plus_seconds(PACKET_LIFETIME).into();
            handle_client_request(deps, env, value, timeout.clone())
        }
    }

    // let channel_ids = state.channel_ids.clone();
    // let channel_ids = state.channel_ids.values().cloned().collect();
    // let channel_ids: StdResult<Vec<_>> = CHANNELS
    //         .range(deps.storage, None, None, Order::Ascending)
    //         .collect();

    // let mut state = STATE.load(deps.storage)?;
    // let tx_id = state.current_tx_id.clone();

    // // Initialize tx info and store in local state(TXS)
    // TXS.save(
    //     deps.storage,
    //     tx_id.clone(),
    //     &Tx {
    //         msg: msg.clone(),
    //         no_of_votes: 1,
    //     },
    // )?;
    // // Update the tx_id to assign and save current state
    // state.current_tx_id += 1;

    // STATE.save(deps.storage, &state)?;

    // broadcast_response(timeout.clone(), channel_ids, packet, "broadcast_propose".to_string())
}


#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetValue { key } => to_binary(&query_value(deps, key)?),
        QueryMsg::GetState {} => to_binary(&STATE.load(deps.storage)?),
        QueryMsg::GetTx { tx_id } => to_binary(&query_tx(deps, tx_id)?),
        QueryMsg::GetChannels {} => to_binary(&query_channels(deps)?),
        QueryMsg::GetTest {} => to_binary(&query_test(deps)?),
        QueryMsg::GetSuggestions { } => to_binary(&query_suggestions(deps)?),
        QueryMsg::GetClientReqCount {  } => to_binary(&query_client_req(deps)?),
    }
}

fn query_client_req(deps: Deps) -> StdResult<ClientReqResponse> {
    let ClientReqCounts: StdResult<Vec<_>> = CLIENT_REQ_COUNT
        .range(deps.storage, None, None, Order::Ascending)
        .collect();
    // let channels = channels?;
    Ok(ClientReqResponse {
        client_req_count: ClientReqCounts?,
    })
}


fn query_suggestions(deps: Deps) -> StdResult<SuggestionsResponse> {
    let state = STATE.load(deps.storage)?;
    Ok(SuggestionsResponse {
        suggestions: state.suggestions,
    })
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
        port_chan_pair: channels?,
    })
}

fn query_tx(deps: Deps, tx_id: String) -> StdResult<Tx> {
    let tx_id = tx_id.parse::<u32>().unwrap();
    let tx = TXS.may_load(deps.storage, tx_id)?;
    match tx {
        Some(tx) => Ok(tx),
        None => Ok(Tx {
            msg: ExecuteMsg::Get {
                key: "-1".to_string(),
            },
            no_of_votes: u32::MAX,
        }),
    }
}

fn query_value(deps: Deps, key: String) -> StdResult<ValueResponse> {
    // let state = STATE.load(deps.storage)?;
    let value = VARS.may_load(deps.storage, &key)?;
    match value {
        Some(v) => Ok(ValueResponse::KeyFound { key, value: v }),
        None => Ok(ValueResponse::KeyNotFound {}),
    }
}


#[entry_point]
pub fn reply(deps: DepsMut, _env: Env, reply: Reply) -> StdResult<Response> {
    match (reply.id, reply.result) {
        (SELF_EXEC_ID_DEBUG, SubMsgResult::Err(err)) => {
            CLIENT_REQ_COUNT.update(deps.storage, err.into(),|mut state| -> StdResult<_> {
                Ok(601)
            })?;        
            Ok(Response::new())
        }    
        _ => {    
            Ok(Response::new())        
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{
        mock_dependencies, mock_env, mock_info, MockApi, MockQuerier, MockStorage,
    };
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

        let msg = InstantiateMsg {
            role: "leader".to_string(),
            chain_id: 0,
            input: 0.to_string(),
        };
        let info = mock_info("creator_V", &coins(100, "BTC"));
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // call the execute function of contract
        let info = mock_info("anyone", &coins(2, "token"));
        let msg = ExecuteMsg::Set {
            key: "TestKey".to_string(),
            // value: "value_of_TestKey".to_string(),
            value: 0
        };
        let _res = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap();

        // check if map in local state has been updated correctly
        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::GetTx {
                tx_id: "0".to_string(),
            },
        )
        .unwrap();
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
        let msg = InstantiateMsg {
            role: "leader".to_string(),
            chain_id: 0,
            input: 0.to_string(),
        };
        let info = mock_info("creator_V", &coins(100, "BTC"));
        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
        deps
    }
}
