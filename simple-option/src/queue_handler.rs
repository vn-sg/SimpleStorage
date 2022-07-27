
use cosmwasm_std::{
    StdResult, DepsMut, Order, Env, IbcReceiveResponse, to_binary, IbcMsg, StdError, Storage
};

use std::convert::TryInto;

use crate::utils::{get_id_channel_pair, get_id_channel_pair_from_storage, 
    F, PACKET_LIFETIME, get_timeout, NUMBER_OF_NODES};
use crate::ibc_msg::{PacketMsg,AcknowledgementMsg, MsgQueueResponse};
use crate::state::{
    HIGHEST_REQ, STATE, SEND_ALL_UPON, CHANNELS, RECEIVED_SUGGEST, ECHO, KEY1, KEY2, KEY3, LOCK, DONE, 
    TEST_QUEUE, RECEIVED_PROOF, TEST, HIGHEST_ABORT
};


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
                    // state.key2_proofs.push((state.current_tx_id,"received_request".to_string(), chain_id as i32));
                    // STATE.save(deps.storage, &state)?;
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
                let received_proof = RECEIVED_PROOF.load(deps.storage, chain_id)?;
                if !received_proof {
                    // Update the state
                    RECEIVED_PROOF.save(deps.storage, chain_id, &true)?;
                    
                    if view > key1 && key1 as i32 > prev_key1 {
                        let mut state = STATE.load(deps.storage)?;
                        state.proofs.push((key1, key1_val, prev_key1));
                        STATE.save(deps.storage, &state)?;
                    } 
                    //// TESTING ////
                    // let mut state = STATE.load(deps.storage)?;
                    // state.proofs.push((key1, key1_val.clone(), prev_key1));
                    // STATE.save(deps.storage, &state)?;

                    // if condition is met, update the proofs accordingly
                    
                }
                
                Ok(())
            },
            PacketMsg::Echo { val, view } => 
            

            // Handle Echo
            {
                let key1_packet = PacketMsg::Key1 { val: val.clone(), view };

                // ignore messages from other views, other than abort, done and request messages
                if view != state.view {
                } else {
                    message_transfer_hop(deps.storage, val.clone(), view, & mut queue, ECHO, key1_packet.clone())?;
                    let mut state = STATE.load(deps.storage)?;
                    if state.key1_val != val {
                        state.prev_key1 = state.key2 as i32;
                        state.key1_val = val;                    
                    }
                    state.key1 = view;
                    STATE.save(deps.storage, &state)?;    
                }
                Ok(())

            },
            PacketMsg::Key1 { val, view } => 

            // Handle Key1
            {
                let key2_packet = PacketMsg::Key2 { val: val.clone(), view };

                // ignore messages from other views, other than abort, done and request messages
                if view != state.view {
                } else {
                    message_transfer_hop(deps.storage, val.clone(), view, & mut queue, KEY1, key2_packet.clone())?;
                    let mut state = STATE.load(deps.storage)?;
                    if state.key2_val != val {
                        state.prev_key2 = state.key2 as i32;
                        state.key2_val = val;                    
                    }
                    state.key2 = view;
                    STATE.save(deps.storage, &state)?;    
                }
                Ok(())
            },
            PacketMsg::Key2 { val, view } => 

            // Handle Key2
            {
                let key3_packet = PacketMsg::Key3 { val: val.clone(), view };
                if view != state.view {
                } else {
                    message_transfer_hop(deps.storage, val.clone(), view, & mut queue, KEY2, key3_packet.clone())?;
                    let mut state = STATE.load(deps.storage)?;
                    state.key3 = view;
                    state.key3_val = val.clone();
                    STATE.save(deps.storage, &state)?;    
                }
                Ok(())

            },
            PacketMsg::Key3 { val, view } => 

            // Handle Key3
            {
                let lock_packet = PacketMsg::Lock { val: val.clone(), view }; 
                if view != state.view {
                } else {
                    message_transfer_hop(deps.storage, val.clone(), view, & mut queue, KEY3, lock_packet.clone())?;
                    let mut state = STATE.load(deps.storage)?;
                    state.lock = view;
                    state.lock_val = val;
                    STATE.save(deps.storage, &state)?;    
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

                // Currently we don't have enough nodes to test with F + 1
                if count >= F + 1 - 1{
                    if !state.sent_done {
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

                // upon receiving from n - f parties with the same val
                if count >= state.n - F {
                    // decide and terminate
                    state.done = Some(val.clone());
                    STATE.save(deps.storage, &state)?;
                }
                
                Ok(())
            }, 
            PacketMsg::Abort { view, chain_id } => 
            {
                handle_abort(deps.storage, view, chain_id)
            }
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

fn message_transfer_hop(storage: &mut dyn Storage, val: String, view: u32,
     queue: &mut Vec<Vec<PacketMsg>>, 
     keys: cw_storage_plus::Map<String, u32>, msg_to_send: PacketMsg) -> Result<(), StdError> {
    let state = STATE.load(storage)?;
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
        let count = keys.update(storage, val.clone(), action)?;

        // upon receiving from n - f parties with the same val
        if count >= state.n - F {

            // send_all_upon_join_queue TODO change to send_all_upon_join_queue
            let channel_ids = get_id_channel_pair_from_storage(storage)?;
            for (chain_id, _channel_id) in &channel_ids {
                let highest_request = HIGHEST_REQ.load(storage, chain_id.clone())?;
                if highest_request == state.view {
                    queue[*chain_id as usize].push(msg_to_send.clone());
                }
                // Otherwise, we need the msg to be recorded in queue so that it could be triggered when condition satisfies
                else{
                    let action = |packets: Option<Vec<PacketMsg>>| -> StdResult<Vec<PacketMsg>> {
                        match packets {
                            Some(mut p) => {
                                p.push(msg_to_send.clone());
                                Ok(p)
                            },
                            None => Ok(vec!(msg_to_send.clone())),
                        }
                        
                    };
                    SEND_ALL_UPON.update(storage, *chain_id, action)?;
                }
            }
            // send_all_upon_join_queue TODO change to send_all_upon_join_queue
        }
    }
    Ok(())
}

pub fn handle_abort(storage: &mut dyn Storage, view: u32, sender_chain_id: u32) -> Result<(), StdError> {
    let mut state = STATE.load(storage)?;
    if ((HIGHEST_ABORT.load(storage, sender_chain_id)? + 1) as u32)< (view+1) {
        HIGHEST_ABORT.update(storage, sender_chain_id, |option| -> StdResult<i32> {
            match option {
                Some(_val) => Ok(view as i32),
                None => Ok(view as i32),
            }
        })?;

        let highest_abort_vector_pair: StdResult<Vec<_>> = HIGHEST_ABORT
            .range(storage, None, None, Order::Ascending)
            .collect();
        let mut vector_values = match highest_abort_vector_pair {
            Ok(vec) => { 
                let temp = vec.iter().map(|(_key, value)| value.clone()).collect::<Vec<i32>>();
                temp
            }
            Err(_) => return Err(StdError::GenericErr { msg: "Error nth".to_string()}),
        };
        vector_values.sort();
        
        let u = vector_values[ (F+1) as usize]; 
        if u > HIGHEST_ABORT.load(storage, state.chain_id)? {
            if u >= -1 {
                HIGHEST_ABORT.update(storage, sender_chain_id, |option| -> StdResult<i32> {
                    match option {
                        Some(_val) => Ok(u),
                        None => Ok(u),
                    }
                })?;
            }
        }

        let w = vector_values[(NUMBER_OF_NODES-F) as usize];
        if (w+1) as u32 >= state.view {
            state.view = (w + 1) as u32;
            STATE.save(storage, &state)?;
        }

    }
    Ok(())
}


