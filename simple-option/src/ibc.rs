use std::convert::TryInto;

use cosmwasm_std::{
    entry_point, from_slice, to_binary, Binary, DepsMut, Env, Event, IbcTimeout, Order, Response,
    StdError, StdResult, SubMsg,
};
use cosmwasm_std::{
    IbcBasicResponse, IbcChannelCloseMsg, IbcChannelConnectMsg, IbcChannelOpenMsg, IbcMsg,
    IbcOrder, IbcPacketAckMsg, IbcPacketReceiveMsg, IbcPacketTimeoutMsg, IbcReceiveResponse,
};

use crate::contract::{REQUEST_REPLY_ID, SUGGEST_REPLY_ID, PROPOSE_REPLY_ID};
use crate::ibc_msg::{
    AcknowledgementMsg, CommitResponse, PacketMsg, ProposeResponse, RequestResponse,
    SuggestResponse, WhoAmIResponse, ProofResponse, MsgQueueResponse, EchoResponse, Key1Response, Key2Response, Key3Response, LockResponse, DoneResponse,
};
use crate::msg::{ExecuteMsg};
use crate::state::{
    CHANNELS, HIGHEST_ABORT, HIGHEST_REQ, RECEIVED_SUGGEST, STATE, VARS, SEND_ALL_UPON, ECHO, RECEIVED_PROOF, TEST_QUEUE, TEST, KEY1, KEY2, KEY3, LOCK, DONE,
};

use crate::ContractError;

pub const IBC_APP_VERSION: &str = "simple_storage";

/// Setting the lifetime of packets to be one hour
pub const PACKET_LIFETIME: u64 = 60 * 60;
/// Setting up constant
pub const F: u32 = 1;


pub fn view_change(deps: DepsMut, timeout: IbcTimeout) -> Result<Response, ContractError> {

    let msgs = create_queue_view_change(deps, timeout)?;
    Ok(Response::new()
        .add_messages(msgs)
        .add_attribute("action", "execute")
        .add_attribute("msg_type", "input"))

    // Contruct Request messages to be broadcasted ----latest1
    // let res = broadcast_submsgs("broadcast_request".to_string(), timeout.clone(), state.channel_ids.clone(), request_packet)?;
    // return Ok(res);

    // Contruct Request messages to be broadcasted and add to Response ----latest2
    // let mut res = add_broadcast_submsgs(Response::new(), timeout.clone(), state.channel_ids[0..1].to_vec(), request_packet.clone(), REQUEST_REPLY_ID)?;
    // if state.channel_ids.len() > 1 {
    //     res = add_broadcast_msgs(res, timeout, state.channel_ids[1..].to_vec(), request_packet)?;
    // }
    
    // Ok(res.add_attribute("action", "broadcast_request".to_string()))

    // if state.chain_id != state.primary {
    //     // Upon highest_request[primary] = view
    //     let prim_highest_req = HIGHEST_REQ.load(deps.storage, state.primary)?;
    //     if prim_highest_req == state.view {
    //         // Contruct Suggest message to delivery to primary
    //         let packet = PacketMsg::Suggest {
    //             chain_id: state.chain_id,
    //             view: state.view,
    //             key2: state.key2,
    //             key2_val: state.key2_val.clone(),
    //             prev_key2: state.prev_key2,
    //             key3: state.key3,
    //             key3_val: state.key3_val.clone(),
    //         };
    //         let channel_id = CHANNELS.load(deps.storage, state.primary)?;
    //         res = res.add_message(IbcMsg::SendPacket {
    //                 channel_id,
    //                 data: to_binary(&packet)?,
    //                 timeout: timeout.clone(),
    //             });
            // msgs.push(IbcMsg::SendPacket {
            //     channel_id,
            //     data: to_binary(&packet)?,
            //     timeout: timeout.clone(),
            // });

            // msgs.extend(suggest_msg);
        // }
    
    // }

    // let all_msgs = send_all_upon_join(&deps, timeout, msgs, proof_packet)?;
    // TEST.save(deps.storage, msgs.is_empty().to_string(), &msgs);
    // Ok(res)
}

