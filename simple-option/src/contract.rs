#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, IbcMsg, IbcTimeout, MessageInfo, Order, Reply, Response,
    StdError, StdResult, SubMsg,
};

use cw2::set_contract_version;
use std::cmp::Ordering;

use crate::error::ContractError;
use crate::ibc_msg::Msg;
use crate::abort::handle_abort;
use crate::utils::get_timeout;
use crate::view_change::view_change;
// use crate::ibc_msg::PacketMsg;
use crate::msg::{
    ChannelsResponse, ExecuteMsg, HighestReqResponse, InstantiateMsg, QueryMsg,
    ReceivedSuggestResponse, SendAllUponResponse, StateResponse, TestQueueResponse, Key1QueryResponse, Key2QueryResponse, Key3QueryResponse, LockQueryResponse, DoneQueryResponse, EchoQueryResponse, AbortResponse,
};
use crate::state::{
    State, CHANNELS, HIGHEST_ABORT, HIGHEST_REQ, RECEIVED_PROOF, RECEIVED_SUGGEST, STATE, TEST, ECHO, KEY1, KEY2, KEY3, LOCK, DONE,
};
use crate::state::{SEND_ALL_UPON, TEST_QUEUE};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:simple-storage";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const REQUEST_REPLY_ID: u64 = 100;
pub const SUGGEST_REPLY_ID: u64 = 101;
pub const PROOF_REPLY_ID: u64 = 102;
pub const PROPOSE_REPLY_ID: u64 = 103;
pub const VIEW_TIMEOUT_SECONDS: u64 = 60;


#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let state = State::new(msg.chain_id, msg.input, env.block.time);
    STATE.save(deps.storage, &state)?;
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // let action = |_| -> StdResult<u32> { Ok(u32::MAX) };
    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender))
}

pub fn handle_execute_input(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    input: String,
) -> Result<Response, ContractError> {
    // set timeout for broadcasting
    let timeout: IbcTimeout = get_timeout(&env);
    let mut state = STATE.load(deps.storage)?;

    // Initialize highest_request (all to the max of u32 to differentiate between the initial state)
    let all_chain_ids: StdResult<Vec<_>> = CHANNELS
        .keys(deps.storage, None, None, Order::Ascending)
        .collect();
    let all_chain_ids = all_chain_ids?;
    for chain_id in all_chain_ids {
        HIGHEST_REQ.save(deps.storage, chain_id, &0)?;
        // Resetting highest_abort
        HIGHEST_ABORT.save(deps.storage, chain_id, &-1)?;
        RECEIVED_SUGGEST.save(deps.storage, chain_id, &false)?;
        RECEIVED_PROOF.save(deps.storage, chain_id, &false)?;
    }
    // initialize the highest_request of oneself
    HIGHEST_REQ.save(deps.storage, state.chain_id, &0)?;
    // initialize the highest_abort of oneself
    HIGHEST_ABORT.save(deps.storage, state.chain_id, &-1)?;
    RECEIVED_SUGGEST.save(deps.storage, state.chain_id, &false)?;
    RECEIVED_PROOF.save(deps.storage, state.chain_id, &false)?;
    /* a better way?
    CHANNELS
        .keys(deps.storage, None, None, Order::Ascending)
        .map(|id| HIGHEST_REQ.save(deps.storage, id?, &0)? );
    */

    // Initialization
    state.sent_suggest = false;
    state.done = None;
    state.sent_done = false;
    state.view = 0;
    state.cur_view = 0;
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

    // Use block time..
    state.start_time = env.block.time.clone();

    // Set the primary to be (view mod n) + 1
    state.primary = state.view % state.n + 1;

    ////    process_messages() part     ////
    // initialize proofs to an empty set
    state.proofs = Vec::new();

    // reset values
    state.is_first_propose = true;

    // Store values to state
    STATE.save(deps.storage, &state)?;

    // By calling view_change(), Request messages will be delivered to all chains that we established a channel with
    view_change(deps, timeout.clone())

    // broadcast message
}

