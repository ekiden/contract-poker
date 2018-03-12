#![feature(use_extern_macros)]

extern crate protobuf;

extern crate ekiden_core_common;
extern crate ekiden_core_trusted;

#[macro_use]
extern crate poker_api;
extern crate rand;
extern crate rs_poker;
extern crate serde_cbor;

mod poker_contract;

use ekiden_core_common::Result;
use ekiden_core_common::contract::{with_contract_state, Address, Contract};
use ekiden_core_trusted::db::Db;
use ekiden_core_trusted::rpc::create_enclave_rpc;

with_api! {
    create_enclave_rpc!(api);
}

fn create(request: &CreateGameRequest) -> Result<(PokerState, CreateGameResponse), ContractError> {
    let contract = PokerContract::new(
        request.get_blind(),
        request.get_max_players(),
        request.get_time_per_turn(),
    );

    let response = CreateGameResponse::new();
    response.set_success(true);

    Db::instance().set("state", contract.get_state())?;

    Ok(response)
}

fn join(request: &JoinGameRequest) -> Result<(PokerState, JoinGameResponse), ContractError> {
    let state = Db::instance().get("state")?;
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

    Db::instance().set("state", state)?;

    Ok(response)
}

fn play(request: &PlayHandRequest) -> Result<(PokerState, PlayHandResponse), ContractError> {
    let state = Db::instance().get("state")?;
    let state = with_contract_state(&state, |contract: &mut PokerContract| {
        contract.play_hand(&Address::from(request.get_sender().tos_string()))?;

        Ok(())
    })?;

    let response = PlayHandResponse::new();
    response.set_success(true);

    Db::instance().set("state", state)?;

    Ok(response)
}

fn take_action(
    request: &TakeActionRequest,
) -> Result<(PokerState, TakeActionResponse), ContractError> {
    let state = Db::instance().get("state")?;
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

    Db::instance().set("state", state)?;

    Ok(response)
}

fn leave(request: &WithdrawRequest) -> Result<(PokerState, WithdrawResponse), ContractError> {
    let state = Db::instance().get("state")?;
    let mut balance = 0;
    let state = with_contract_state(&state, |contract: &mut PokerContract| {
        balance = contract.withdraw(&Address::from(request.get_sender().to_string()))?;

        Ok(())
    })?;

    let response = WithdrawResponse::new();
    response.set_success(true);
    response.set_balance(balance);

    Db::instance().set("state", state)?;

    Ok(response)
}

fn get_player_information(request: &PlayerStateRequest) -> Result<PlayerState, ContractError> {
    let state = Db::instance().get("state")?;
    let contract = PokerContract::from_state(state);
    let player_state = contract.get_player_state(&Address::from(request.get_sender().to_string()));

    Ok(player_state)
}

fn get_game_information(request: &PublicStateRequest) -> Result<PublicState, ContractError> {
    let state = Db::instance().get("state")?;
    let contract = PokerContract::from_state(state);
    let public_game_state = contract.get_public_state();

    Ok(public_game_state)
}
