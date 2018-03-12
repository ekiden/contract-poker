#![feature(use_extern_macros)]

#[macro_use]
extern crate clap;
extern crate futures;
extern crate rand;
extern crate tokio_core;

#[macro_use]
extern crate client_utils;
extern crate ekiden_core_common;
extern crate ekiden_rpc_client;

extern crate poker_api;

use clap::{App, Arg};
use rand::{thread_rng, Rng};

use ekiden_rpc_client::create_client_rpc;
use poker_api::with_api;

with_api! {
    create_client_rpc!(poker, poker_api, api);
}

/// Initializes the poker scenario.
fn init<Backend>(client: &mut poker::Client<Backend>, _runs: usize, _threads: usize)
where
    Backend: ekiden_rpc_client::backend::ContractClientBackend,
{
    // Create new poker contract.
    let mut request = poker::CreateGameRequest::new();
    request.set_blind(2);
    request.set_max_players(4);
    request.set_time_per_turn(4);

    client.create(request).unwrap();

    // Check balances.
    let response = client
    .join({
        let mut request = poker::JoinGameRequest::new();
        request.set_sender("client1".to_string());
        request.set_deposit(5);
        request.set_seed(3);
        request
    })
    .unwrap();
    assert_eq!(response.get_joined(), 1);
}

/// Runs the poker scenario.
fn scenario<Backend>(client: &mut poker::Client<Backend>)
where
    Backend: ekiden_rpc_client::backend::ContractClientBackend,
{
    //Second player joins
    let response = client
    .join({
        let mut request = poker::JoinGameRequest::new();
        request.set_sender("client2".to_string());
        request.set_deposit(4);
        request.set_seed(2);
        request
    })
    .unwrap();
    assert_eq!(response.get_joined(), 1);

    //Start game
    let response = client
    .play({
        let mut request = poker::PlayHandRequest::new();
        request.set_sender("client1".to_string());
        request
    })
    .unwrap();
    assert_eq!(response.get_success(), 1);

    //both check
    let response = client
    .take_action({
        let mut request = poker::TakeActionRequest::new();
        request.set_sender("client1".to_string());
        request.set_action("Check".to_string());
        request
    })
    .unwrap();
    assert_eq!(response.get_success(), 1);

    let response = client
    .take_action({
        let mut request = poker::TakeActionRequest::new();
        request.set_sender("client2".to_string());
        request.set_action("Check".to_string());
        request
    })
    .unwrap();
    assert_eq!(response.get_success(), 1);

    //fold
    let response = client
    .take_action({
        let mut request = poker::TakeActionRequest::new();
        request.set_sender("client1".to_string());
        request.set_action("Fold".to_string());
        request
    })
    .unwrap();
    assert_eq!(response.get_success(), 1);
}

/// Finalize the poker scenario.
fn finalize<Backend>(client: &mut poker::Client<Backend>, runs: usize, _threads: usize)
where
    Backend: ekiden_rpc_client::backend::ContractClientBackend,
{
    //both withdraw, verify final balance
    let response = client
    .withdraw({
        let mut request = poker::WithdrawRequest::new();
        request.set_sender("client1".to_string());
        request
    })
    .unwrap();
    assert_eq!(response.get_balance(), 3);

    let response = client
    .withdraw({
        let mut request = poker::WithdrawRequest::new();
        request.set_sender("client2".to_string());
        request
    })
    .unwrap();
    assert_eq!(response.get_balance(), 6);
}

#[cfg(feature = "benchmark")]
fn main() {
    let results = benchmark_client!(poker, init, scenario, finalize);
    results.show();
}

#[cfg(not(feature = "benchmark"))]
fn main() {
    let mut client = contract_client!(poker);
    init(&mut client, 1, 1);
    scenario(&mut client);
    finalize(&mut client, 1, 1);
}
