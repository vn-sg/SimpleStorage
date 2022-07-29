use std::convert::TryInto;

use cosmwasm_std::{
    entry_point, from_slice, to_binary, Binary, DepsMut, Env, Event, IbcTimeout, Response, StdResult, SubMsg,
};
use cosmwasm_std::{
    IbcBasicResponse, IbcChannelCloseMsg, IbcChannelConnectMsg, IbcChannelOpenMsg, IbcMsg, IbcPacketAckMsg, IbcPacketReceiveMsg, IbcPacketTimeoutMsg, IbcReceiveResponse,
};

use crate::contract::{REQUEST_REPLY_ID};
use crate::ibc_msg::{
    AcknowledgementMsg, PacketMsg, WhoAmIResponse, ProofResponse, EchoResponse, Key1Response, Key2Response, Key3Response, LockResponse, DoneResponse, Msg,
};

use crate::state::{
    CHANNELS, STATE,
};
use crate::utils::{get_timeout, F};
use crate::queue_handler::{receive_queue};

use crate::ContractError;


// pub fn send_all_upon_join_sub(
//     deps: &DepsMut,
//     timeout: IbcTimeout,
//     mut res: Response,
//     packet_to_broadcast: PacketMsg,
//     reply_id: u64
// ) -> Result<Response, ContractError> {
//     let channel_ids = get_id_channel_pair(deps.storage)?;
//     // let mut res = res;
//     let state = STATE.load(deps.storage)?;
//     for (chain_id, channel_id) in &channel_ids {
//         let highest_request = HIGHEST_REQ.load(deps.storage, chain_id.clone())?;
//         if highest_request == state.view {
//             let msg = IbcMsg::SendPacket {
//                 channel_id: channel_id.clone(),
//                 data: to_binary(&packet_to_broadcast)?,
//                 timeout: timeout.clone(),
//             };
//             let submsg = SubMsg::reply_on_success(msg, reply_id);
//             res = res.add_submessage(submsg);
//         }
//     }

//     Ok(res)
// }

// pub fn send_all_upon_join(
//     deps: &DepsMut,
//     timeout: IbcTimeout,
//     packet_to_broadcast: PacketMsg,
// ) -> Result<Vec<SubMsg>, ContractError> {
//     let channel_ids = get_id_channel_pair(deps.storage)?;

//     let mut msgs = Vec::new();
//     let state = STATE.load(deps.storage)?;
//     for (chain_id, channel_id) in &channel_ids {
//         let highest_request = HIGHEST_REQ.load(deps.storage, chain_id.clone())?;
//         if highest_request == state.view {
//             let msg = IbcMsg::SendPacket {
//                 channel_id: channel_id.clone(),
//                 data: to_binary(&packet_to_broadcast)?,
//                 timeout: timeout.clone(),
//             };
//             let submsg: SubMsg = SubMsg::reply_on_success(msg, PROPOSE_REPLY_ID);
//             msgs.push(submsg);
//         }
//     }

//     Ok(msgs)
// }

pub fn broadcast_submsgs(
    attrib: String,
    timeout: IbcTimeout,
    channel_ids: Vec<String>,
    packet_to_broadcast: PacketMsg,
) -> Result<Response, ContractError> {
    let mut msgs = Vec::new();
    for channel_id in channel_ids {
        let msg = IbcMsg::SendPacket {
            channel_id: channel_id.clone(),
            data: to_binary(&packet_to_broadcast)?,
            timeout: timeout.clone()
        };
        let submsg = SubMsg::reply_on_success(msg, REQUEST_REPLY_ID);
        // let submsg = msg;
        msgs.push(submsg);
    }
    let res = Response::new()
        .add_submessages(msgs)
        .add_attribute("action", attrib);
    Ok(res)
}

