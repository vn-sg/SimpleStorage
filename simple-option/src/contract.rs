#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, IbcMsg, IbcTimeout, MessageInfo, Order, Reply, Response,
    StdError, StdResult, SubMsg,
};

use std::convert::TryInto;

use cw2::set_contract_version;
use std::cmp::Ordering;
use std::collections::HashSet;

use crate::error::ContractError;
use crate::ibc_msg::{Msg, PacketMsg};
use crate::queue_handler::{receive_queue, send_all_party};
use crate::utils::{get_timeout, init_receive_map, get_id_channel_pair_from_storage, convert_send_ibc_msg};
use crate::view_change::{view_change, convert_queue_to_ibc_msgs, testing_add2queue};
// use crate::ibc_msg::PacketMsg;
use crate::msg::{
    AbortResponse, ChannelsResponse, DoneQueryResponse, EchoQueryResponse, ExecuteMsg,
    HighestAbortResponse, HighestReqResponse, InstantiateMsg, Key1QueryResponse, Key2QueryResponse,
    Key3QueryResponse, LockQueryResponse, QueryMsg, ReceivedSuggestResponse, SendAllUponResponse,
    StateResponse, TestQueueResponse,
};
use crate::state::{
    State, CHANNELS, DEBUG, HIGHEST_ABORT, HIGHEST_REQ, RECEIVED, RECEIVED_ECHO,
    RECEIVED_KEY1, RECEIVED_KEY2, RECEIVED_KEY3, RECEIVED_LOCK, STATE, TEST, RECEIVED_DONE, IBC_MSG_SEND_DEBUG,
};
use crate::state::{SEND_ALL_UPON, TEST_QUEUE};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:simple-storage";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const REQUEST_REPLY_ID: u64 = 100;
pub const SUGGEST_REPLY_ID: u64 = 101;
pub const PROOF_REPLY_ID: u64 = 102;
pub const PROPOSE_REPLY_ID: u64 = 103;
pub const VIEW_TIMEOUT_SECONDS: u64 = 10;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let state = State::new(msg.chain_id, msg.input, env.block.time);
    STATE.save(deps.storage, &state)?;
    for msg_type in vec!["Suggest", "Proof"] {
        RECEIVED.save(deps.storage, msg_type.to_string(), &HashSet::new())?;
    }
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // let action = |_| -> StdResult<u32> { Ok(u32::MAX) };
    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender))
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
        ExecuteMsg::PreInput { value } => handle_execute_preinput(deps, env, info, value),
        ExecuteMsg::ForceAbort {} => {
            todo!()
        }
        ExecuteMsg::Abort {} => handle_execute_abort(deps, env),
        ExecuteMsg::Trigger { behavior } => handle_trigger(deps, env, behavior),
    }
}

pub fn handle_trigger(
    deps: DepsMut,
    env: Env,
    behavior: String,
) -> Result<Response, ContractError> {

    match behavior.as_str() {
        "multi_propose" => trigger_multi_propose(deps, env),
        "key1_diff_val" => trigger_key1_diff_val(deps, env),
        "abort" => trigger_abort(deps, &env),
        "done" => trigger_done(deps, env),
        "done_2" => trigger_done_2(deps, env),
        _ => Ok(Response::new()
                .add_attribute("action", "trigger")
                .add_attribute("trigger_behavior", "unknown"))
    }
}

fn trigger_done(
    deps: DepsMut,
    env: Env
) -> Result<Response, ContractError> {
    let res = 
    Response::new()
        .add_attribute("action", "trigger")
        .add_attribute("trigger_behavior", "done");
    let state = STATE.load(deps.storage)?;

    let mut queue: Vec<Vec<Msg>> = vec!(Vec::new(); state.n.try_into().unwrap());
    // self-send msg
    // receive_queue(store, timeout, None, vec![packet.clone()], queue)?;
    let done_packet = Msg::Done {
        val: "MALICIOUS_VAL".to_string()
    };
    send_all_party(deps.storage, &mut queue, done_packet, get_timeout(&env), &env)?;
    let msgs = convert_queue_to_ibc_msgs(deps.storage, &mut queue, get_timeout(&env))?;

    Ok(res
        .add_messages(msgs))
}