pub fn handle_execute_abort(
    deps: DepsMut,
    env: Env
) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;
    let response = Response::new()
        .add_attribute("action", "execute")
        .add_attribute("msg_type", "abort");
    match state.start_time.plus_seconds(VIEW_TIMEOUT_SECONDS).cmp(&env.block.time) {
        Ordering::Greater => {
            handle_abort(deps.storage, state.view, state.chain_id);
            Ok(response)
        },
        _ => {
            Err(ContractError::CustomError { val: "Invalid Abort".to_string() })
        }
    } 
}

// execute entry_point is used for beginning new instance of IT-HS consensus
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Input { value } => handle_execute_input(deps, env, info, value),
        ExecuteMsg::ForceAbort {} => {
            //TODO add abort timestamp validation and start new view
            let state = STATE.load(deps.storage)?;
            let result = handle_abort(deps.storage, state.view, state.chain_id);
            let response = Response::new()
                .add_attribute("action", "execute")
                .add_attribute("msg_type", "get");
            match result {
                Ok(_) => Ok(response),
                Err(error_msg) => Err(ContractError::CustomError {
                    val: error_msg.to_string(),
                }),
            }
        },
        ExecuteMsg::Abort {} => handle_execute_abort(deps, env),
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
        QueryMsg::GetState {} => to_binary(&query_state(deps)?),
        QueryMsg::GetChannels {} => to_binary(&query_channels(deps)?),
        QueryMsg::GetTest {} => to_binary(&query_test(deps)?),
        QueryMsg::GetHighestReq {} => to_binary(&query_highest_request(deps)?),
        QueryMsg::GetReceivedSuggest {} => to_binary(&query_received_suggest(deps)?),
        QueryMsg::GetSendAllUpon {} => to_binary(&query_send_all_upon(deps)?),
        QueryMsg::GetTestQueue {} => to_binary(&query_test_queue(deps)?),
        QueryMsg::GetEcho {} => to_binary(&query_echo(deps)?),
        QueryMsg::GetKey1 {} => to_binary(&query_key1(deps)?),
        QueryMsg::GetKey2 {} => to_binary(&query_key2(deps)?),
        QueryMsg::GetKey3 {} => to_binary(&query_key3(deps)?),
        QueryMsg::GetLock {} => to_binary(&query_lock(deps)?),
        QueryMsg::GetDone {} => to_binary(&query_done(deps)?),
        QueryMsg::GetAbortInfo {} => to_binary(&query_abort_info(deps)?),
     }
}

fn query_echo(deps: Deps) -> StdResult<EchoQueryResponse> {
    let query: StdResult<Vec<_>> = ECHO
        .range(deps.storage, None, None, Order::Ascending)
        .collect();
    Ok(EchoQueryResponse {
        echo: query?,
    })
}
fn query_key1(deps: Deps) -> StdResult<Key1QueryResponse> {
    let query: StdResult<Vec<_>> = KEY1
        .range(deps.storage, None, None, Order::Ascending)
        .collect();
    Ok(Key1QueryResponse {
        key1: query?,
    })
}
fn query_key2(deps: Deps) -> StdResult<Key2QueryResponse> {
    let query: StdResult<Vec<_>> = KEY2
        .range(deps.storage, None, None, Order::Ascending)
        .collect();
    Ok(Key2QueryResponse {
        key2: query?,
    })
}
fn query_key3(deps: Deps) -> StdResult<Key3QueryResponse> {
    let query: StdResult<Vec<_>> = KEY3
        .range(deps.storage, None, None, Order::Ascending)
        .collect();
    Ok(Key3QueryResponse {
        key3: query?,
    })
}
fn query_lock(deps: Deps) -> StdResult<LockQueryResponse> {
    let query: StdResult<Vec<_>> = LOCK
        .range(deps.storage, None, None, Order::Ascending)
        .collect();
    Ok(LockQueryResponse {
        lock: query?,
    })
}
fn query_done(deps: Deps) -> StdResult<DoneQueryResponse> {
    let query: StdResult<Vec<_>> = DONE
        .range(deps.storage, None, None, Order::Ascending)
        .collect();
    Ok(DoneQueryResponse {
        done: query?,
    })
}


