
use cosmwasm_std::{
    StdResult, Order, IbcReceiveResponse, to_binary, IbcMsg, StdError, Storage, IbcTimeout, DepsMut
};

use std::convert::TryInto;

use crate::utils::{get_id_channel_pair, get_id_channel_pair_from_storage, 
    F, NUMBER_OF_NODES};
use crate::ibc_msg::{Msg,AcknowledgementMsg, MsgQueueResponse, PacketMsg};
use crate::state::{
    HIGHEST_REQ, STATE, SEND_ALL_UPON, CHANNELS, RECEIVED_SUGGEST, ECHO, KEY1, KEY2, KEY3, LOCK, DONE, 
    TEST_QUEUE, TEST, HIGHEST_ABORT, State, DEDUPE_PACKET, RECEIVED_ECHO, RECEIVED_KEY_1, RECEIVED_KEY_2, RECEIVED_PROOF
};
use crate::abort::{handle_abort};


pub fn receive_queue(
    store: &mut dyn Storage,
    timeout: IbcTimeout,
    local_channel_id: Option<String>,
    queue_to_process: Vec<Msg>,
    queue: &mut Vec<Vec<Msg>>
) -> StdResult<IbcReceiveResponse> {
    let state = STATE.load(store)?;
    // let mut queue: Vec<Vec<Msg>> = vec!(Vec::new(); state.n.try_into().unwrap());

    for msg in queue_to_process {
        let result: StdResult<()> = match msg {
            Msg::Propose {
                chain_id,
                k,
                v,
                view,
            } => 
                // Handle Propose
                {
                    let mut state = STATE.load(store)?;
                    // ignore messages from other views, other than abort, done and request messages
                    if view != state.view {
                    } else {
                        // upon receiving the first propose message from a chain
                        if chain_id == state.primary && state.is_first_propose {
                            // RECEIVED_PROPOSE.save(store, chain_id, &true)?;
                            let mut broadcast = false;
                            state.is_first_propose = false;
                            STATE.save(store, &state)?;
                            
                            // First case we should broadcast Echo message
                            if state.lock == 0 || v == state.lock_val {
                                broadcast = true;
                            } else if view > k && k >= state.lock {
                                // upon open_lock(proofs) == true
                                // Second case we should broadcast Echo message
                                if open_lock(store, state.proofs)? {
                                    broadcast = true;
                                }
                            }
                            // send_all_upon_join_queue(<echo, k, v, view>)
                            if broadcast {
                                let echo_packet = Msg::Echo { val: v, view };
                                let channel_ids = get_id_channel_pair(store)?;
                                let state = STATE.load(store)?;
                                for (chain_id, _channel_id) in &channel_ids {
                                    let highest_request = HIGHEST_REQ.load(store, chain_id.clone())?;
                                    if highest_request == state.view {
                                        queue[*chain_id as usize].push(echo_packet.clone());
                                    }
                                    // Otherwise, we need the msg to be recorded in queue so that it could be triggered when condition satisfies
                                    else{
                                        let action = |packets: Option<Vec<Msg>>| -> StdResult<Vec<Msg>> {
                                            match packets {
                                                Some(mut p) => {
                                                    p.push(echo_packet.clone());
                                                    Ok(p)
                                                },
                                                None => Ok(vec!(echo_packet.clone())),
                                            }
                                            
                                        };
                                        SEND_ALL_UPON.update(store, *chain_id, action)?;
                                    }
                                }
                            }
                            // send_all_upon_join_queue(<echo, k, v, view>)/

                        }
                    }
                    Ok(())

                },
            // Msg::Commit { msg, tx_id } => receive_commit(deps, dest_channel_id, msg, tx_id),
            Msg::Request { view, chain_id } => 
                
                // Handle Request
                {
                    let mut state = STATE.load(store)?;
                    // state.key2_proofs.push((state.current_tx_id,"received_request".to_string(), chain_id as i32));
                    // STATE.save(store, &state)?;
                    // Update stored highest_request for that blockchain accordingly
                    let highest_request = HIGHEST_REQ.load(store, chain_id)?;
                    if highest_request < view {
                        HIGHEST_REQ.save(store, chain_id, &view)?;
                            
                        if view == state.view {
                            // Check if we are ready to send Suggest to Primary
                            if chain_id == state.primary && !state.sent_suggest {
                                let packet = Msg::Suggest {
                                    chain_id: state.chain_id,
                                    view: state.view,
                                    key2: state.key2,
                                    key2_val: state.key2_val.clone(),
                                    prev_key2: state.prev_key2,
                                    key3: state.key3,
                                    key3_val: state.key3_val.clone(),
                                };
                                state.sent_suggest = true;
                                STATE.save(store, &state)?;
                                queue[chain_id as usize].push(packet);
                            }

                            // Check if any pending send_all_upon_join
                            let packets = SEND_ALL_UPON.may_load(store, chain_id)?;
                            match packets {
                                Some(p) => {
                                    // Add to queue and remove from the buffer
                                    queue[chain_id as usize].extend(p);
                                    SEND_ALL_UPON.remove(store, chain_id);

                                },
                                None => (),
                            };
                        }
                    }
                    Ok(())
                },
            Msg::Suggest {
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
                let mut state = STATE.load(store)?;

                // When I'm the primary
                if state.primary == state.chain_id {

                    // upon receiving the first suggest message from a chain
                    if !RECEIVED_SUGGEST.load(store, chain_id)? {
                        RECEIVED_SUGGEST.save(store, chain_id, &true)?;
                        // Check if the following conditions hold
                        if prev_key2 < key2 as i32 && key2 < view {
                            state.key2_proofs.push((key2, key2_val, prev_key2));
                            STATE.save(store, &state)?;
                        }
                        if key3 == 0 {
                            state.suggestions.push((key3, key3_val));
                            STATE.save(store, &state)?;
                        } else if key3 < view {
                            // Upon accept_key = true
                            if accept_key(key3, key3_val.clone(), state.key2_proofs.clone()) {
                                state.suggestions.push((key3, key3_val.clone()));
                                STATE.save(store, &state)?;
                            }
                        }

                        // Check if |suggestions| >= n - f
                        if state.suggestions.len() >= (state.n - F) as usize {
                            // Retrive the entry with the largest k
                            let (k, v) = state.suggestions.iter().max().unwrap();
                            let propose_packet = Msg::Propose {
                                chain_id: state.chain_id,
                                k: k.clone(),
                                v: v.clone(),
                                view: state.view,
                            };
                            
                            send_all_upon_join_queue(store, queue, propose_packet, timeout.clone())?;
                            /*

                            // send_all_upon_join_queue(<propose, k, v, view>)
                            let channel_ids = get_id_channel_pair(store)?;
                            // let state = STATE.load(store)?;
                            for (chain_id, _channel_id) in &channel_ids {
                                let highest_request = HIGHEST_REQ.load(store, chain_id.clone())?;
                                if highest_request == state.view {
                                    queue[*chain_id as usize].push(propose_packet.clone());

                                }
                                // Otherwise, we need the msg to be recorded in queue so that it could be triggered when condition satisfies
                                else {
                                    let action = |packets: Option<Vec<Msg>>| -> StdResult<Vec<Msg>> {
                                        match packets {
                                            Some(mut p) => {
                                                p.push(propose_packet.clone());
                                                Ok(p)
                                            },
                                            None => Ok(vec!(propose_packet.clone())),
                                        }
                                        
                                    };
                                    SEND_ALL_UPON.update(store, *chain_id, action)?;
                                }
                            }
                            */

                        }
                        
                    }
                }

                Ok(())
    
            },

            Msg::Proof {
                key1,
                key1_val,
                prev_key1,
                view,
            } => 

            // Handle Proof
            {
                // detect if self-send
                let chain_id = match local_channel_id.clone() {
                    Some(id) => {
                        // Get the chain_id of the sender
                        CHANNELS
                        .range(store, None, None, Order::Ascending)
                        .find_map(|res| { 
                            let (chain_id,channel_id) = res.unwrap(); 
                            if channel_id == id { 
                                Some(chain_id) 
                            } 
                            else { None }
                        }).unwrap()
                    },
                    None => state.chain_id,
                };
    
                // let chain_id = chain_id_res.unwrap();
                let received_proof = RECEIVED_PROOF.load(store, chain_id)?;
                if !received_proof {
                    // Update the state
                    RECEIVED_PROOF.save(store, chain_id, &true)?;
                    
                    if view > key1 && key1 as i32 > prev_key1 {
                        let mut state = STATE.load(store)?;
                        state.proofs.push((key1, key1_val, prev_key1));
                        STATE.save(store, &state)?;
                    } 
                    //// TESTING ////
                    // let mut state = STATE.load(store)?;
                    // state.proofs.push((key1, key1_val.clone(), prev_key1));
                    // STATE.save(store, &state)?;

                    // if condition is met, update the proofs accordingly
                    
                }
                
                Ok(())
            },
            Msg::Echo { val, view } => 
            

            // Handle Echo
            {

                // detect if self-send
                let chain_id = match local_channel_id.clone() {
                    Some(id) => {
                        // Get the chain_id of the sender
                        CHANNELS
                        .range(store, None, None, Order::Ascending)
                        .find_map(|res| { 
                            let (chain_id,channel_id) = res.unwrap(); 
                            if channel_id == id { 
                                Some(chain_id) 
                            } 
                            else { None }
                        }).unwrap()
                    },
                    None => state.chain_id,
                };

                let key1_packet = Msg::Key1 { val: val.clone(), view };
                let mut state = STATE.load(store)?;
                let received_key_1 = RECEIVED_KEY_1.load(store, chain_id)?;

                // ignore messages from other views, other than abort, done and request messages
                if view != state.view && received_key_1 {
                } else { 
                    message_transfer_hop(store, val.clone(), view, queue, ECHO, key1_packet.clone(), timeout.clone())?;

                    if state.key1_val != val {
                        state.prev_key1 = state.key2 as i32;
                        state.key1_val = val;                    
                    }
                    state.key1 = view;
                    STATE.save(store, &state)?;    
                }
                Ok(())

            },
            Msg::Key1 { val, view } => 

            // Handle Key1
            {
                let key2_packet = Msg::Key2 { val: val.clone(), view };
 
                // ignore messages from other views, other than abort, done and request messages
                if view != state.view {
                } else {
                    message_transfer_hop(store, val.clone(), view, queue, KEY1, key2_packet.clone(), timeout.clone())?;
                    let mut state = STATE.load(store)?;
                    if state.key2_val != val {
                        state.prev_key2 = state.key2 as i32;
                        state.key2_val = val;                    
                    }
                    state.key2 = view;
                    STATE.save(store, &state)?;    
                }
                Ok(())
            },
            Msg::Key2 { val, view } => 

            // Handle Key2
            {
                let key3_packet = Msg::Key3 { val: val.clone(), view };
                if view != state.view {
                } else {
                    message_transfer_hop(store, val.clone(), view, queue, KEY2, key3_packet.clone(),timeout.clone())?;
                    let mut state = STATE.load(store)?;
                    state.key3 = view;
                    state.key3_val = val.clone();
                    STATE.save(store, &state)?;    
                }
                Ok(())

            },
            Msg::Key3 { val, view } => 

            // Handle Key3
            {
                let lock_packet = Msg::Lock { val: val.clone(), view }; 
                if view != state.view {
                } else {
                    message_transfer_hop(store, val.clone(), view, queue, KEY3, lock_packet.clone(), timeout.clone())?;
                    let mut state = STATE.load(store)?;
                    state.lock = view;
                    state.lock_val = val;
                    STATE.save(store, &state)?;    
                }
                Ok(())
            },
            Msg::Lock { val, view } => 
            
            // Handle Lock
            {
                let mut state = STATE.load(store)?;
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
                    let count = LOCK.update(store, val.clone(), action)?;

                    // upon receiving from n - f parties with the same val
                    if count >= state.n - F && !state.sent_done {
                        state.sent_done = true;
                        STATE.save(store, &state)?;

                        // send <done, val> to every party
                        let done_packet = Msg::Done { val: val.clone() };
                        send_all_party(store, queue, done_packet, timeout.clone())?;

                    }
                }
                Ok(())
            },
            Msg::Done { val } => 
            // Handle Done
            {
                let mut state = STATE.load(store)?;
                // Update local record of done messages
                let action = |count: Option<u32>| -> StdResult<u32> {
                    match count {
                        Some(c) => Ok(c + 1),
                        None => Ok(1),
                    }
                };
                let count = DONE.update(store, val.clone(), action)?;

                // Currently we don't have enough nodes to test with F + 1
                if count >= F + 1 {
                    if !state.sent_done {
                        state.sent_done = true;
                        STATE.save(store, &state)?;
                        let done_packet = Msg::Done { val: val.clone() };
                        send_all_party(store, queue, done_packet, timeout.clone())?;
                    }

                }

                // upon receiving from n - f parties with the same val
                if count >= state.n - F {
                    // decide and terminate
                    state.done = Some(val.clone());
                    STATE.save(store, &state)?;
                }
                
                Ok(())
            }, 
            Msg::Abort { view, chain_id } => 
            {
                handle_abort(store, view, chain_id)
            }
        };
        
        // unwrap the result to handle any errors
        result?
    }

    match local_channel_id {
        Some(_) => {
            // After handling all msgs in queue sucessfully
            // Generate msg queue to send
            let mut msgs = Vec::new();
            // let timeout = get_timeout(env);

            //// TESTING /////
            let mut state = STATE.load(store)?;
            for (chain_id, msg_queue) in queue.iter().enumerate() {
                //// TESTING /////
                TEST_QUEUE.save(store, state.current_tx_id, &(chain_id as u32, msg_queue.to_vec()))?;
                state.current_tx_id += 1;
                STATE.save(store, &state)?;
                if chain_id != state.chain_id as usize {
                    
                    // When chain wish to send some msgs to dest chain
                    if msg_queue.len() > 0 {
                        let channel_id = CHANNELS.load(store, chain_id.try_into().unwrap())?;
                        let msg = IbcMsg::SendPacket {
                            channel_id,
                            data: to_binary(&PacketMsg::MsgQueue ( msg_queue.to_vec() ) )?,
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
                TEST.save(store, state.current_tx_id, &msgs)?;
                // state.current_tx_id += 1;
                STATE.save(store, &state)?;
                res = res.add_messages(msgs);
            }
            
            Ok(res
                .set_ack(acknowledgement)
                .add_attribute("action", "receive_msg_queue"))
        },
        None => Ok(IbcReceiveResponse::new()),
    }



    

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


fn open_lock(store: &mut dyn Storage, proofs: Vec<(u32, String, i32)>) -> StdResult<bool> {
    let mut supporting: u32 = 0;
    let state = STATE.load(store)?;
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
     queue: &mut Vec<Vec<Msg>>, 
     message_type: cw_storage_plus::Map<String, u32>, msg_to_send: Msg, timeout: IbcTimeout) -> Result<(), StdError> {
    let state = STATE.load(storage)?;
    // ignore messages from other views, other than abort, done and request messages
    if view != state.view {
    } else {
        // Update local record of messages of type key
        let action = |count: Option<u32>| -> StdResult<u32> {
            match count {
                Some(c) => Ok(c + 1),
                None => Ok(1),
            }
        };
        let count = message_type.update(storage, val.clone(), action)?;

        // upon receiving from n - f parties with the same val
        if count >= state.n - F {

            // send_all_upon_join_queue
            send_all_upon_join_queue(storage, queue, msg_to_send, timeout)?;

        }
    }
    Ok(())
}

// send_all_upon_join_queue Operation
pub fn send_all_upon_join_queue(storage: &mut dyn Storage, queue: &mut Vec<Vec<Msg>>, packet_msg: Msg, timeout: IbcTimeout) -> Result<(), StdError> {
    let state = STATE.load(storage)?;
    let channel_ids = get_id_channel_pair_from_storage(storage)?;
    // self-send msg
    receive_queue(storage, timeout, None, vec![packet_msg.clone()], queue)?;

    for (chain_id, _channel_id) in &channel_ids {
        let highest_request = HIGHEST_REQ.load(storage, chain_id.clone())?;
        if highest_request == state.view {
            queue[*chain_id as usize].push(packet_msg.clone());
        }
        // Otherwise, we need the msg to be recorded in queue so that it could be triggered when condition satisfies
        else{
            let action = |packets: Option<Vec<Msg>>| -> StdResult<Vec<Msg>> {
                match packets {
                    Some(mut p) => {
                        p.push(packet_msg.clone());
                        Ok(p)
                    },
                    None => Ok(vec!(packet_msg.clone())),
                }
                
            };
            SEND_ALL_UPON.update(storage, *chain_id, action)?;
        }
    }
    Ok(())
}

pub fn send_all_party(store: &mut dyn Storage, queue: &mut Vec<Vec<Msg>>, packet: Msg, timeout: IbcTimeout) -> Result<(), StdError> {
    let channel_ids = get_id_channel_pair_from_storage(store)?;
    // self-send msg
    receive_queue(store, timeout, None, vec![packet.clone()], queue)?;

    // send <done, val> to every party
    // Add Done msg to MsgQueue of every party
    for (chain_id, _channel_id) in &channel_ids {
        queue[*chain_id as usize].push(packet.clone());
    }
    // send <done, val> to every party /
    
    Ok(())
}