fn trigger_done_2(
    deps: DepsMut,
    env: Env
) -> Result<Response, ContractError> {
    let res = 
    Response::new()
        .add_attribute("action", "trigger")
        .add_attribute("trigger_behavior", "done");
    let state = STATE.load(deps.storage)?;

    let mut queue: Vec<Vec<Msg>> = vec!(Vec::new(); state.n.try_into().unwrap());
    // self-send msg
    // receive_queue(store, timeout, None, vec![packet.clone()], queue)?;
    let packet_1 = Msg::Done {
        val: "PACKET_A".to_string()
    };

    let packet_2 = Msg::Done {
        val: "PACKET_B".to_string()
    };

    let channel_id_1 = CHANNELS.load(deps.storage, 1)?;
    let channel_id_2 = CHANNELS.load(deps.storage, 2)?;

    let mut vec_1:Vec<Msg> = Vec::new();
    let mut vec_2:Vec<Msg> = Vec::new();
    vec_1.push(packet_1);
    vec_2.push(packet_2);

    let packet_queue_1 = PacketMsg::MsgQueue(vec_1);
    let packet_queue_2 = PacketMsg::MsgQueue(vec_2);

    let ibc_packet_1 = IbcMsg::SendPacket { channel_id: channel_id_1, data: to_binary(&packet_queue_1)?, timeout: get_timeout(&env) };
    let ibc_packet_2 = IbcMsg::SendPacket { channel_id: channel_id_2, data: to_binary(&packet_queue_2)?, timeout: get_timeout(&env) };

    Ok(res
        .add_message(ibc_packet_1)
        .add_message(ibc_packet_2))
}


fn trigger_abort(
    deps: DepsMut,
    env: &Env
) -> Result<Response, ContractError> {
    let res = 
    Response::new()
        .add_attribute("action", "trigger")
        .add_attribute("trigger_behavior", "abort");
    let state = STATE.load(deps.storage)?;

    if state.chain_id == state.primary {
        return Ok(res
        .add_attribute("error", "is primary"));
    }
    // let mut msgs = Vec::new();
    let mut queue: Vec<Vec<Msg>> = vec!(Vec::new(); state.n.try_into().unwrap());
    // self-send msg
    // receive_queue(store, timeout, None, vec![packet.clone()], queue)?;
    let abort_packet = Msg::Abort {
        view: state.view,
        chain_id: state.chain_id,
    };
    send_all_party(deps.storage, &mut queue, abort_packet, get_timeout(&env), env)?;
    let msgs = convert_queue_to_ibc_msgs(deps.storage, &mut queue, get_timeout(&env))?;

    Ok(res
        .add_messages(msgs))
}

fn trigger_key1_diff_val(
    deps: DepsMut,
    env: Env
) -> Result<Response, ContractError> {
    let res = 
        Response::new()
            .add_attribute("action", "trigger");
    let state = STATE.load(deps.storage)?;
    
    if state.chain_id == state.primary {
        return Ok(res
        .add_attribute("trigger_behavior", "key1_diff_val")
        .add_attribute("error", "is primary"));
    }
    let mut msgs = Vec::new();
    // self-send msg
    // receive_queue(store, timeout, None, vec![packet.clone()], queue)?;

    let channel_ids = get_id_channel_pair_from_storage(deps.storage)?;

    for (chain_id, channel_id) in &channel_ids {
        let val = ["TRIGGER_", &chain_id.to_string()].join("");
        let msg_queue = vec![Msg::Key1 { val, view: state.view }];
        testing_add2queue(deps.storage, *chain_id, msg_queue.clone())?;
        let packet = PacketMsg::MsgQueue(msg_queue);
    
        let msg = convert_send_ibc_msg(channel_id.to_string(), packet, get_timeout(&env));
        msgs.push(msg);
    }

    return Ok(res
    .add_messages(msgs)
    .add_attribute("trigger_behavior", "key1_diff_val"));
}