fn query_state(deps: Deps) -> StdResult<StateResponse> {
    let state = STATE.load(deps.storage)?;
    Ok(match state.done {
        Some(val) => StateResponse::Done { decided_val: val },
        None => StateResponse::InProgress { state },
    })
}

fn query_test_queue(deps: Deps) -> StdResult<TestQueueResponse> {
    let req: StdResult<Vec<_>> = TEST_QUEUE
        .range(deps.storage, None, None, Order::Ascending)
        .collect();
    Ok(TestQueueResponse { test_queue: req? })
}

fn query_send_all_upon(deps: Deps) -> StdResult<SendAllUponResponse> {
    let req: StdResult<Vec<_>> = SEND_ALL_UPON
        .range(deps.storage, None, None, Order::Ascending)
        .collect();
    Ok(SendAllUponResponse {
        send_all_upon: req?,
    })
}

fn query_received_suggest(deps: Deps) -> StdResult<ReceivedSuggestResponse> {
    let req: StdResult<Vec<_>> = RECEIVED_SUGGEST
        .range(deps.storage, None, None, Order::Ascending)
        .collect();
    Ok(ReceivedSuggestResponse {
        received_suggest: req?,
    })
}

fn query_highest_request(deps: Deps) -> StdResult<HighestReqResponse> {
    let req: StdResult<Vec<_>> = HIGHEST_REQ
        .range(deps.storage, None, None, Order::Ascending)
        .collect();
    Ok(HighestReqResponse {
        highest_request: req?,
    })
}