pub fn create_queue_view_change(
    deps: DepsMut,
    timeout: IbcTimeout,
) -> Result<Vec<IbcMsg>, ContractError> {
    // load the state
    let mut state = STATE.load(deps.storage)?;
    // Add Request message to packets_to_be_broadcasted
    let request_packet = PacketMsg::Request {
        view: state.view,
        chain_id: state.chain_id,
    };

    // Contruct Request messages to be broadcasted
    let channels = get_id_channel_pair(&deps)?;
    let proof_packet = PacketMsg::Proof {
        key1: state.key1,
        key1_val: state.key1_val.clone(),
        prev_key1: state.prev_key1,
        view: state.view,
    };
    let mut msgs: Vec<IbcMsg> = Vec::new();

    for (chain_id, channel_id) in &channels {
        // construct the msg queue to send
        let mut queue = vec![request_packet.clone()];
        let highest_request = HIGHEST_REQ.load(deps.storage, chain_id.clone())?;
        // If dest chain is primary, check if satisfiy condition
        if chain_id.clone() == state.primary && state.chain_id != state.primary {
            // Upon highest_request[primary] = view
            if highest_request == state.view && !state.sent_suggest {
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
                state.sent_suggest = true;
                STATE.save(deps.storage, &state)?;
                queue.push(packet);
            }
        }
        // send_all_upon_join(proof)
        if highest_request == state.view {
            queue.push(proof_packet.clone());
        }

        let msg = IbcMsg::SendPacket {
            channel_id: channel_id.clone(),
            data: to_binary(&PacketMsg::MsgQueue { q: queue })?,
            timeout: timeout.clone(),
        };
        msgs.push(msg);
    }
    
    Ok(msgs)
    
    // // construct Response and put Suggest message in the query on the fly
    // return Ok(Response::new()
    // .add_submessage(submsg)
    // .add_attribute("action", "send_suggest2primary".to_string()))
}

// pub fn create_msgs_view_change(
//     deps: &DepsMut,
//     timeout: IbcTimeout,
//     packet_to_broadcast: PacketMsg,
// ) -> Result<Vec<IbcMsg>, ContractError> {

    
// }

// }

pub fn get_id_channel_pair(deps: &DepsMut) -> StdResult<Vec<(u32, String)>> {
    let channels: StdResult<Vec<_>> = CHANNELS
        .range(deps.storage, None, None, Order::Ascending)
        .collect();
    channels
}

// pub fn send_all_upon_join_queue(
//     deps: &DepsMut,
//     packet_to_broadcast: PacketMsg,
//     queue: &mut Vec<Vec<PacketMsg>>
// ) -> Result<(), ContractError> {
//     let channel_ids = get_id_channel_pair(&deps)?;
//     let state = STATE.load(deps.storage)?;
//     for (chain_id, _channel_id) in &channel_ids {
//         let highest_request = HIGHEST_REQ.load(deps.storage, chain_id.clone())?;
//         if highest_request == state.view {
//             queue[*chain_id as usize].push(packet_to_broadcast.clone());
//         }
//         else{
//             let action = |packets: Option<Vec<PacketMsg>>| -> StdResult<Vec<PacketMsg>> {
//                 match packets {
//                     Some(_) => todo!(),
//                     None => Ok(vec!(packet_to_broadcast.clone())),
//                 }
                
//             };
//             SEND_ALL_UPON.update(deps.storage, *chain_id, action)?;
//         }
//     }
//     Ok(())
// }

pub fn send_all_upon_join_sub(
    deps: &DepsMut,
    timeout: IbcTimeout,
    mut res: Response,
    packet_to_broadcast: PacketMsg,
    reply_id: u64
) -> Result<Response, ContractError> {
    let channel_ids = get_id_channel_pair(&deps)?;
    // let mut res = res;
    let state = STATE.load(deps.storage)?;
    for (chain_id, channel_id) in &channel_ids {
        let highest_request = HIGHEST_REQ.load(deps.storage, chain_id.clone())?;
        if highest_request == state.view {
            let msg = IbcMsg::SendPacket {
                channel_id: channel_id.clone(),
                data: to_binary(&packet_to_broadcast)?,
                timeout: timeout.clone(),
            };
            let submsg = SubMsg::reply_on_success(msg, reply_id);
            res = res.add_submessage(submsg);
        }
    }

    Ok(res)
}