fn trigger_multi_propose(
    deps: DepsMut,
    env: Env
) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;

    // check if this chain is the primary of current view
    if state.chain_id != state.primary {
        return Ok(Response::new()
        .add_attribute("action", "trigger")
        .add_attribute("trigger_behavior", "multi_propose")
        .add_attribute("error", "not primary"));
    }
    // let mut queue: Vec<Vec<Msg>> = vec!(Vec::new(); state.n.try_into().unwrap());
    let mut msgs = Vec::new();

    // Send different Propose to other parties
    let channel_ids = get_id_channel_pair_from_storage(deps.storage)?;

    for (chain_id, channel_id) in &channel_ids {
        let v = ["TRIGGER_", &chain_id.to_string()].join("");
        let msg_queue = vec![Msg::Propose {chain_id: state.chain_id, k: state.view, v, view: state.view}];
        testing_add2queue(deps.storage, *chain_id, msg_queue.clone())?;

        let packet = PacketMsg::MsgQueue(msg_queue);
        let msg = convert_send_ibc_msg(channel_id.to_string(), packet, get_timeout(&env));
        msgs.push(msg);
    }
    /* 
    for k in 0..3 {
        let v = ["TESTING", &k.to_string()].join("");
        let propose_packet = Msg::Propose {
            chain_id: state.chain_id,
            k,
            v,
            view: state.view,
        };
        
        send_all_party(deps.storage, &mut queue, propose_packet, get_timeout(&env))?;
    }
    let msgs = convert_queue_to_ibc_msgs(deps.storage, &mut queue, get_timeout(&env))?;
    */
    let mut state = STATE.load(deps.storage)?;
    state.current_tx_id += 1;
    STATE.save(deps.storage, &state)?;

    return Ok(Response::new()
    .add_messages(msgs)
    .add_attribute("action", "trigger")
    .add_attribute("trigger_behavior", "multi-propose"));
}



pub fn handle_execute_input(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    input: String,
) -> Result<Response, ContractError> {
    // set timeout for broadcasting
    let timeout: IbcTimeout = get_timeout(&env);
    /* a better way?
    CHANNELS
        .keys(deps.storage, None, None, Order::Ascending)
        .map(|id| HIGHEST_REQ.save(deps.storage, id?, &0)? );
    */

    // Initialization
    init_receive_map(deps.storage)?;
    // Re-init
    let mut state = STATE.load(deps.storage)?;
    state.re_init(input, env.block.time.clone());

    // Store values to state
    STATE.save(deps.storage, &state)?;

    // By calling view_change(), Request messages will be delivered to all chains that we established a channel with
    view_change(deps.storage, timeout.clone(), &env)

    // broadcast message
}

pub fn handle_execute_preinput(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    input: String,
) -> Result<Response, ContractError> {
    // Initialization
    init_receive_map(deps.storage)?;

    // Re-init
    let mut state = STATE.load(deps.storage)?;
    state.re_init(input, env.block.time.clone());
    // Store values to state
    STATE.save(deps.storage, &state)?;

    Ok(Response::new()
        .add_attribute("action", "execute")
        .add_attribute("msg_type", "pre_input"))
}

pub fn handle_execute_abort(deps: DepsMut, env: Env) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;


    match state.done {
        Some(val) => {
            return Err(ContractError::CustomError {val: "Process is Done Cannot abort".to_string()});
        },
        None => ()
    };

    let end_time = state.start_time.plus_seconds(VIEW_TIMEOUT_SECONDS);
    match env.block.time.cmp(&end_time) {
        Ordering::Greater => {
            let abort_packet = Msg::Abort {
                view: state.view,
                chain_id: state.chain_id,
            };
            let mut queue: Vec<Vec<Msg>> =
                vec![vec![abort_packet.clone()]; state.n.try_into().unwrap()];

            let response = receive_queue(
                deps.storage,
                get_timeout(&env),
                Some("ABORT_UNUSED_CHANNEL".to_string()),
                vec![abort_packet.clone()],
                &mut queue,
                &env
            )?;

            let sub_msgs = response.messages;

            // let state = STATE.load(deps.storage)?;
            // if previous_view != state.view {
            //     let t = format!("STATE IS NOT EQUAL PREVIOUS_VIEW = {} STATE.VIEW = {}", previous_view, state.view);
            //     DEBUG.save(deps.storage, 100, &t)?;
            //     reset_view_specific_maps(deps.storage)?;
            //     view_change(deps, get_timeout(&env))
            // } else {
            //     let t = format!("STATE IS STILL EQUAL EQUAL PREVIOUS_VIEW = {} STATE.VIEW = {}", previous_view, state.view);
            //     DEBUG.save(deps.storage, 200, &t)?;
            //     Ok(Response::new()
            //         .add_attribute("action", "execute")
            //         .add_attribute("msg_type", "abort"))
            // }

            IBC_MSG_SEND_DEBUG.save(deps.storage, "ABORT".to_string(), &sub_msgs)?;
            Ok(Response::new()
                .add_attribute("action", "execute")
                .add_submessages(sub_msgs)
                .add_attribute("msg_type", "abort"))
        }
        _ => {
            // handle_abort(deps.storage, state.view, state.chain_id);
            // Ok(response)
            Err(ContractError::CustomError {
                val: "Invalid Abort timetsamp hasn't passed yet".to_string(),
            })
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
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
        QueryMsg::GetAbortInfo {} => to_binary(&query_abort_info(deps, env)?),
        QueryMsg::GetDebug {} => to_binary(&query_debug(deps)?),
        QueryMsg::GetHighestAbort {} => to_binary(&query_highest_abort(deps)?),
        QueryMsg::GetIbcDebug {} => to_binary(&query_ibc_debug(deps)?),
     }
}

