use std::convert::TryInto;
use std::vec;

use cosmwasm_std::{
    DepsMut, to_binary, IbcTimeout, Response, IbcMsg, Storage
};

use crate::ibc_msg::{PacketMsg, Msg};
use crate::queue_handler::{receive_queue, send_all_party, send_all_upon_join_queue};
use crate::state::{
    HIGHEST_REQ, STATE, TEST_QUEUE, CHANNELS
};

use crate::ContractError;
use crate::utils::{get_id_channel_pair};

pub fn view_change(storage: &mut dyn Storage, timeout: IbcTimeout) -> Result<Response, ContractError> {

    let msgs = create_queue_view_change(storage, timeout)?;
    Ok(Response::new()
        .add_messages(msgs)
        .add_attribute("action", "execute")
        .add_attribute("msg_type", "input"))
}

pub fn create_queue_view_change(
    storage: &mut dyn Storage,
    timeout: IbcTimeout,
) -> Result<Vec<IbcMsg>, ContractError> {
    // load the state
    let state = STATE.load(storage)?;
    // Add Request message to packets_to_be_broadcasted
    let request_packet = Msg::Request {
        view: state.view,
        chain_id: state.chain_id,
    };

    // Contruct Request messages to be broadcasted
    let proof_packet = Msg::Proof {
        key1: state.key1,
        key1_val: state.key1_val.clone(),
        prev_key1: state.prev_key1,
        view: state.view,
    };
    // let mut msgs: Vec<IbcMsg> = Vec::new();
    let mut queue: Vec<Vec<Msg>> = vec!(Vec::new(); state.n.try_into().unwrap());

    // Send Request to all parties
    send_all_party(storage, &mut queue, request_packet, timeout.clone())?;

    
    let suggest_packet = Msg::Suggest {
        chain_id: state.chain_id,
        view: state.view,
        key2: state.key2,
        key2_val: state.key2_val.clone(),
        prev_key2: state.prev_key2,
        key3: state.key3,
        key3_val: state.key3_val.clone(),
    };
    // Upon highest_request[primary] == view
    if state.chain_id != state.primary {
        if state.view == HIGHEST_REQ.load(storage, state.primary)? {
            queue[state.primary as usize].push(suggest_packet);
        }
    } else {
        receive_queue(storage, timeout.clone(), None, vec![suggest_packet], &mut queue)?;
    }
    let mut state = STATE.load(storage)?;

    // send_all_upon_join(Proof)
    send_all_upon_join_queue(storage, &mut queue, proof_packet, timeout.clone())?;
    
    let mut msgs = Vec::new();
    for (chain_id, msg_queue) in queue.iter().enumerate() {
        //// TESTING /////
        TEST_QUEUE.save(storage, state.current_tx_id, &(chain_id as u32, msg_queue.to_vec()))?;
        state.current_tx_id += 1;
        STATE.save(storage, &state)?;
        if chain_id != state.chain_id as usize {
            // When chain wish to send some msgs to dest chain
            if msg_queue.len() > 0 {
                let channel_id = CHANNELS.load(storage, chain_id.try_into().unwrap())?;
                let msg = IbcMsg::SendPacket {
                    channel_id,
                    data: to_binary(&PacketMsg::MsgQueue ( msg_queue.to_vec() ) )?,
                    timeout: timeout.clone(),
                };
                msgs.push(msg);
            }
        }
    }
    
    Ok(msgs)
    
}

pub fn _create_queue_view_change_backup(
    deps: DepsMut,
    timeout: IbcTimeout,
) -> Result<Vec<IbcMsg>, ContractError> {
    // load the state
    let mut state = STATE.load(deps.storage)?;
    // Add Request message to packets_to_be_broadcasted
    let request_packet = Msg::Request {
        view: state.view,
        chain_id: state.chain_id,
    };

    // Contruct Request messages to be broadcasted
    let channels = get_id_channel_pair(deps.storage)?;
    let proof_packet = Msg::Proof {
        key1: state.key1,
        key1_val: state.key1_val.clone(),
        prev_key1: state.prev_key1,
        view: state.view,
    };
    // let mut msgs: Vec<IbcMsg> = Vec::new();
    let mut queue: Vec<Vec<Msg>> = vec!(vec![request_packet.clone()]; state.n.try_into().unwrap());

    for (chain_id, _channel_id) in &channels {
        // construct the msg queue to send
        // let mut queue = vec![request_packet.clone()];
        let highest_request = HIGHEST_REQ.load(deps.storage, chain_id.clone())?;

        if *chain_id == state.chain_id {
            if *chain_id == state.primary {
                // Contruct Suggest message
                let suggest_packet = Msg::Suggest {
                    chain_id: state.chain_id,
                    view: state.view,
                    key2: state.key2,
                    key2_val: state.key2_val.clone(),
                    prev_key2: state.prev_key2,
                    key3: state.key3,
                    key3_val: state.key3_val.clone(),
                };
                // self-send Suggest, Proof
                receive_queue(deps.storage, timeout.clone(), None, vec![suggest_packet, proof_packet.clone()], &mut queue)?;
            }
            else{
                // self-send Proof
                receive_queue(deps.storage, timeout.clone(), None, vec![proof_packet.clone()], &mut queue)?;
            }
            
        } else {
            // If dest chain is primary, check if satisfiy condition
            if chain_id.clone() == state.primary {
                // Contruct Suggest message to delivery to primary
                let suggest_packet = Msg::Suggest {
                    chain_id: state.chain_id,
                    view: state.view,
                    key2: state.key2,
                    key2_val: state.key2_val.clone(),
                    prev_key2: state.prev_key2,
                    key3: state.key3,
                    key3_val: state.key3_val.clone(),
                };
                
                // if state.chain_id != state.primary {
                // Upon highest_request[primary] = view
                if highest_request == state.view {
                    // queue.push(suggest_packet);
                    queue[state.primary as usize].push(suggest_packet);
                    
                }
                // } 
                state.sent.insert("Suggest".to_string());
                STATE.save(deps.storage, &state)?;
            }
            // send_all_upon_join(proof)
            if highest_request == state.view {
                queue[*chain_id as usize].push(proof_packet.clone());
            }
        }
        

        // msgs.push(msg);
    }
    let mut msgs = Vec::new();
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
                    data: to_binary(&PacketMsg::MsgQueue ( msg_queue.to_vec() ) )?,
                    timeout: timeout.clone(),
                };
                msgs.push(msg);
            }
        }
    }
    
    Ok(msgs)
    
}