pub fn add_broadcast_submsgs(
    mut res: Response,
    timeout: IbcTimeout,
    channel_ids: Vec<String>,
    packet_to_broadcast: PacketMsg,
    submsg_id: u64
) -> Result<Response, ContractError> {
        for channel_id in channel_ids {
            let msg = IbcMsg::SendPacket {
                channel_id: channel_id.clone(),
                data: to_binary(&packet_to_broadcast)?,
                timeout: timeout.clone()
            };
            let submsg = SubMsg::reply_on_success(msg, submsg_id);
            res = res.add_submessage(submsg);
        }
    Ok(res)
}


pub fn add_broadcast_msgs(
    mut res: Response,
    timeout: IbcTimeout,
    channel_ids: Vec<String>,
    packet_to_broadcast: PacketMsg,
) -> Result<Response, ContractError> {
    // let mut res = res;
        for channel_id in channel_ids {
            let msg = IbcMsg::SendPacket {
                channel_id: channel_id.clone(),
                data: to_binary(&packet_to_broadcast)?,
                timeout: timeout.clone()
            };
            res = res.add_message(msg);
        }
    // }
    Ok(res)
}

pub fn broadcast_response(
    timeout: IbcTimeout,
    channel_ids: Vec<(u32, String)>,
    packets_to_broadcast: Vec<PacketMsg>,
    attrib: String,
) -> Result<Response, ContractError> {
    // broadcast Propose message
    let mut msgs: Vec<IbcMsg> = Vec::new();
    for packet in packets_to_broadcast {
        for (_, channel_id) in &channel_ids {
            let msg = IbcMsg::SendPacket {
                channel_id: channel_id.clone(),
                data: to_binary(&packet)?,
                timeout: timeout.clone(),
            };
            msgs.push(msg);
        }
    }

    let res = Response::new()
        .add_messages(msgs)
        .add_attribute("action", attrib);
    Ok(res)
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

    // Keep a record of connected channels
    let mut state = STATE.load(deps.storage)?;
    state.channel_ids.push(channel_id.to_string());
    // increment the total no of chains
    state.n += 1;
    STATE.save(deps.storage, &state)?;
    // let dst_port =  &channel.counterparty_endpoint.port_id;

    // let action = | mut state: State | -> StdResult<_> {
    //     state.channel_ids.insert(dst_port.to_string(), channel_id.to_string());
    //     Ok(state)
    // };
    // Storing channel_id info to state
    // STATE.update(deps.storage, action)?;

    // let action = |_| -> StdResult<String> {
    //     Ok(channel_id.to_string())
    // };
    // CHANNELS.update(deps.storage, dst_port.to_string(), action)?;

    // construct a packet to send, using the WhoAmI specification
    let packet = PacketMsg::WhoAmI {
        chain_id: state.chain_id,
    };
    let msg = IbcMsg::SendPacket {
        channel_id: channel_id.clone(),
        data: to_binary(&packet)?,
        timeout: get_timeout(env)
    };

    Ok(IbcBasicResponse::new()
        .add_message(msg)
        .add_attribute("action", "ibc_connect")
        .add_attribute("channel_id", channel_id))
}