fn query_echo(deps: Deps) -> StdResult<EchoQueryResponse> {
    let query: StdResult<Vec<_>> = RECEIVED_ECHO
        .range(deps.storage, None, None, Order::Ascending)
        .collect();
    Ok(EchoQueryResponse { echo: query? })
}
fn query_key1(deps: Deps) -> StdResult<Key1QueryResponse> {
    let query: StdResult<Vec<_>> = RECEIVED_KEY1
        .range(deps.storage, None, None, Order::Ascending)
        .collect();
    Ok(Key1QueryResponse { key1: query? })
}
fn query_key2(deps: Deps) -> StdResult<Key2QueryResponse> {
    let query: StdResult<Vec<_>> = RECEIVED_KEY2
        .range(deps.storage, None, None, Order::Ascending)
        .collect();
    Ok(Key2QueryResponse { key2: query? })
}
fn query_key3(deps: Deps) -> StdResult<Key3QueryResponse> {
    let query: StdResult<Vec<_>> = RECEIVED_KEY3
        .range(deps.storage, None, None, Order::Ascending)
        .collect();
    Ok(Key3QueryResponse { key3: query? })
}
fn query_lock(deps: Deps) -> StdResult<LockQueryResponse> {
    let query: StdResult<Vec<_>> = RECEIVED_LOCK
        .range(deps.storage, None, None, Order::Ascending)
        .collect();
    Ok(LockQueryResponse { lock: query? })
}
fn query_done(deps: Deps) -> StdResult<DoneQueryResponse> {
    let query: StdResult<Vec<_>> = RECEIVED_DONE
        .range(deps.storage, None, None, Order::Ascending)
        .collect();
    Ok(DoneQueryResponse { done: query? })
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
    // let req: StdResult<Vec<_>> = RECEIVED_SUGGEST
    //     .range(deps.storage, None, None, Order::Ascending)
    //     .collect();
    let req: StdResult<HashSet<_>> = RECEIVED.load(deps.storage, "Suggest".to_string());
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

fn query_highest_abort(deps: Deps) -> StdResult<HighestAbortResponse> {
    let req: StdResult<Vec<_>> = HIGHEST_ABORT
        .range(deps.storage, None, None, Order::Ascending)
        .collect();
    Ok(HighestAbortResponse {
        highest_abort: req?,
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

fn query_abort_info(deps: Deps, env: Env) -> StdResult<AbortResponse> {
    let state = STATE.load(deps.storage)?;
    // let channels = channels?;

    let end_time = state.start_time.plus_seconds(VIEW_TIMEOUT_SECONDS);
    let timeout = match env.block.time.cmp(&end_time) {
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
        current_time: env.block.time,
        is_timeout: timeout,
        done: is_input_finished,
        should_abort: (timeout && is_input_finished),
    })
}

fn query_debug(deps: Deps) -> StdResult<Vec<(u32, String)>> {
    let test: StdResult<Vec<_>> = DEBUG
        .range(deps.storage, None, None, Order::Ascending)
        .collect();
    Ok(test?)
}

fn query_ibc_debug(deps: Deps) -> StdResult<Vec<(String, Vec<SubMsg>)>> {
    let test: StdResult<Vec<_>> = IBC_MSG_SEND_DEBUG
        .range(deps.storage, None, None, Order::Ascending)
        .collect();
    Ok(test?)
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
