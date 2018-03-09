rpc_api! {
    metadata {
        name = poker;
        version = "0.1.0";
        state_type = PokerState;
        client_attestation_required = false;
    }

    rpc create(CreateGameRequest) -> (PublicState, CreateGameResponse);

    rpc join(state, JoinGameRequest) -> (PublicState, JoinGameResponse);

    rpc play(state, PlayHandRequest) -> (PublicState, PlayHandResponse);

    rpc take_action(state, TakeActionRequest) -> (PublicState, TakeActionResponse);

    rpc leave(state, WithdrawRequest) -> (PublicState, WithdrawResponse);

}
