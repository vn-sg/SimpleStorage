use cosmwasm_std::{
    entry_point, from_slice, to_binary, Binary, DepsMut, Env, Event, IbcTimeout, Order, Response,
    StdError, StdResult,
};
use cosmwasm_std::{
    IbcBasicResponse, IbcChannelCloseMsg, IbcChannelConnectMsg, IbcChannelOpenMsg, IbcMsg,
    IbcOrder, IbcPacketAckMsg, IbcPacketReceiveMsg, IbcPacketTimeoutMsg, IbcReceiveResponse,
};

use crate::ibc_msg::{
    AcknowledgementMsg, CommitResponse, PacketMsg, ProposeResponse, RequestResponse,
    SuggestResponse, WhoAmIResponse, ProofResponse,
};
use crate::msg::ExecuteMsg;
use crate::state::{
    CHANNELS, HIGHEST_ABORT, HIGHEST_REQ, RECEIVED_SUGGEST, STATE, VARS,
};

use crate::ContractError;

pub const IBC_APP_VERSION: &str = "simple_storage";

/// Setting the lifetime of packets to be one hour
pub const PACKET_LIFETIME: u64 = 60 * 60;

pub fn view_change(deps: DepsMut, timeout: IbcTimeout) -> Result<Vec<IbcMsg>, ContractError> {
    // load the state
    let state = STATE.load(deps.storage)?;

    // Add Request message to packets_to_be_broadcasted
    let packets = vec![PacketMsg::Request {
        view: state.view,
        chain_id: state.chain_id,
    }];
    // Contruct Request messages to be broadcasted
    let mut msgs: Vec<IbcMsg> =
        create_broadcast_msgs(timeout.clone(), state.channel_ids.clone(), packets)?;

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
            let channel_id = CHANNELS.load(deps.storage, state.primary)?;
            msgs.push(IbcMsg::SendPacket {
                channel_id,
                data: to_binary(&packet)?,
                timeout: timeout.clone(),
            });

            // msgs.extend(suggest_msg);
        }
    
    }
    let proof_packet = PacketMsg::Proof {
        key1: state.key1,
        key1_val: state.key1_val,
        prev_key1: state.prev_key1,
        view: state.view,
    };
    let all_msgs = send_all_upon_join(&deps, timeout, msgs, proof_packet)?;

    Ok(all_msgs)
}

pub fn get_id_channel_pair(deps: &DepsMut) -> StdResult<Vec<(u32, String)>> {
    let channels: StdResult<Vec<_>> = CHANNELS
        .range(deps.storage, None, None, Order::Ascending)
        .collect();
    channels
}

pub fn send_all_upon_join(
    deps: &DepsMut,
    timeout: IbcTimeout,
    msgs_so_far: Vec<IbcMsg>,
    packet_to_broadcast: PacketMsg,
) -> Result<Vec<IbcMsg>, ContractError> {
    let channel_ids = get_id_channel_pair(&deps)?;

    let mut msgs = msgs_so_far;
    let state = STATE.load(deps.storage)?;
    for (chain_id, channel_id) in &channel_ids {
        let highest_request = HIGHEST_REQ.load(deps.storage, chain_id.clone())?;
        if highest_request == state.view {
            let msg = IbcMsg::SendPacket {
                channel_id: channel_id.clone(),
                data: to_binary(&packet_to_broadcast)?,
                timeout: timeout.clone(),
            };
            msgs.push(msg);
        }
    }

    Ok(msgs)
}