pub fn send_all_upon_join(
    deps: &DepsMut,
    timeout: IbcTimeout,
    packet_to_broadcast: PacketMsg,
) -> Result<Vec<SubMsg>, ContractError> {
    let channel_ids = get_id_channel_pair(&deps)?;

    let mut msgs = Vec::new();
    let state = STATE.load(deps.storage)?;
    for (chain_id, channel_id) in &channel_ids {
        let highest_request = HIGHEST_REQ.load(deps.storage, chain_id.clone())?;
        if highest_request == state.view {
            let msg = IbcMsg::SendPacket {
                channel_id: channel_id.clone(),
                data: to_binary(&packet_to_broadcast)?,
                timeout: timeout.clone(),
            };
            let submsg: SubMsg = SubMsg::reply_on_success(msg, PROPOSE_REPLY_ID);
            msgs.push(submsg);
        }
    }

    Ok(msgs)
}

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

pub fn get_timeout(env: Env) -> IbcTimeout {
    env.block.time.plus_seconds(PACKET_LIFETIME).into()
}


pub fn receive_queue(
    deps: DepsMut,
    env: Env,
    local_channel_id: String,
    queue_to_process: Vec<PacketMsg>
) -> StdResult<IbcReceiveResponse> {
    let state = STATE.load(deps.storage)?;
    let mut queue: Vec<Vec<PacketMsg>> = vec!(Vec::new(); state.n.try_into().unwrap());
    for msg in queue_to_process {
        let result: StdResult<()> = match msg {
            PacketMsg::MsgQueue { q: _q } => todo!(),
            PacketMsg::Propose {
                chain_id,
                k,
                v,
                view,
            } => 
                // Handle Propose
                {
                    let mut state = STATE.load(deps.storage)?;
                    // ignore messages from other views, other than abort, done and request messages
                    if view != state.view {
                    } else {
                        // upon receiving the first propose message from a chain
                        if chain_id == state.primary && state.is_first_propose {
                            // RECEIVED_PROPOSE.save(deps.storage, chain_id, &true)?;
                            let mut broadcast = false;
                            state.is_first_propose = false;
                            STATE.save(deps.storage, &state)?;
                            
                            // First case we should broadcast Echo message
                            if state.lock == 0 || v == state.lock_val {
                                broadcast = true;
                            } else if view > k && k >= state.lock {
                                // upon open_lock(proofs) == true
                                // Second case we should broadcast Echo message
                                if open_lock(&deps, state.proofs)? {
                                    broadcast = true;
                                }
                            }
                            // send_all_upon_join_queue(<propose, k, v, view>)
                            if broadcast {
                                let echo_packet = PacketMsg::Echo { val: v, view };
                                let channel_ids = get_id_channel_pair(&deps)?;
                                let state = STATE.load(deps.storage)?;
                                for (chain_id, _channel_id) in &channel_ids {
                                    let highest_request = HIGHEST_REQ.load(deps.storage, chain_id.clone())?;
                                    if highest_request == state.view {
                                        queue[*chain_id as usize].push(echo_packet.clone());
                                    }
                                    // Otherwise, we need the msg to be recorded in queue so that it could be triggered when condition satisfies
                                    else{
                                        let action = |packets: Option<Vec<PacketMsg>>| -> StdResult<Vec<PacketMsg>> {
                                            match packets {
                                                Some(mut p) => {
                                                    p.push(echo_packet.clone());
                                                    Ok(p)
                                                },
                                                None => Ok(vec!(echo_packet.clone())),
                                            }
                                            
                                        };
                                        SEND_ALL_UPON.update(deps.storage, *chain_id, action)?;
                                    }
                                }
                            }
                            // send_all_upon_join_queue(<propose, k, v, view>)/

                        }
                    }
                    Ok(())

                },
            PacketMsg::WhoAmI { chain_id: _ } => Ok(()),
            // PacketMsg::Commit { msg, tx_id } => receive_commit(deps, dest_channel_id, msg, tx_id),
            PacketMsg::Request { view, chain_id } => 
                
                // Handle Request
                {
                    let mut state = STATE.load(deps.storage)?;
                    state.key2_proofs.push((state.current_tx_id,"received_request".to_string(), chain_id as i32));
                    STATE.save(deps.storage, &state)?;
                    // Update stored highest_request for that blockchain accordingly
                    let highest_request = HIGHEST_REQ.load(deps.storage, chain_id)?;
                    if highest_request < view {
                        HIGHEST_REQ.save(deps.storage, chain_id, &view)?;
                            
                        if view == state.view {
                            // Check if we are ready to send Suggest to Primary
                            if chain_id == state.primary && !state.sent_suggest {
                                let packet = PacketMsg::Suggest {
                                    chain_id: state.chain_id,
                                    view: state.view,
                                    key2: state.key2,
                                    key2_val: state.key2_val.clone(),
                                    prev_key2: state.prev_key2,
                                    key3: state.key3,
                                    key3_val: state.key3_val.clone(),
                                };
                                state.sent_suggest = true;
                                STATE.save(deps.storage, &state)?;
                                queue[chain_id as usize].push(packet);
                            }

                            // Check if any pending send_all_upon_join
                            let packets = SEND_ALL_UPON.may_load(deps.storage, chain_id)?;
                            match packets {
                                Some(p) => {
                                    // Add to queue and remove from the buffer
                                    queue[chain_id as usize].extend(p);
                                    SEND_ALL_UPON.remove(deps.storage, chain_id);

                                },
                                None => (),
                            };
                        }
                    }
                    Ok(())
                },
            PacketMsg::Suggest {
                chain_id,
                view,
                key2,
                key2_val,
                prev_key2,
                key3,
                key3_val,
            } => 

            // Handle Suggest msg within MsgQueue
            {
                let mut state = STATE.load(deps.storage)?;

                // When I'm the primary
                if state.primary == state.chain_id {

                    // upon receiving the first suggest message from a chain
                    if !RECEIVED_SUGGEST.load(deps.storage, chain_id)? {
                        RECEIVED_SUGGEST.save(deps.storage, chain_id, &true)?;
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
                            // Retrive the entry with the largest k
                            let (k, v) = state.suggestions.iter().max().unwrap();
                            let propose_packet = PacketMsg::Propose {
                                chain_id: state.chain_id,
                                k: k.clone(),
                                v: v.clone(),
                                view: state.view,
                            };

                            // send_all_upon_join_queue(<propose, k, v, view>)
                            let channel_ids = get_id_channel_pair(&deps)?;
                            // let state = STATE.load(deps.storage)?;
                            for (chain_id, _channel_id) in &channel_ids {
                                let highest_request = HIGHEST_REQ.load(deps.storage, chain_id.clone())?;
                                if highest_request == state.view {
                                    queue[*chain_id as usize].push(propose_packet.clone());

                                }
                                // Otherwise, we need the msg to be recorded in queue so that it could be triggered when condition satisfies
                                else {
                                    let action = |packets: Option<Vec<PacketMsg>>| -> StdResult<Vec<PacketMsg>> {
                                        match packets {
                                            Some(mut p) => {
                                                p.push(propose_packet.clone());
                                                Ok(p)
                                            },
                                            None => Ok(vec!(propose_packet.clone())),
                                        }
                                        
                                    };
                                    SEND_ALL_UPON.update(deps.storage, *chain_id, action)?;
                                }
                            }

                        }
                        
                    }
                }

                Ok(())
    
            },

            PacketMsg::Proof {
                key1,
                key1_val,
                prev_key1,
                view,
            } => 

            // Handle Proof
            {
                // Get the chain_id of the sender
                let chain_id_res = CHANNELS
                .range(deps.storage, None, None, Order::Ascending)
                .find_map(|res| { 
                    let (chain_id,channel_id) = res.unwrap(); 
                    if channel_id == local_channel_id 
                    { 
                        Some(chain_id) 
                    } 
                    else { None }
                });
                let chain_id = chain_id_res.unwrap();
                // Update the state
                RECEIVED_PROOF.save(deps.storage, chain_id, &true)?;


                //// TESTING ////
                let mut state = STATE.load(deps.storage)?;
                state.proofs.push((key1, key1_val.clone(), prev_key1));
                STATE.save(deps.storage, &state)?;

                // if condition is met, update the proofs accordingly
                if view > key1 && key1 as i32 > prev_key1 && !RECEIVED_PROOF.load(deps.storage, chain_id)? {
                    let mut state = STATE.load(deps.storage)?;
                    state.proofs.push((key1, key1_val, prev_key1));
                    STATE.save(deps.storage, &state)?;
                } 
                Ok(())
            },
            PacketMsg::Echo { val, view } => 
            

            // Handle Echo
            {
                let mut state = STATE.load(deps.storage)?;
                // ignore messages from other views, other than abort, done and request messages
                if view != state.view {
                } else {
                    // Update local record of echo messages
                    let action = |count: Option<u32>| -> StdResult<u32> {
                        match count {
                            Some(c) => Ok(c + 1),
                            None => Ok(1),
                        }
                    };
                    let count = ECHO.update(deps.storage, val.clone(), action)?;

                    // upon receiving from n - f parties with the same val
                    if count >= state.n - F {

                        // send_all_upon_join_queue(<key1, val, view>)
                        let key1_packet = PacketMsg::Key1 { val: val.clone(), view };
                        let channel_ids = get_id_channel_pair(&deps)?;

                        for (chain_id, _channel_id) in &channel_ids {
                            let highest_request = HIGHEST_REQ.load(deps.storage, chain_id.clone())?;
                            if highest_request == state.view {
                                queue[*chain_id as usize].push(key1_packet.clone());
                            }
                            // Otherwise, we need the msg to be recorded in queue so that it could be triggered when condition satisfies
                            else{
                                let action = |packets: Option<Vec<PacketMsg>>| -> StdResult<Vec<PacketMsg>> {
                                    match packets {
                                        Some(mut p) => {
                                            p.push(key1_packet.clone());
                                            Ok(p)
                                        },
                                        None => Ok(vec!(key1_packet.clone())),
                                    }
                                    
                                };
                                SEND_ALL_UPON.update(deps.storage, *chain_id, action)?;
                            }
                        }
                        // send_all_upon_join_queue(<key1, val, view>)/

                        if state.key1_val != val {
                            state.prev_key1 = state.key1 as i32;
                            state.key1_val = val;
                            
                        }
                        state.key1 = view;
                        STATE.save(deps.storage, &state)?;
                    
                    }

                }
                Ok(())

            },
            PacketMsg::Key1 { val, view } => 

            // Handle Key1
            {
                let mut state = STATE.load(deps.storage)?;
                // ignore messages from other views, other than abort, done and request messages
                if view != state.view {
                } else {
                    // Update local record of key1 messages
                    let action = |count: Option<u32>| -> StdResult<u32> {
                        match count {
                            Some(c) => Ok(c + 1),
                            None => Ok(1),
                        }
                    };
                    let count = KEY1.update(deps.storage, val.clone(), action)?;

                    // upon receiving from n - f parties with the same val
                    if count >= state.n - F {

                        // send_all_upon_join_queue(<key2, val, view>)
                        let key2_packet = PacketMsg::Key2 { val: val.clone(), view };
                        let channel_ids = get_id_channel_pair(&deps)?;

                        for (chain_id, _channel_id) in &channel_ids {
                            let highest_request = HIGHEST_REQ.load(deps.storage, chain_id.clone())?;
                            if highest_request == state.view {
                                queue[*chain_id as usize].push(key2_packet.clone());
                            }
                            // Otherwise, we need the msg to be recorded in queue so that it could be triggered when condition satisfies
                            else{
                                let action = |packets: Option<Vec<PacketMsg>>| -> StdResult<Vec<PacketMsg>> {
                                    match packets {
                                        Some(mut p) => {
                                            p.push(key2_packet.clone());
                                            Ok(p)
                                        },
                                        None => Ok(vec!(key2_packet.clone())),
                                    }
                                    
                                };
                                SEND_ALL_UPON.update(deps.storage, *chain_id, action)?;
                            }
                        }
                        // send_all_upon_join_queue(<key2, val, view>)/

                        if state.key2_val != val {
                            state.prev_key2 = state.key2 as i32;
                            state.key2_val = val;
                            
                        }
                        state.key2 = view;
                        STATE.save(deps.storage, &state)?;
                    }
                }
                Ok(())

            },
            PacketMsg::Key2 { val, view } => 

            // Handle Key2
            {
                let mut state = STATE.load(deps.storage)?;
                // ignore messages from other views, other than abort, done and request messages
                if view != state.view {
                } else {
                    // Update local record of key2 messages
                    let action = |count: Option<u32>| -> StdResult<u32> {
                        match count {
                            Some(c) => Ok(c + 1),
                            None => Ok(1),
                        }
                    };
                    let count = KEY2.update(deps.storage, val.clone(), action)?;

                    // upon receiving from n - f parties with the same val
                    if count >= state.n - F {

                        // send_all_upon_join_queue(<key3, val, view>)
                        let key3_packet = PacketMsg::Key3 { val: val.clone(), view };
                        let channel_ids = get_id_channel_pair(&deps)?;

                        for (chain_id, _channel_id) in &channel_ids {
                            let highest_request = HIGHEST_REQ.load(deps.storage, chain_id.clone())?;
                            if highest_request == state.view {
                                queue[*chain_id as usize].push(key3_packet.clone());
                            }
                            // Otherwise, we need the msg to be recorded in queue so that it could be triggered when condition satisfies
                            else{
                                let action = |packets: Option<Vec<PacketMsg>>| -> StdResult<Vec<PacketMsg>> {
                                    match packets {
                                        Some(mut p) => {
                                            p.push(key3_packet.clone());
                                            Ok(p)
                                        },
                                        None => Ok(vec!(key3_packet.clone())),
                                    }
                                    
                                };
                                SEND_ALL_UPON.update(deps.storage, *chain_id, action)?;
                            }
                        }
                        // send_all_upon_join_queue(<key3, val, view>)/
                        
                        state.key3 = view;
                        state.key3_val = val;
                        STATE.save(deps.storage, &state)?;
                    }
                }
                Ok(())

            },
            PacketMsg::Key3 { val, view } => 

            // Handle Key3
            {
                let mut state = STATE.load(deps.storage)?;
                // ignore messages from other views, other than abort, done and request messages
                if view != state.view {
                } else {
                    // Update local record of key3 messages
                    let action = |count: Option<u32>| -> StdResult<u32> {
                        match count {
                            Some(c) => Ok(c + 1),
                            None => Ok(1),
                        }
                    };
                    let count = KEY3.update(deps.storage, val.clone(), action)?;

                    // upon receiving from n - f parties with the same val
                    if count >= state.n - F {

                        // send_all_upon_join_queue(<lock, val, view>)
                        let lock_packet = PacketMsg::Lock { val: val.clone(), view };
                        let channel_ids = get_id_channel_pair(&deps)?;

                        for (chain_id, _channel_id) in &channel_ids {
                            let highest_request = HIGHEST_REQ.load(deps.storage, chain_id.clone())?;
                            if highest_request == state.view {
                                queue[*chain_id as usize].push(lock_packet.clone());
                            }
                            // Otherwise, we need the msg to be recorded in queue so that it could be triggered when condition satisfies
                            else{
                                let action = |packets: Option<Vec<PacketMsg>>| -> StdResult<Vec<PacketMsg>> {
                                    match packets {
                                        Some(mut p) => {
                                            p.push(lock_packet.clone());
                                            Ok(p)
                                        },
                                        None => Ok(vec!(lock_packet.clone())),
                                    }
                                    
                                };
                                SEND_ALL_UPON.update(deps.storage, *chain_id, action)?;
                            }
                        }
                        // send_all_upon_join_queue(<lock, val, view>)/
                        
                        state.lock = view;
                        state.lock_val = val;
                        STATE.save(deps.storage, &state)?;
                    }
                }
                Ok(())

            },
            PacketMsg::Lock { val, view } => 
            
            // Handle Lock
            {
                let mut state = STATE.load(deps.storage)?;
                // ignore messages from other views, other than abort, done and request messages
                if view != state.view {
                } else {
                    // Update local record of lock messages
                    let action = |count: Option<u32>| -> StdResult<u32> {
                        match count {
                            Some(c) => Ok(c + 1),
                            None => Ok(1),
                        }
                    };
                    let count = LOCK.update(deps.storage, val.clone(), action)?;

                    // upon receiving from n - f parties with the same val
                    if count >= state.n - F && !state.sent_done {
                        state.sent_done = true;
                        STATE.save(deps.storage, &state)?;

                        // send <done, val> to every party
                        let done_packet = PacketMsg::Done { val: val.clone() };
                        let channel_ids = get_id_channel_pair(&deps)?;

                        // Add Done msg to MsgQueue of every party
                        for (chain_id, _channel_id) in &channel_ids {
                            queue[*chain_id as usize].push(done_packet.clone());
                        }
                        // send <done, val> to every party /

                    }
                }
                Ok(())

            },
            PacketMsg::Done { val } => 

            // Handle Done
            {
                let mut state = STATE.load(deps.storage)?;
                // Update local record of done messages
                let action = |count: Option<u32>| -> StdResult<u32> {
                    match count {
                        Some(c) => Ok(c + 1),
                        None => Ok(1),
                    }
                };
                let count = DONE.update(deps.storage, val.clone(), action)?;

                if count > F + 1 {
                    // TODO: Sent Done
                }

                // upon receiving from n - f parties with the same val
                if count >= state.n - F {
                    // decide and terminate
                    state.done = Some(val.clone());
                    STATE.save(deps.storage, &state)?;
                }
                
                Ok(())
            },
        };
        
        // unwrap the result to handle any errors
        result?
    }

    // After handling all msgs in queue sucessfully
    // Generate msg queue to send
    let mut msgs = Vec::new();
    let timeout = get_timeout(env);

    //// TESTING /////
    let mut state = STATE.load(deps.storage)?;
    for (chain_id, msg_queue) in queue.iter().enumerate() {
        //// TESTING /////
        TEST_QUEUE.save(deps.storage, state.current_tx_id, &(chain_id as u32, msg_queue.to_vec()))?;
        state.current_tx_id += 1;
        STATE.save(deps.storage, &state)?;
        if chain_id != state.chain_id as usize {
            
            // When chain wish to send some msgs to dest chain
            if msg_queue.len() > 0 {
                let channel_id = CHANNELS.load(deps.storage, chain_id.try_into().unwrap())?;
                let msg = IbcMsg::SendPacket {
                    channel_id,
                    data: to_binary(&PacketMsg::MsgQueue { q: msg_queue.to_vec() } )?,
                    timeout: timeout.clone(),
                };
                msgs.push(msg);
            }
        }
    }
    let acknowledgement = to_binary(&AcknowledgementMsg::Ok(MsgQueueResponse { }))?;
    let mut res = IbcReceiveResponse::new();
    
    // Add to Response if there are pending messages
    if msgs.len() > 0 {
        TEST.save(deps.storage, state.current_tx_id, &msgs)?;
        // state.current_tx_id += 1;
        STATE.save(deps.storage, &state)?;
        res = res.add_messages(msgs);
    }
    
    Ok(res
        .set_ack(acknowledgement)
        .add_attribute("action", "receive_msg_queue"))



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
            PacketMsg::MsgQueue { q } => receive_queue(deps, env, dest_channel_id, q),
            PacketMsg::Propose {
                chain_id,
                k,
                v,
                view,
            } => receive_propose(
                deps,
                dest_channel_id,
                get_timeout(env),
                chain_id,
                k,
                v,
                view,
            ),
            PacketMsg::WhoAmI { chain_id } => receive_who_am_i(deps, dest_channel_id, chain_id),
            // PacketMsg::Commit { msg, tx_id } => receive_commit(deps, dest_channel_id, msg, tx_id),
            PacketMsg::Request { view, chain_id } => {
                receive_request(deps, dest_channel_id, view, chain_id)
            }
            PacketMsg::Suggest {
                chain_id,
                view,
                key2,
                key2_val,
                prev_key2,
                key3,
                key3_val,
            } => receive_suggest(
                deps,
                env,
                chain_id,
                view,
                key2,
                key2_val,
                prev_key2,
                key3,
                key3_val,
            ),
            PacketMsg::Proof {
                key1,
                key1_val,
                prev_key1,
                view,
            } => receive_proof(deps, key1, key1_val, prev_key1, view),
            PacketMsg::Echo { val, view} => receive_echo(deps, val, view),
            PacketMsg::Key1 { val, view } => receive_key1(deps, val, view),
            PacketMsg::Key2 { val, view } => receive_key2(deps, val, view),
            PacketMsg::Key3 { val, view } => receive_key3(deps, val, view),
            PacketMsg::Lock { val, view } => receive_lock(deps, val, view),
            PacketMsg::Done { val } => receive_done(deps, val),
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