#[entry_point]
/// On closed channel, simply delete the channel_id local state
pub fn ibc_channel_close(
    _deps: DepsMut,
    _env: Env,
    msg: IbcChannelCloseMsg,
) -> StdResult<IbcBasicResponse> {
    // fetch the connected channel_id
    let channel = msg.channel();
    let channel_id = &channel.endpoint.channel_id;
    // Remove the channel_ids stored in CHANNELS
    // CHANNELS.remove(deps.storage, dst_port.to_string());

    // let action = | mut state: State | -> StdResult<_> {
    //     state.channel_ids.retain(|_, v| !(v==channel_id));
    //     Ok(state)
    // };
    // STATE.update(deps.storage, action)?;

    // let action = |_| -> StdResult<String> {
    //     Ok(channel_id.to_string())
    // };
    // CHANNELS.update(deps.storage, dst_port.to_string(), action)?;

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
    env: Env,
    msg: IbcPacketReceiveMsg,
) -> StdResult<IbcReceiveResponse> {
    (|| {
        let packet = msg.packet;
        // which local channel did this packet come on
        let dest_channel_id = packet.dest.channel_id;
        let msg: PacketMsg = from_slice(&packet.data)?;
        match msg {
            PacketMsg::MsgQueue(q) => 
            {
                let state = STATE.load(deps.storage)?;
                let mut queue: Vec<Vec<Msg>> = vec!(Vec::new(); state.n.try_into().unwrap());
                receive_queue(deps.storage, get_timeout(env), Some(dest_channel_id), q, &mut queue)
            },
            // PacketMsg::Propose {
            //     chain_id,
            //     k,
            //     v,
            //     view,
            // } => receive_propose(
            //     deps,
            //     dest_channel_id,
            //     get_timeout(env),
            //     chain_id,
            //     k,
            //     v,
            //     view,
            // ),
            PacketMsg::WhoAmI { chain_id } => receive_who_am_i(deps, dest_channel_id, chain_id),
            // // PacketMsg::Commit { msg, tx_id } => receive_commit(deps, dest_channel_id, msg, tx_id),
            // PacketMsg::Request { view, chain_id } => {
            //     receive_request(deps, dest_channel_id, view, chain_id)
            // }
            // PacketMsg::Suggest {
            //     chain_id,
            //     view,
            //     key2,
            //     key2_val,
            //     prev_key2,
            //     key3,
            //     key3_val,
            // } => receive_suggest(
            //     deps,
            //     env,
            //     chain_id,
            //     view,
            //     key2,
            //     key2_val,
            //     prev_key2,
            //     key3,
            //     key3_val,
            // ),
            // PacketMsg::Proof {
            //     key1,
            //     key1_val,
            //     prev_key1,
            //     view,
            // } => receive_proof(deps, key1, key1_val, prev_key1, view),
            // PacketMsg::Echo { val, view} => receive_echo(deps, val, view),
            // PacketMsg::Key1 { val, view } => receive_key1(deps, val, view),
            // PacketMsg::Key2 { val, view } => receive_key2(deps, val, view),
            // PacketMsg::Key3 { val, view } => receive_key3(deps, val, view),
            // PacketMsg::Lock { val, view } => receive_lock(deps, val, view),
            // PacketMsg::Done { val } => receive_done(deps, val),
            // PacketMsg::Abort { view, chain_id } => todo!()
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
}

pub fn receive_done(
    _deps: DepsMut,
    _val: String,
) -> StdResult<IbcReceiveResponse> {
    let acknowledgement = to_binary(&AcknowledgementMsg::Ok(DoneResponse { }))?;
    Ok(IbcReceiveResponse::new()
        .set_ack(acknowledgement)
        .add_attribute("action", "receive_done"))  
}

pub fn receive_lock(
    _deps: DepsMut,
    _val: String,
    _view: u32,
) -> StdResult<IbcReceiveResponse> {
    let acknowledgement = to_binary(&AcknowledgementMsg::Ok(LockResponse { }))?;
    Ok(IbcReceiveResponse::new()
        .set_ack(acknowledgement)
        .add_attribute("action", "receive_lock"))  
}

pub fn receive_key3(
    _deps: DepsMut,
    _val: String,
    _view: u32,
) -> StdResult<IbcReceiveResponse> {
    let acknowledgement = to_binary(&AcknowledgementMsg::Ok(Key3Response { }))?;
    Ok(IbcReceiveResponse::new()
        .set_ack(acknowledgement)
        .add_attribute("action", "receive_key3"))  
}

pub fn receive_key2(
    _deps: DepsMut,
    _val: String,
    _view: u32,
) -> StdResult<IbcReceiveResponse> {
    let acknowledgement = to_binary(&AcknowledgementMsg::Ok(Key2Response { }))?;
    Ok(IbcReceiveResponse::new()
        .set_ack(acknowledgement)
        .add_attribute("action", "receive_key2"))  
}

pub fn receive_key1(
    _deps: DepsMut,
    _val: String,
    _view: u32,
) -> StdResult<IbcReceiveResponse> {

    let acknowledgement = to_binary(&AcknowledgementMsg::Ok(Key1Response { }))?;
    Ok(IbcReceiveResponse::new()
        .set_ack(acknowledgement)
        .add_attribute("action", "receive_key1"))   
}

pub fn receive_echo(
    _deps: DepsMut,
    _val: String,
    _view: u32,
) -> StdResult<IbcReceiveResponse> {

    let acknowledgement = to_binary(&AcknowledgementMsg::Ok(EchoResponse { }))?;
    Ok(IbcReceiveResponse::new()
        .set_ack(acknowledgement)
        .add_attribute("action", "receive_echo"))
}

pub fn receive_proof(
    deps: DepsMut,
    _k1: u32,
    _key1_val: String,
    _pk1: i32,
    _view: u32,
) -> StdResult<IbcReceiveResponse> {
    let mut state = STATE.load(deps.storage)?;
    state.current_tx_id += 10;
    STATE.save(deps.storage, &state)?;
    // if view > k1 && k1 as i32 > pk1 && RECEIVED_PROOF.load(deps.storage, k)? {
        // Get the chain_id of the sender
        // let chain_id = CHANNELS.range(&deps.storage, min, max, order)

    // } 
    let response = ProofResponse {};
    let acknowledgement = to_binary(&AcknowledgementMsg::Ok(response))?;
    Ok(IbcReceiveResponse::new()
        .set_ack(acknowledgement)
        .add_attribute("action", "receive_proof"))
}


/*
pub fn queue_receive_suggest(
    _queue_to_process: Vec<Vec<PacketMsg>>,
    deps: DepsMut,
    env: Env,
    from_chain_id: u32,
    view: u32,
    key2: u32,
    key2_val: String,
    prev_key2: i32,
    key3: u32,
    key3_val: String,
) -> StdResult<Vec<Vec<PacketMsg>>> {
    let mut state = STATE.load(deps.storage)?;
    let _acknowledgement = to_binary(&AcknowledgementMsg::Ok(SuggestResponse {}))?;

    // When I'm the primary
    if state.primary == state.chain_id {

        // upon receiving the first suggest message from a chain
        if !RECEIVED_SUGGEST.load(deps.storage, from_chain_id)? {
            RECEIVED_SUGGEST.save(deps.storage, from_chain_id, &true)?;
            // Check if the following conditions hold
            if prev_key2 < key2 as i32 && key2 < view {
                state.key2_proofs.push((key2, key2_val, prev_key2));
                STATE.save(deps.storage, &state)?;
            }
            if key3 == 0 {
                state.suggestions.push((key3, key3_val));
                STATE.save(deps.storage, &state)?;
            } else if key3 < view {
                // Upon accept_key = true
                if accept_key(key3, key3_val.clone(), state.key2_proofs.clone()) {
                    state.suggestions.push((key3, key3_val.clone()));
                    STATE.save(deps.storage, &state)?;
                }
            }

            // Check if |suggestions| >= n - f
            if state.suggestions.len() >= (state.n - F) as usize {
                let _timeout: IbcTimeout = get_timeout(env);
                // Retrive the entry with the largest k
                let (k, v) = state.suggestions.iter().max().unwrap();
                let _propose_packet = PacketMsg::Propose {
                    chain_id: state.chain_id,
                    k: k.clone(),
                    v: v.clone(),
                    view: state.view,
                };
            }
        }
    }
    Ok(Vec::new())

}

pub fn receive_suggest(
    deps: DepsMut,
    env: Env,
    from_chain_id: u32,
    view: u32,
    key2: u32,
    key2_val: String,
    prev_key2: i32,
    key3: u32,
    key3_val: String,
) -> StdResult<IbcReceiveResponse> {
    let mut state = STATE.load(deps.storage)?;
    let acknowledgement = to_binary(&AcknowledgementMsg::Ok(SuggestResponse {}))?;

    // When I'm the primary
    if state.primary == state.chain_id {

        // upon receiving the first suggest message from a chain
        if !RECEIVED_SUGGEST.load(deps.storage, from_chain_id)? {
            RECEIVED_SUGGEST.save(deps.storage, from_chain_id, &true)?;
            // Check if the following conditions hold
            if prev_key2 < key2 as i32 && key2 < view {
                state.key2_proofs.push((key2, key2_val, prev_key2));
                STATE.save(deps.storage, &state)?;
            }
            if key3 == 0 {
                state.suggestions.push((key3, key3_val));
                STATE.save(deps.storage, &state)?;
            } else if key3 < view {
                // Upon accept_key = true
                if accept_key(key3, key3_val.clone(), state.key2_proofs.clone()) {
                    state.suggestions.push((key3, key3_val.clone()));
                    STATE.save(deps.storage, &state)?;
                }
            }

            // Check if |suggestions| >= n - f
            if state.suggestions.len() >= (state.n - F).try_into().unwrap() {
                let timeout = get_timeout(env);
                // Retrive the entry with the largest k
                let (k, v) = state.suggestions.iter().max().unwrap();
                let propose_packet = PacketMsg::Propose {
                    chain_id: state.chain_id,
                    k: k.clone(),
                    v: v.clone(),
                    view: state.view,
                };

                return Ok(IbcReceiveResponse::new()
                    .set_ack(acknowledgement)
                    .add_submessages(send_all_upon_join(&deps, timeout, propose_packet).unwrap())
                    .add_attribute("action", "receive_suggest")
                    .add_attribute("suggest_sender_chain_id", from_chain_id.to_string()));
            }
            
        }
        
    }
    // let acknowledgement = to_binary(&AcknowledgementMsg::Ok(SuggestResponse {}))?;
    Ok(IbcReceiveResponse::new()
        .set_ack(acknowledgement)
        .add_attribute("action", "receive_suggest")
        .add_attribute("suggest_sender_chain_id", from_chain_id.to_string()))
}
*/

fn _accept_key(key: u32, value: String, proofs: Vec<(u32, String, i32)>) -> bool {
    let mut supporting = 0;
    for (k, v, pk) in proofs {
        if (key as i32) < pk {
            supporting += 1;
        } else if key <= k && value == v {
            supporting += 1;
        }
    }
    if supporting >= 1 + 1 {
        return true;
    }
    false
}

// pub fn receive_request(
//     deps: DepsMut,
//     _caller: String,
//     view: u32,
//     chain_id: u32,
// ) -> StdResult<IbcReceiveResponse> {
//     let mut state = STATE.load(deps.storage)?;
//     state.key2_proofs.push((state.current_tx_id,"received_request".to_string(), chain_id as i32));
//     state.current_tx_id += 1;
//     STATE.save(deps.storage, &state)?;
//     // Update stored highest_request for that blockchain accordingly
//     let highest_request = HIGHEST_REQ.load(deps.storage, chain_id)?;
//     if highest_request < view {
//         HIGHEST_REQ.save(deps.storage, chain_id, &view)?;
//     }

//     let acknowledgement = to_binary(&AcknowledgementMsg::Ok(RequestResponse {}))?;

//     Ok(IbcReceiveResponse::new()
//         .set_ack(acknowledgement)
//         .add_attribute("action", "receive_request")
//         .add_attribute("chain_id", chain_id.to_string()))
// }

fn _open_lock(deps: &DepsMut, proofs: Vec<(u32, String, i32)>) -> StdResult<bool> {
    let mut supporting: u32 = 0;
    let state = STATE.load(deps.storage)?;
    for (k, v, pk) in proofs {
        if (state.lock as i32) <= pk {
            supporting += 1;
        } else if state.lock <= k && v != state.lock_val {
            supporting += 1;
        }
    }
    if supporting >= F + 1 {
        Ok(true)
    } else {
        Ok(false)
    }
}


/* 
pub fn receive_wrapper(
    msgs: Vec<SubMsg>,
    receive_type: String
) -> StdResult<IbcReceiveResponse> {
    let res = IbcReceiveResponse::new();
    Ok(res)
}

pub fn queue_receive_propose(
    deps: DepsMut,
    _caller: String,
    timeout: IbcTimeout,
    chain_id: u32,
    k: u32,
    v: String,
    view: u32,
) -> StdResult<Vec<SubMsg>> {
    let mut state = STATE.load(deps.storage)?;
    // let mut send_msg = false;
    let mut msgs: Vec<SubMsg> = Vec::new();
    // ignore messages from other views, other than abort, done and request messages
    if view != state.view {
    } else {
        // upon receiving the first propose message from a chain
        if chain_id == state.primary && state.received_propose {
            // RECEIVED_PROPOSE.save(deps.storage, chain_id, &true)?;
            state.received_propose = false;
            STATE.save(deps.storage, &state)?;
            
            // First case we should broadcast Echo message
            if state.lock == 0 || v == state.lock_val {
                let echo_packet = PacketMsg::Echo { val: v, view };
                msgs.extend(send_all_upon_join(&deps, timeout.clone(), echo_packet).unwrap());
            

            } else if view > k && k >= state.lock {
                // upon open_lock(proofs) == true
                // Second case we should broadcast Echo message
                if open_lock(&deps, state.proofs)? {
                    let echo_packet = PacketMsg::Echo { val: v, view };
                    msgs.extend(send_all_upon_join(&deps, timeout.clone(), echo_packet).unwrap());
                }
            }
        }
    }

    // specify the type of AcknowledgementMsg to be ProposeResponse
    let acknowledgement = to_binary(&AcknowledgementMsg::Ok(ProposeResponse {}))?;
    let _res: IbcReceiveResponse = IbcReceiveResponse::new()
        .set_ack(acknowledgement)
        .add_attribute("action", "receive_propose");
    // send back acknowledgement, containing the response info
    Ok(msgs)
}

pub fn receive_propose(
    deps: DepsMut,
    _caller: String,
    timeout: IbcTimeout,
    chain_id: u32,
    k: u32,
    v: String,
    view: u32,
) -> StdResult<IbcReceiveResponse> {
    let mut state = STATE.load(deps.storage)?;
    // let mut send_msg = false;
    let mut msgs: Vec<SubMsg> = Vec::new();
    // ignore messages from other views, other than abort, done and request messages
    if view != state.view {
    } else {
        // upon receiving the first propose message from a chain
        if chain_id == state.primary && state.received_propose {
            // RECEIVED_PROPOSE.save(deps.storage, chain_id, &true)?;
            state.received_propose = false;
            STATE.save(deps.storage, &state)?;
            
            // First case we should broadcast Echo message
            if state.lock == 0 || v == state.lock_val {
                let echo_packet = PacketMsg::Echo { val: v, view };
                msgs.extend(send_all_upon_join(&deps, timeout.clone(), echo_packet).unwrap());
            

            } else if view > k && k >= state.lock {
                // upon open_lock(proofs) == true
                // Second case we should broadcast Echo message
                if open_lock(&deps, state.proofs)? {
                    let echo_packet = PacketMsg::Echo { val: v, view };
                    msgs.extend(send_all_upon_join(&deps, timeout.clone(), echo_packet).unwrap());
                }
            }
        }
    }

    // specify the type of AcknowledgementMsg to be ProposeResponse
    let acknowledgement = to_binary(&AcknowledgementMsg::Ok(ProposeResponse {}))?;
    let res = IbcReceiveResponse::new()
        .set_ack(acknowledgement)
        .add_attribute("action", "receive_propose");
    // send back acknowledgement, containing the response info
    if msgs.is_empty() {
        Ok(res)
    } else {
        Ok(res.add_submessages(msgs))
    }
}
*/

// processes PacketMsg::WhoAmI
fn receive_who_am_i(
    deps: DepsMut,
    channel_id: String,
    chain_id: u32,
) -> StdResult<IbcReceiveResponse> {
    let action = |_| -> StdResult<String> { Ok(channel_id.to_string()) };
    CHANNELS.update(deps.storage, chain_id, action)?;

    // initialize the highest_request of that chain
    // let action = |_| -> StdResult<u32> { Ok(0) };
    // HIGHEST_REQ.update(deps.storage, chain_id, action)?;
    // initialize the highest_request of that chain
    // HIGHEST_ABORT.save(deps.storage, chain_id, &0)?;

    let response = WhoAmIResponse {};
    let acknowledgement = to_binary(&AcknowledgementMsg::Ok(response))?;
    // and we are golden
    Ok(IbcReceiveResponse::new()
        .set_ack(acknowledgement)
        .add_attribute("action", "receive_who_am_i")
        .add_attribute("chain_id", chain_id.to_string()))
}

#[entry_point]
pub fn ibc_packet_ack(
    _deps: DepsMut,
    _env: Env,
    msg: IbcPacketAckMsg,
) -> StdResult<IbcBasicResponse> {
    let packet: PacketMsg = from_slice(&msg.original_packet.data)?;
    match packet {
        PacketMsg::MsgQueue(_q) => Ok(IbcBasicResponse::new()),
/*
        PacketMsg::Propose {
            chain_id: _,
            k: _,
            v: _,
            view: _,
        } => {
            // let res: AcknowledgementMsg<ProposeResponse> = ;
            acknowledge_propose(deps, env, from_slice(&msg.acknowledgement.data)?)
        }
        // PacketMsg::Commit { msg: _, tx_id: _ } => Ok(IbcBasicResponse::new()),
*/
        PacketMsg::WhoAmI { chain_id: _ } => Ok(IbcBasicResponse::new()),
/*
        PacketMsg::Request {
            chain_id: _,
            view: _,
        } => Ok(IbcBasicResponse::new()),
        // acknowledge_request(deps, env),
        PacketMsg::Suggest {
            view: _,
            key2: _,
            key2_val: _,
            prev_key2: _,
            key3: _,
            key3_val: _,
            chain_id: _,
        } => Ok(IbcBasicResponse::new()),
        PacketMsg::Proof {
            key1: _,
            key1_val: _,
            prev_key1: _,
            view: _,
        } => Ok(IbcBasicResponse::new()),
        PacketMsg::Echo { val: _, view: _ } => Ok(IbcBasicResponse::new()),
        PacketMsg::Key1 { val: _, view: _ } => Ok(IbcBasicResponse::new()),
        PacketMsg::Key2 { val: _, view: _ } => Ok(IbcBasicResponse::new()),
        PacketMsg::Key3 { val: _, view: _ } => Ok(IbcBasicResponse::new()),
        PacketMsg::Lock { val: _, view: _ } => Ok(IbcBasicResponse::new()),
        PacketMsg::Done { val: _ } => Ok(IbcBasicResponse::new()),
        PacketMsg::Abort { view, chain_id } => Ok(IbcBasicResponse::new()),
         */
    }
}
/*
fn _acknowledge_request(
    deps: DepsMut,
    env: Env,
) -> StdResult<IbcBasicResponse> {
    
    // Upon sucessfully called the broadcast of Request Messages
    // Load the state 
    let mut state = STATE.load(deps.storage)?;
    if !state.is_first_req_ack {
        return Ok(IbcBasicResponse::new());
    }
    state.is_first_req_ack = false;
    STATE.save(deps.storage, &state)?;
    if state.chain_id != state.primary {
        // Upon highest_request[primary] = view
        let prim_highest_req = HIGHEST_REQ.load(deps.storage, state.primary)?;
        if prim_highest_req == state.view {
            // Contruct Suggest message to delivery to primary
            let packet = PacketMsg::Suggest {
                chain_id: state.chain_id,
                view: state.view,
                key2: state.key2,
                key2_val: state.key2_val.clone(),
                prev_key2: state.prev_key2,
                key3: state.key3,
                key3_val: state.key3_val.clone(),
            };
            // let timeout: IbcTimeout = env.block.time.plus_seconds(PACKET_LIFETIME).into();
            let channel_id = CHANNELS.load(deps.storage, state.primary)?;
            let timeout = get_timeout(env);
            let msg = IbcMsg::SendPacket {
                channel_id,
                data: to_binary(&packet)?,
                timeout: timeout.clone(),
            };
            let submsg = SubMsg::reply_on_success(msg, SUGGEST_REPLY_ID);
            // let submsg = msg;
            // construct Response and put Suggest message in the query on the fly
            return Ok(IbcBasicResponse::new()
                .add_submessage(submsg)
                .add_attribute("action", "send_suggest2primary".to_string()))
        }
    }
    Ok(IbcBasicResponse::new())
}

fn acknowledge_propose(
    _deps: DepsMut,
    env: Env,
    ack: AcknowledgementMsg<ProposeResponse>,
) -> StdResult<IbcBasicResponse> {
    let _timeout: IbcTimeout = get_timeout(env);
    // retrive tx_id from acknowledge message
    let _tx_id = match ack {
        AcknowledgementMsg::Ok(res) => res,
        AcknowledgementMsg::Err(e) => {
            return Ok(IbcBasicResponse::new()
                .add_attribute("action", "acknowledge_propose")
                .add_attribute("error", e))
        }
    };
    // let action = |tx: Option<Tx>| -> StdResult<Tx> {
    //     let mut tx = tx.unwrap();
    //     tx.no_of_votes += 1;
    //     Ok(tx)
    // };

    // let tx = TXS.update(deps.storage, tx_id.clone(), action)?;

    // broadcast Commit message
    // if tx.no_of_votes >= 2 {
    //     // let state: State = STATE.load(deps.storage)?;
    //     // let channel_ids = state.channel_ids.clone();
    //     let channel_ids: StdResult<Vec<_>> = CHANNELS
    //         .range(deps.storage, None, None, Order::Ascending)
    //         .collect();
    //     let channel_ids = channel_ids?;
    //     let packet = PacketMsg::Commit {
    //         msg: tx.msg.clone(),
    //         tx_id: tx_id.clone(),
    //     };

    //     receive_commit(deps, "self".to_string(), tx.msg.clone(), tx_id.clone())?;

    //     // Broadcast Commit messages
    //     let mut commit_msgs: Vec<IbcMsg> = Vec::new();
    //     for (_, channel_id) in channel_ids {
    //         let msg = IbcMsg::SendPacket {
    //             channel_id: channel_id.clone(),
    //             data: to_binary(&packet)?,
    //             timeout: timeout.clone(),
    //         };
    //         commit_msgs.push(msg);
    //     }

    // let msg0 = IbcMsg::SendPacket {
    //     channel_id: channel_ids[0].clone(),
    //     data: to_binary(&packet)?,
    //     timeout: timeout.clone()
    // };
    // let msg1 = IbcMsg::SendPacket {
    //     channel_id: channel_ids[1].clone(),
    //     data: to_binary(&packet)?,
    //     timeout: timeout.clone()
    // };

    //     Ok(IbcBasicResponse::new()
    //         // .add_message(msg0)
    //         // .add_message(msg1)
    //         .add_messages(commit_msgs)
    //         .add_attribute("action", "acknowledge_propose_response")
    //         .add_attribute("commit", "true"))
    // } else {
    Ok(IbcBasicResponse::new()
        .add_attribute("action", "acknowledge_propose_response")
        .add_attribute("commit", "false"))
    // }
}
*/

#[entry_point]
/// This will never be called
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
    use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
    use crate::utils::IBC_APP_VERSION;

    use cosmwasm_std::testing::{
        mock_dependencies, mock_env, mock_ibc_channel_connect_ack, mock_ibc_channel_open_init,
        mock_ibc_channel_open_try, mock_ibc_packet_ack, mock_info, MockApi, MockQuerier,
        MockStorage,
    };
    use cosmwasm_std::{coins, CosmosMsg, OwnedDeps, IbcOrder};

    fn setup() -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            // role: "leader".to_string(),
            chain_id: 0,
            input: 0.to_string(),
        };
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
