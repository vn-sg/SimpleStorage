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
use crate::utils::{get_id_channel_pair, get_timeout};

pub fn view_change(storage: &mut dyn Storage, timeout: IbcTimeout) -> Result<Response, ContractError> {

    let state = STATE.load(storage)?;
    let mut queue: Vec<Vec<Msg>> = vec!(Vec::new(); state.n.try_into().unwrap());

    append_queue_view_change(storage, & mut queue, timeout.clone())?;
    let msgs = convert_queue_to_ibc_msgs(storage, & mut queue, timeout.clone())?;

    Ok(Response::new()
        .add_messages(msgs)
        .add_attribute("action", "execute")
        .add_attribute("msg_type", "input"))
}

pub fn append_queue_view_change(
    storage: &mut dyn Storage,
    queue: &mut Vec<Vec<Msg>>,
    timeout: IbcTimeout,
) -> Result<(), ContractError> {
    // load the state
    let state = STATE.load(storage)?;
    // Add Request message to packets_to_be_broadcasted
    let request_packet = Msg::Request {
        view: state.view,
        chain_id: state.chain_id,
    };

    // Send Request to all parties
    send_all_party(storage, queue, request_packet, timeout.clone())?;

    
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
        receive_queue(storage, timeout.clone(), None, vec![suggest_packet], queue)?;
    }


    // Contruct Request messages to be broadcasted
    let proof_packet = Msg::Proof {
        key1: state.key1,
        key1_val: state.key1_val.clone(),
        prev_key1: state.prev_key1,
        view: state.view,
    };
    // send_all_upon_join(Proof)
    send_all_upon_join_queue(storage, queue, proof_packet, timeout.clone())?;
    Ok(())
}

fn convert_queue_to_ibc_msgs(storage: &mut dyn Storage,
                            queue: &mut Vec<Vec<Msg>>,
                            timeout: IbcTimeout,
) -> Result<Vec<IbcMsg>, ContractError>{
    let mut state = STATE.load(storage)?;
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