pub fn queue_receive_suggest(
    _queue_to_process: Vec<Vec<PacketMsg>>,
    deps: DepsMut,
    env: &Env,
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
                let _timeout: IbcTimeout = env.block.time.plus_seconds(PACKET_LIFETIME).into();
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
                let timeout = env.block.time.plus_seconds(PACKET_LIFETIME).into();
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

fn accept_key(key: u32, value: String, proofs: Vec<(u32, String, i32)>) -> bool {
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

pub fn receive_request(
    deps: DepsMut,
    _caller: String,
    view: u32,
    chain_id: u32,
) -> StdResult<IbcReceiveResponse> {
    let mut state = STATE.load(deps.storage)?;
    state.key2_proofs.push((state.current_tx_id,"received_request".to_string(), chain_id as i32));
    state.current_tx_id += 1;
    STATE.save(deps.storage, &state)?;
    // Update stored highest_request for that blockchain accordingly
    let highest_request = HIGHEST_REQ.load(deps.storage, chain_id)?;
    if highest_request < view {
        HIGHEST_REQ.save(deps.storage, chain_id, &view)?;
    }

    let acknowledgement = to_binary(&AcknowledgementMsg::Ok(RequestResponse {}))?;

    Ok(IbcReceiveResponse::new()
        .set_ack(acknowledgement)
        .add_attribute("action", "receive_request")
        .add_attribute("chain_id", chain_id.to_string()))
}

fn open_lock(deps: &DepsMut, proofs: Vec<(u32, String, i32)>) -> StdResult<bool> {
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

// pub fn receive_wrapper(
//     msgs: Vec<SubMsg>,
//     receive_type: String
// ) -> StdResult<IbcReceiveResponse> {
//     let res = IbcReceiveResponse::new();
//     Ok(res)
// }

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
        if chain_id == state.primary && state.is_first_propose {
            // RECEIVED_PROPOSE.save(deps.storage, chain_id, &true)?;
            state.is_first_propose = false;
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
        if chain_id == state.primary && state.is_first_propose {
            // RECEIVED_PROPOSE.save(deps.storage, chain_id, &true)?;
            state.is_first_propose = false;
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
pub fn receive_commit(
    deps: DepsMut,
    _caller: String,
    msg: ExecuteMsg,
    tx_id: u32,
) -> StdResult<IbcReceiveResponse> {
    match msg {
        ExecuteMsg::Set { key, value } => try_set(deps, key, value.to_string(), tx_id),
        ExecuteMsg::Get { key } => try_get(deps, key, tx_id),
        ExecuteMsg::Input { value: _ } => todo!(),
    }
}
pub fn try_get(deps: DepsMut, key: String, tx_id: u32) -> StdResult<IbcReceiveResponse> {
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
        .add_attribute("value", value))
}

pub fn try_set(
    deps: DepsMut,
    key: String,
    value: String,
    tx_id: u32,
) -> StdResult<IbcReceiveResponse> {
    // let mut state = STATE.load(deps.storage)?;
    let action = |_| -> StdResult<String> { Ok(value.clone()) };
    VARS.update(deps.storage, &key, action)?;
    // STATE.save(deps.storage, &state)?;

    let response = CommitResponse {
        tx_id: tx_id.clone(),
    };
    let acknowledgement = to_binary(&AcknowledgementMsg::Ok(response))?;

    Ok(IbcReceiveResponse::new()
        .set_ack(acknowledgement)
        .add_attribute("action", "commit")
        .add_attribute("msg", "Set")
        .add_attribute("tx_id", tx_id.to_string())
        .add_attribute("key", key)
        .add_attribute("value", value))
}

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
    HIGHEST_ABORT.save(deps.storage, chain_id, &0)?;

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
    deps: DepsMut,
    env: Env,
    msg: IbcPacketAckMsg,
) -> StdResult<IbcBasicResponse> {
    let packet: PacketMsg = from_slice(&msg.original_packet.data)?;
    match packet {
        PacketMsg::MsgQueue { q: _ } => Ok(IbcBasicResponse::new()),
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
        PacketMsg::WhoAmI { chain_id: _ } => Ok(IbcBasicResponse::new()),
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
    }
}

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
    let _timeout: IbcTimeout = env.block.time.plus_seconds(PACKET_LIFETIME).into();
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

    use cosmwasm_std::testing::{
        mock_dependencies, mock_env, mock_ibc_channel_connect_ack, mock_ibc_channel_open_init,
        mock_ibc_channel_open_try, mock_ibc_packet_ack, mock_info, MockApi, MockQuerier,
        MockStorage,
    };
    use cosmwasm_std::{coins, CosmosMsg, OwnedDeps};

    fn setup() -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            role: "leader".to_string(),
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
