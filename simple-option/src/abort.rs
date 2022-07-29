use cosmwasm_std::{
    StdResult, Order, IbcReceiveResponse, to_binary, IbcMsg, StdError, Storage, IbcTimeout, DepsMut
};

use std::convert::TryInto;

use crate::utils::{get_id_channel_pair, get_id_channel_pair_from_storage, 
    F, NUMBER_OF_NODES};
use crate::ibc_msg::{Msg,AcknowledgementMsg, MsgQueueResponse, PacketMsg};
use crate::state::{
    HIGHEST_REQ, STATE, SEND_ALL_UPON, CHANNELS, RECEIVED_SUGGEST, ECHO, KEY1, KEY2, KEY3, LOCK, DONE, 
    TEST_QUEUE, RECEIVED_PROOF, TEST, HIGHEST_ABORT, State
};

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
        
        let u = vector_values[ (F+1-1) as usize]; 
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

        let w = vector_values[(state.n-F-1) as usize];
        if (w+1) as u32 >= state.view {
            state.view = (w + 1) as u32;
            STATE.save(storage, &state)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {

    use std::error::Error;

    // https://docs.cosmwasm.com/tutorials/simple-option/testing/
    use super::*;
    use cosmwasm_std::testing::{
        mock_dependencies, mock_env, mock_info, MockApi, MockQuerier, MockStorage,
    };
    use cosmwasm_std::{coins, from_binary, OwnedDeps};
    use crate::state::{HIGHEST_ABORT, STATE, State};

    #[test]
    fn test_abort() {
        let mut deps = mock_dependencies();
        let mut _env = mock_env();
        let mut _info = mock_info(&"test".to_string(), 
        &coins(2, "token"));

        let storage = deps.as_mut().storage;

        //init..
        let mut mock_state = State::new(0, "test_abort".to_string());
        mock_state.n = 4;
        STATE.save(storage, &mock_state);
        HIGHEST_ABORT.update(storage, 0, |_| -> StdResult<_> {Ok(-1)});
        HIGHEST_ABORT.update(storage, 1, |_| -> StdResult<_> {Ok(-1)});
        HIGHEST_ABORT.update(storage, 2, |_| -> StdResult<_> {Ok(-1)});
        HIGHEST_ABORT.update(storage, 3, |_| -> StdResult<_> {Ok(-1)});
        //init..
        assert_eq!(mock_state.view, 0);


        let result = handle_abort(storage, 0, 1);
        match result {
            Ok(_) => (),
            Err(msg) => {
                let cause = msg.source().unwrap();
                panic!(msg.to_string())
            }
        }
        let mut state = STATE.load(storage).unwrap();
        let mut abort0 = HIGHEST_ABORT.load(storage, 0).unwrap();
        let mut abort1 = HIGHEST_ABORT.load(storage, 1).unwrap();
        let mut abort2 = HIGHEST_ABORT.load(storage,2).unwrap();
        let mut abort3 = HIGHEST_ABORT.load(storage,3).unwrap();
        assert_eq!(state.view, 0);
        assert_eq!(abort0, -1);
        assert_eq!(abort1, 0);
        assert_eq!(abort2, -1);
        assert_eq!(abort3, -1);

        let result = handle_abort(storage, 0, 0);
        match result {
            Ok(_) => (),
            Err(msg) => {
                let cause = msg.source().unwrap();
                panic!(msg.to_string())
            }
        }
        let mut state = STATE.load(storage).unwrap();
        let mut abort0 = HIGHEST_ABORT.load(storage, 0).unwrap();
        let mut abort1 = HIGHEST_ABORT.load(storage, 1).unwrap();
        let mut abort2 = HIGHEST_ABORT.load(storage,2).unwrap();
        let mut abort3 = HIGHEST_ABORT.load(storage,3).unwrap();
        assert_eq!(state.view, 0);
        assert_eq!(abort0, -1);
        assert_eq!(abort1, 0);
        assert_eq!(abort2, -1);
        assert_eq!(abort3, -1);
    }


}