fn query_test(deps: Deps) -> StdResult<Vec<(u32, Vec<IbcMsg>)>> {
    let test: StdResult<Vec<_>> = TEST
        .range(deps.storage, None, None, Order::Ascending)
        .collect();

    Ok(test?)
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

fn query_abort_info(deps: Deps) -> StdResult<AbortResponse> {
    let state = STATE.load(deps.storage)?;
    // let channels = channels?;
    
    let end_time = state.start_time.plus_seconds(VIEW_TIMEOUT_SECONDS);
    let timeout = match end_time.cmp(&state.start_time) {
        Ordering::Greater => true,
        _ => false,
    };

    let is_input_finished = match state.done {
        Some(_) => true,
        _ => false,
    };
    
    Ok(AbortResponse {
        start_time: state.start_time,
        end_time: state.start_time.plus_seconds(60),
        is_timeout: timeout,
        done: is_input_finished,
        should_abort: (timeout && is_input_finished),
    })
}


/*
fn query_value(deps: Deps, key: String) -> StdResult<ValueResponse> {
    let value = VARS.may_load(deps.storage, &key)?;
    match value {
        Some(v) => Ok(ValueResponse::KeyFound { key, value: v }),
        None => Ok(ValueResponse::KeyNotFound {}),
    }
}
*/

// entry_point for sub-messages
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> StdResult<Response> {
    match msg.id {
        // REQUEST_REPLY_ID => handle_request_reply(deps, get_timeout(env), msg),
        REQUEST_REPLY_ID => Ok(Response::new()),
        SUGGEST_REPLY_ID => handle_suggest_reply(deps, get_timeout(&env), msg),
        id => Err(StdError::generic_err(format!("Unknown reply id: {}", id))),
    }
}

fn _handle_request_reply(deps: DepsMut, timeout: IbcTimeout, _msg: Reply) -> StdResult<Response> {
    // Upon sucessfully called the broadcast of Request Messages
    // Load the state
    let state = STATE.load(deps.storage)?;
    if state.chain_id != state.primary {
        // Upon highest_request[primary] = view
        let prim_highest_req = HIGHEST_REQ.load(deps.storage, state.primary)?;
        if prim_highest_req == state.view {
            // Contruct Suggest message to delivery to primary
            let packet = Msg::Suggest {
                chain_id: state.chain_id,
                view: state.view,
                key2: state.key2,
                key2_val: state.key2_val.clone(),
                prev_key2: state.prev_key2,
                key3: state.key3,
                key3_val: state.key3_val.clone(),
            };

            let channel_id = CHANNELS.load(deps.storage, state.primary)?;
            let msg = IbcMsg::SendPacket {
                channel_id,
                data: to_binary(&packet)?,
                timeout: timeout.clone(),
            };
            let submsg: SubMsg = SubMsg::reply_on_success(msg, SUGGEST_REPLY_ID);

            // construct Response and put Suggest message in the query on the fly
            return Ok(Response::new()
                .add_submessage(submsg)
                .add_attribute("action", "send_suggest2primary".to_string()));
        }
    }

    // TODO: Add ops for reply of Request message
    Ok(Response::new())
    // Add consecutive submessages
}

fn handle_suggest_reply(_deps: DepsMut, _timeout: IbcTimeout, _msg: Reply) -> StdResult<Response> {
    // Upon sucessfully delivered the Suggest Message
    // Load the state
    // let _state = STATE.load(deps.storage)?;
    let res: Response = Response::new();

    // Add consecutive submessages
    Ok(res)
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
        // let deps = instantiate_then_get_deps();

        // query the state and verify if successful
        // let res = query(deps.as_ref(), mock_env(), QueryMsg::GetState {}).unwrap();
        // let value: State = from_binary(&res).unwrap();
        // assert_eq!("leader", value.role);
        // assert_eq!(0, value.current_tx_id);
        // assert!(value.channel_ids.is_empty());
    }

    #[test]
    fn test_execute() {
        // let mut deps = mock_dependencies_with_balance(&coins(2, "token"));
        // let mut deps = mock_dependencies();

        // let msg = InstantiateMsg {
        //     // role: "leader".to_string(),
        //     chain_id: 0,
        //     input: 0.to_string(),
        // };
        // let info = mock_info("creator_V", &coins(100, "BTC"));
        // let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // // call the execute function of contract
        // let info = mock_info("anyone", &coins(2, "token"));
        // let msg = ExecuteMsg::Set {
        //     key: "TestKey".to_string(),
        //     // value: "value_of_TestKey".to_string(),
        //     value: 0,
        // };
        // let _res = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap();

        // // check if map in local state has been updated correctly
        // let res = query(
        //     deps.as_ref(),
        //     mock_env(),
        //     QueryMsg::GetTx {
        //         tx_id: "0".to_string(),
        //     },
        // )
        // .unwrap();
        // let tx: Tx = from_binary(&res).unwrap();
        // assert_eq!(1, tx.no_of_votes);
        // assert_eq!(msg.clone(), tx.msg);

        // // CHECK for key/value in VARS
        // let res = query(deps.as_ref(), mock_env(), QueryMsg::GetValue { key: "TestKey".to_string() }).unwrap();
        // let value: String = from_binary(&res).unwrap();
        // assert_eq!("value_of_TestKey", value);

        // // should increase counter by 1
        // let res = query(deps.as_ref(), mock_env(), QueryMsg::GetCount {}).unwrap();
        // let value: CountResponse = from_binary(&res).unwrap();
        // assert_eq!(18, value.count);
    }

    // fn instantiate_then_get_deps() -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
    //     let mut deps = mock_dependencies();
    //     let msg = InstantiateMsg {
    //         // role: "leader".to_string(),
    //         chain_id: 0,
    //         input: 0.to_string(),
    //     };
    //     let info = mock_info("creator_V", &coins(100, "BTC"));
    //     // we can just call .unwrap() to assert this was a success
    //     let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
    //     assert_eq!(0, res.messages.len());
    //     deps
    // }
}
