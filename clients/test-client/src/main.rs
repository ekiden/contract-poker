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
use rand::{Rng};

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

    ekiden_rpc_client::FutureExtra::wait(client.create(request)).unwrap();

    let mut rng = rand::thread_rng();
    // Check balances.
    let response = ekiden_rpc_client::FutureExtra::wait(client.join({
        let mut request = poker::JoinGameRequest::new();
        request.set_sender("client1".to_string());
        request.set_deposit(5);
        request.set_seed(vec![rng.gen::<u32>(); 32]);
        request
    })).unwrap();
    assert_eq!(response.get_joined(), true);
}

/// Runs the poker scenario.
fn scenario<Backend>(client: &mut poker::Client<Backend>)
where
    Backend: ekiden_rpc_client::backend::ContractClientBackend,
{
    //Second player joins
    let mut rng = rand::thread_rng();
    let response = ekiden_rpc_client::FutureExtra::wait(client.join({
        let mut request = poker::JoinGameRequest::new();
        request.set_sender("client2".to_string());
        request.set_deposit(4);
        request.set_seed(vec![rng.gen::<u32>(), 32]);
        request
    })).unwrap();
    assert_eq!(response.get_joined(), true);

    //Start game
    let response = ekiden_rpc_client::FutureExtra::wait(client.play({
        let mut request = poker::PlayHandRequest::new();
        request.set_sender("client1".to_string());
        request
    })).unwrap();
    assert_eq!(response.get_success(), true);

    //both check
    let response = ekiden_rpc_client::FutureExtra::wait(client.take_action({
        let mut request = poker::TakeActionRequest::new();
        request.set_sender("client1".to_string());
        request.set_action("Check".to_string());
        request
    })).unwrap();
    assert_eq!(response.get_success(), true);

    let response = ekiden_rpc_client::FutureExtra::wait(client.take_action({
        let mut request = poker::TakeActionRequest::new();
        request.set_sender("client2".to_string());
        request.set_action("Check".to_string());
        request
    })).unwrap();
    assert_eq!(response.get_success(), true);

    //fold
    let response = ekiden_rpc_client::FutureExtra::wait(client.take_action({
        let mut request = poker::TakeActionRequest::new();
        request.set_sender("client1".to_string());
        request.set_action("Fold".to_string());
        request
    })).unwrap();
    assert_eq!(response.get_success(), true);
}

/// Finalize the poker scenario.
fn finalize<Backend>(client: &mut poker::Client<Backend>, _runs: usize, _threads: usize)
where
    Backend: ekiden_rpc_client::backend::ContractClientBackend,
{
    //both withdraw, verify final balance
    let response = ekiden_rpc_client::FutureExtra::wait(client.leave({
        let mut request = poker::WithdrawRequest::new();
        request.set_sender("client1".to_string());
        request
    })).unwrap();
    assert_eq!(response.get_balance(), 3);

    let response = ekiden_rpc_client::FutureExtra::wait(client.leave({
        let mut request = poker::WithdrawRequest::new();
        request.set_sender("client2".to_string());
        request
    })).unwrap();
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
