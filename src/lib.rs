#[macro_use]
extern crate sgx_tstd as std;

extern crate protobuf;

extern crate ekiden_core_common;
#[macro_use]
extern crate ekiden_core_trusted;

#[macro_use]
extern crate poker_api;
extern crate rand;
extern crate rs_poker;
extern crate serde_cbor;

mod poker_contract;

use poker_contract::PokerContract;
use poker_api::*;

use ekiden_core_common::{with_contract_state, Address, Contract, ContractError};

#[allow(unused)]
#[prelude_import]
use std::prelude::v1::*;
create_enclave_api!();

fn create(request: &CreateGameRequest) -> Result<(PokerState, CreateGameResponse), ContractError> {
    let contract = PokerContract::new(
        request.get_blind(),
        request.get_max_players(),
        request.get_time_per_turn(),
    );

    let response = CreateGameResponse::new();
    response.set_success(true);

    Ok((contract.get_state(), response))
}

fn join(
    state: &PokerState,
    request: &JoinGameRequest,
) -> Result<(PokerState, JoinGameResponse), ContractError> {
    let mut playing;
    let state = with_contract_state(&state, |contract: &mut PokerContract| {
        playing = contract.join_game(
            &Address::from(request.get_sender().to_string()),
            request.get_deposit(),
            request.get_seed(),
        )?;

        Ok(());
    })?;

    let response = JoinGameResponse::new();
    response.set_success(true);
    response.set_playing(playing);

    Ok(state, response)
}

fn play(
    state: &PokerState,
    request: &PlayHandRequest,
) -> Result<(PokerState, PlayHandResponse), ContractError> {
    let state = with_contract_state(&state, |contract: &mut PokerContract| {
        contract.play_hand(&Address::from(request.get_sender().tos_string()))?;

        Ok(())
    })?;

    let response = PlayHandResponse::new();
    response.set_success(true);

    Ok(state, response)
}

fn take_action(
    state: &PokerState,
    request: &TakeActionRequest,
) -> Result<(PokerState, TakeActionResponse), ContractError> {
    let state = with_contract_state(&state, |contract: &mut PokerContract| {
        let action = match request.get_action().to_string() {
            "Check" => poker_contract::Action::Check,
            "Match" => poker_contract::Action::Match,
            "Raise" => poker_contract::Action::Raise,
            "Fold" => poker_contract::Action::Fold,
            () => poker_contract::Action::None,
        };
        contract.take_action(
            &Address::from(request.get_sender().to_string()),
            action,
            request.get_value(),
        )?;

        Ok(());
    })?;

    let response = TakeActionResponse::new();
    response.set_success(true);

    Ok(state, response)
}

fn leave(
    state: &PokerState,
    request: &WithdrawRequest,
) -> Result<(PokerState, WithdrawResponse), ContractError> {
    let mut balance = 0;
    let state = with_contract_state(&state, |contract: &mut PokerContract| {
        balance = contract.withdraw(&Address::from(request.get_sender().to_string()))?;

        Ok(())
    })?;

    let response = WithdrawResponse::new();
    response.set_success(true);
    response.set_balance(balance);

    Ok(state, response)
}

fn get_player_information(
    state: &PokerState,
    request: &PlayerStateRequest,
) -> Result<PlayerState, ContractError> {
    let contract = PokerContract::from_state(state);
    let player_state = contract.get_player_state(&Address::from(request.get_sender().to_string()));

    Ok(player_state)
}

fn get_game_information(
    state: &PokerState,
    request: &PublicStateRequest,
) -> Result<PublicState, ContractError> {
    let contract = PokerContract::from_state(state);
    let public_game_state = contract.get_public_state();

    Ok(public_game_state)
}