pub fn create_broadcast_msgs(
    timeout: IbcTimeout,
    channel_ids: Vec<String>,
    packets_to_broadcast: Vec<PacketMsg>,
) -> Result<Vec<IbcMsg>, ContractError> {
    let mut msgs: Vec<IbcMsg> = Vec::new();
    // if is_to_primary {
    //     let channel_id = &channel_ids[0];
    //     for packet in packets_to_broadcast {
    //         let msg = IbcMsg::SendPacket {
    //             channel_id: channel_id.clone(),
    //             data: to_binary(&packet)?,
    //             timeout: timeout.clone(),
    //         };
    //         msgs.push(msg);
    //     }
    // } else {
        for packet in packets_to_broadcast {
            for channel_id in &channel_ids {
                let msg = IbcMsg::SendPacket {
                    channel_id: channel_id.clone(),
                    data: to_binary(&packet)?,
                    timeout: timeout.clone(),
                };
                msgs.push(msg);
            }
        }
    // }
    Ok(msgs)
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

fn get_timeout(env: Env) -> IbcTimeout {
    env.block.time.plus_seconds(PACKET_LIFETIME).into()
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
            PacketMsg::Commit { msg, tx_id } => receive_commit(deps, dest_channel_id, msg, tx_id),
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
            PacketMsg::Echo { val: _, view: _ } => todo!(),
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
    let mut msgs: Vec<IbcMsg> = Vec::new();

    // When I'm the primary
    if state.primary == state.chain_id { 
        //// TESTING ////
        // state.suggestions.push((u32::MAX, "TESTING".to_string()));

        // upon receiving the first suggest message from a chain
        if !RECEIVED_SUGGEST.load(deps.storage, from_chain_id)? {
            RECEIVED_SUGGEST.save(deps.storage, from_chain_id, &true)?;
            // Check if the following conditions hold
            if prev_key2 < key2 as i32 && key2 < view {
                state.key2_proofs.push((key2, key2_val, prev_key2));
            }
            if key3 == 0 {
                state.suggestions.push((key3, key3_val));
            } else if key3 < view {
                // Upon accept_key = true
                if accept_key(key3, key3_val.clone(), state.key2_proofs.clone()) {
                    state.suggestions.push((key3, key3_val.clone()));
                }
            }

            // Check if |suggestions| >= n - f
            if state.suggestions.len() >= 3 - 1 {
                let timeout = env.block.time.plus_seconds(PACKET_LIFETIME).into();
                // Retrive the entry with the largest k
                let (k, v) = state.suggestions.iter().max().unwrap();
                let propose_packet = PacketMsg::Propose {
                    chain_id: state.chain_id,
                    k: k.clone(),
                    v: v.clone(),
                    view: state.view,
                };

                msgs.extend(
                    send_all_upon_join(&deps, timeout, Vec::new(), propose_packet)
                        .unwrap(),
                );
            }
            
        }
        STATE.save(deps.storage, &state)?;
    }

    let response = SuggestResponse {};
    let acknowledgement = to_binary(&AcknowledgementMsg::Ok(response))?;
    let res = IbcReceiveResponse::new()
        .set_ack(acknowledgement)
        .add_attribute("action", "receive_suggest")
        .add_attribute("suggest_sender_chain_id", from_chain_id.to_string());
    if !msgs.is_empty() {
        Ok(res.add_messages(msgs))
    } else {
        Ok(res)
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

pub fn receive_request(
    deps: DepsMut,
    _caller: String,
    view: u32,
    chain_id: u32,
) -> StdResult<IbcReceiveResponse> {
    let _state = STATE.load(deps.storage)?;
    // Update stored highest_request for that blockchain accordingly
    let highest_request = HIGHEST_REQ.load(deps.storage, chain_id)?;
    if highest_request == u32::MAX {
    } else if highest_request < view {
        HIGHEST_REQ.save(deps.storage, chain_id, &view)?;
    }

    let response = RequestResponse {};
    let acknowledgement = to_binary(&AcknowledgementMsg::Ok(response))?;

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
    if supporting >= 1 + 1 {
        Ok(true)
    } else {
        Ok(false)
    }
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
    let mut msgs: Vec<IbcMsg> = Vec::new();
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
                msgs.extend(send_all_upon_join(&deps, timeout.clone(), msgs.clone(), echo_packet).unwrap());
            

            } else if view > k && k >= state.lock {
                // upon open_lock(proofs) == true
                // Second case we should broadcast Echo message
                if open_lock(&deps, state.proofs)? {
                    let echo_packet = PacketMsg::Echo { val: v, view };
                    msgs.extend(send_all_upon_join(&deps, timeout.clone(), msgs.clone(), echo_packet).unwrap());
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
        Ok(res.add_messages(msgs))
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
        PacketMsg::Propose {
            chain_id: _,
            k: _,
            v: _,
            view: _,
        } => {
            let res: AcknowledgementMsg<ProposeResponse> = from_slice(&msg.acknowledgement.data)?;
            acknowledge_propose(deps, env, res)
        }
        PacketMsg::Commit { msg: _, tx_id: _ } => Ok(IbcBasicResponse::new()),
        PacketMsg::WhoAmI { chain_id: _ } => Ok(IbcBasicResponse::new()),
        PacketMsg::Request {
            chain_id: _,
            view: _,
        } => Ok(IbcBasicResponse::new()),
        PacketMsg::Suggest {
            view: _,
            key2: _,
            key2_val: _,
            prev_key2: _,
            key3: _,
            key3_val: _,
            chain_id: _,
        } => todo!(),
        PacketMsg::Proof {
            key1: _,
            key1_val: _,
            prev_key1: _,
            view: _,
        } => todo!(),
        PacketMsg::Echo { val: _, view: _ } => todo!()
    }
}

fn acknowledge_propose(
    _deps: DepsMut,
    env: Env,
    ack: AcknowledgementMsg<ProposeResponse>,
) -> StdResult<IbcBasicResponse> {
    let _timeout: IbcTimeout = env.block.time.plus_seconds(PACKET_LIFETIME).into();
    // retrive tx_id from acknowledge message
    match ack {
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
