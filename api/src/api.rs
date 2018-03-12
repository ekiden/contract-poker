rpc_api! {
    metadata {
        name = poker;
        version = "0.1.0";
        client_attestation_required = false;
    }

    rpc create(CreateGameRequest) -> (CreateGameResponse);

    rpc join(JoinGameRequest) -> (JoinGameResponse);

    rpc play(PlayHandRequest) -> (PlayHandResponse);

    rpc take_action(TakeActionRequest) -> (TakeActionResponse);

    rpc leave(WithdrawRequest) -> (WithdrawResponse);

}
