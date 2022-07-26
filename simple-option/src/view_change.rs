use cosmwasm_std::{
    DepsMut, to_binary, IbcTimeout, Response, IbcMsg
};

use crate::ibc_msg::{PacketMsg};
use crate::state::{
    HIGHEST_REQ, STATE
};

use crate::ContractError;
use crate::utils::{get_id_channel_pair};

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