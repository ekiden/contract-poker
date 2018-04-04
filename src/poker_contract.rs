//An implementation of Texas Hold'em compatible with Ekiden
#![no_std]
use ekiden_core_common::{Address, Contract};

use poker_api::{PlayerState, PokerState, PublicState};
use rs_poker::core::{Card, Deck, Hand, Rankable};
use rand::*;
use serde_cbor;
use core::slice::Iter;
use std::collections::HashMap;

pub struct PokerContract<'a> {
    game_id: u64,
    blind: u64,
    max_players: u64,
    time_per_turn: u64,
    players: Vec<Player>,
    on_deck: Vec<Player>,
    index: HashMap<String, i32>,
    cards: Vec<Card>,
    deck: &'a Iter<'a, Card>,
    pot: u64,
    min_bet: u64,
    dealer: i32,
    next_player: i32,
    last_player: i32,
    stage: GameStage,
    seed: [u8; 32],
}

//TODO: how to index players and get the right one
//TODO: winning logic
//TODO: lookup table???
//TODO: check shuffle flow
//TODO: serialization stuff

impl<'a> PokerContract<'a> {
    //Creates a new instance of a poker game with all values set to default
    //save for provided parameters
    pub fn new(blind: u64, max_players: u64, time_per_turn: u64) -> Result<PokerContract<'a>> {
        if max_players > 22 || blind == 0 || time_per_turn == 0 {
            return Err(ContractError::new("Invalid game paramaters."));
        }

        //TODO: Review if this is the game state that is trying to be returned.
        return Ok(PokerContract {
            game_id: blind + max_players + time_per_turn,
            blind,
            max_players,
            time_per_turn,
            players: Vec::new(),
            on_deck: Vec::new(),
            cards: Vec::new(),
            deck: Deck::default().iter(),
            pot: 0,
            min_bet: 0,
            dealer: -1,
            last_player: 0,
            state: GameStage::Join,
            seed: [0; 32],
        });
    }

    //Allows a player to join a game. If a hand is being played, the player is placed `on_deck`
    pub fn join_game(
        &mut self,
        msg_sender: &Address,
        deposit: u64,
        seed: [u8; 32],
    ) -> Result<bool> {
        //Validate the seed. (Might not need this)
        if seed.len() != 32 {
            return Err(ContractError::new("Invalid format for the random seed."));
        }
        for i in 0..32 {
            self.seed[i] = self.seed[i] ^ seed[i];
        }
        //Initialize the new player.
        let new_player = Player {
            addr: msg_sender.clone(),
            cards: Vec::new(),
            action: Action::None,
            playing: false,
            bet: 0,
            balance: deposit,
        };
        //Check that the new player is not already in the game
        for player in self.players.iter() {
            if msg_sender == player.addr {
                return Err(ContractError::new("Player is already in the table."));
            }
        }
        //Check that the new player is not already on deck
        for waiting in self.on_deck.iter() {
            if msg_sender == waiting.addr {
                return Err(ContractError::new("Player is already on deck."));
            }
        }
        //Take action based on game stage.
        match self.stage {
            GameStage::Join => {
                if self.player.len() < self.max_players {
                    self.players.push(new_player);
                    self.index[new_player.addr.to_string()] = self.players.len();
                    return Ok(true);
                } else {
                    self.on_deck.push(new_player);
                    self.index[new_player.addr.to_string()] = -1;
                    return Ok(false);
                }
            }
            GameStage::Play => {
                self.on_deck.push(new_player);
                self.index[new_player.addr.to_string()] = -1;
                return Ok(false);
            }
        }
    }

    //Initiates the start of the hand, provided that there is more than one player
    //joined in the game.
    pub fn play_hand(&mut self, msg_sender: &Address) -> Result<()> {
        if self.stage != GameStage::Join {
            return Err(ContractError::new(
                "Cannot call `play_hand` if the game is not in the `Join` stage.",
            ));
        }
        //Add players on deck up to the maximum number of players allowed
        for i in 0..(self.max_players - self.players.len()) {
            self.players.append(self.on_deck.remove(i));
        }
        //Check there are at least 2 players
        if self.players.len() < 2 {
            return Err(ContractError::new(
                "Cannot call 'play_hand' with less than 2 players.",
            ));
        }
        //Shuffle the cards.
        let mut deck = Deck::default().iter().collect();
        let mut rng: XorShiftRng = SeedableRng::from_seed(&self.seed);
        rng.shuffle(&mut deck);
        self.deck = deck.into_iter();

        //Set the dealer
        self.dealer = (self.dealer + 1) % self.players.len();

        //Pay small and big blinds
        let small_blind_player = (self.dealer + 1) % self.players.len();
        let big_blind_player = (self.small_blind_player + 1) % self.players.len();
        self.players[small_blind_player].bet = self.blind / 2;
        self.players[small_blind_player].balance -= self.blind / 2;
        self.players[big_blind_player].bet = self.blind;
        self.players[big_blind_player].balance -= self.blind;
        self.min_bet = self.blind;

        //Deal cards
        let start = (self.dealer + 1) % self.players.len();
        for i in start..(start + 2 * self.players.len()) {
            let player = self.players[i % self.players.len()];
            let next = self.deck.next;
            match next {
                Some(card) => player.cards.push(card),
                () => Err(ContractError::new("Error dealing cards")),
            }?;
        }

        //Set the turn to the next player
        self.next_player = (big_blind_player + 1) % self.players.len();
        self.last_player = big_blind_player % self.players.len();

        //Update game stage to `Play`
        self.stage = GameStage::Play;
        return Ok(());
    }

    //Allows a player to take an action or store an action to be made when it is a players turn.
    //The last player in line will initiate the drawing of the next cards.
    //Illegal actions return a contract error
    pub fn take_action(&mut self, msg_sender: &Address, action: Action, value: u64) -> Result<()> {
        if self.stage != GameStage::Play {
            return Err(ContractError::new(
                "Cannot call `take_action` if the game is not in the `Play` stage.",
            ));
        }
        if self.index[msg_sender.to_string()] != self.next_player {
            return Err(ContractError::new("Out of turn"));
        }
        let player_index = self.index[msg_sender.to_string()];
        match action {
            Action::None => {
                return Err(ContractError::new("Invalid action."));
            }
            Action::Check => {
                if self.min_bet != 0 {
                    return Err(ContractError::new("Invalid move."));
                }
            }
            Action::Raise => {
                if value <= self.min_bet * 2 {
                    return Err(ContractError::new(
                        "Invalid raise. Must raise by two times the minimum bet.",
                    ));
                }
                self.min_bet = value;
                self.players[player_index].bet = value;
                self.players[player_index].deposit -= value - self.players[player_index].bet;
                self.last_player = msg_sender;
            }
            Action::Match => {
                self.players[player_index].bet = self.min_bet;
                self.players[player_index].deposit -= value - self.players[player_index].bet;
            }
            Action::Fold => {
                self.fold_player(player_index)?;
            }
        }
        self.next_player = (self.next_player + 1) % self.players.len();

        if msg_sender == self.last_player && action != Action::Raise {
            //Put all bets into the pot
            for player in self.players.iter() {
                self.pot += player.bet;
                player.bet = 0;
            }
            //Finalize round or turn card
            if self.cards.len() == 8 {
                self.pay_winners();
                self.stage == GameStage::Join;
            } else {
                self.turn_card()?;
            }
        }
    }

    //Allows a player to leave the game with his or her final balance.
    //If a player is in the middle of the hand, his or her cards are folded.
    //Returns the player's final balance
    pub fn withdraw(&mut self, msg_sender: &Address) -> Result<u64> {
        //Remove player from current hand.
        let player_index = self.index[msg_sender.to_string()];
        if player_index > -1 {
            //Fold cards
            return self.fold_player(player_index)?;
        } else {
            //Remove player from waiting
            for i in 0..self.on_deck.len() {
                if msg_sender == self.on_deck[i].addr {
                    return Ok(self.on_deck.remove(i).balance);
                }
            }
        }
        return Err(ContractError::new("This player has not joined the game."));
    }

    //+++++++++++++++++++++++++++++++++++++++++++++++++++++
    // HELPER FUNCTIONS
    //+++++++++++++++++++++++++++++++++++++++++++++++++++++

    fn turn_card(&mut self) -> Result<()> {
        self.deck.next();
        let next = self.deck.next();
        match next {
            Some(card) => {
                self.cards.push(card);
                Ok(())
            }
            () => Err(ContractError::new(
                "Error in turning the cards. Deck is empty.",
            )),
        }
    }

    fn fold_player(&mut self, player_index: i32) -> Result<u64> {
        self.pot += self.players[player_index].bet;
        self.players[player_index].bet = 0;
        for i in player_index + 1..self.players.len() {
            self.index[self.players[i].addr.to_string()] -= 1;
        }
        Ok(self.players.remove(player_index).balance)
    }

    fn pay_winners(&mut self) -> Result<()> {
        let winners: Vec<Player> = Vec::new();
        let max = 0;
        for player in self.players.iter() {
            let rank = Hand::new_with_cards(&player.cards).rank();
            if rank > max {
                winners.clear();
                winners.push(player);
                max = rank;
            } else if rank == max {
                winners.push(player);
            }
        }
        for player in self.winners.iter() {
            player.balance += self.pot / winners.len();
        }
        self.pot = 0;
        Ok(())
    }

    //+++++++++++++++++++++++++++++++++++++++++++++++++++++
    // FUNCTIONS TO REQUEST AND FORMAT STATE
    //+++++++++++++++++++++++++++++++++++++++++++++++++++++

    pub fn get_public_state(&mut self) -> Result<PublicState> {
        let mut state = PublicState::new();

        state.set_game_id(self.game_id);
        state.set_blind(self.blind);
        state.set_max_players(self.max_players);
        state.set_players(self.serialize_players(&self.players));
        state.set_on_deck(self.serialize_players(&self.on_deck));
        state.set_pot(self.pot);
        state.set_min_bet(self.min_bet);
        state.set_dealer(self.dealer);
        state.set_next_player(self.next_player);
        state.set_last_player(self.set_last_player);
        state.set_stage(self.stage);

        Ok(state)
    }

    fn get_player_state(&mut self, msg_sender: &Address) -> Result<PlayerState> {
        let player = self.players[0];
        let mut state = PlayerState::new();

        state.set_addr(player.addr.to_string());
        state.set_action(player.action.to_string());
        state.set_cards(serde_cbor::to_vec(&player.cards).expect("Unable to serialize cards."));
        state.set_playing(player.playing);
        state.set_bet(player.bet);
        state.set_balance(player.balance);

        Ok(state)
    }

    pub fn from_player_state(state: &PlayerState) -> Player {}

    fn serialize_players(&mut self, players: &Vec<Player>) -> Vec<PlayerState> {
        let formatted_players: Vec<PlayerState> = Vec::new();
        for player in players {
            let state = self.get_player_state(&player.addr).unwrap();
            formatted_players.push(state);
        }
        return formatted_players;
    }
}

impl<'a> Contract<PokerState> for PokerContract<'a> {
    /// Get serializable contract state.
    fn get_state(&self) -> PokerState {
        let mut state = PokerState::new();
        state.set_game_id(self.game_id);
        state.set_blind(self.blind);
        state.set_max_players(self.max_players);
        state.set_players(self.serialize_players(&self.players));
        state.set_on_deck(self.serialize_players(&self.on_deck));
        state.set_cards(serde_cbor::to_vec(&self.cards).expect("Unable to serialze cards."));
        state.set_deck(serde_cbor::to_vec(&self.deck).expect("Unable to serialze deck."));
        state.set_pot(self.pot);
        state.set_min_bet(self.min_bet);
        state.set_dealer(self.dealer);
        state.set_next_player(self.next_player);
        state.set_last_player(self.last_player);
        state.set_stage(self.stage.to_string());
        state.set_seed(self.seed.clone());

        state
    }

    /// Create contract instance from serialized state.
    fn from_state(state: &PokerState) -> PokerContract {
        PokerContract {
            game_id: state.get_name(),
            blind: state.get_blind(),
            max_players: state.get_max_players(),
            time_per_turn: state.get_time_per_turn(),
            players: state.get_players().clone(),
            on_deck: state.get_on_deck().clone(),
            cards: serde_cbor::from_slice(state.get_cards()).expect("Unable to deserialize cards"),
            deck: serde_cbor::from_slice(state.get_deck()).expect("Unable to deserialize deck"),
            pot: state.get_pot(),
            min_bet: state.get_min_bet(),
            dealer: state.get_dealer(),
            next_player: state.get_next_player(),
            last_player: state.get_last_player(),
            stage: GameStage::from_string(state.get_stage()),
            seed: state.get_seed().clone(),
        }
    }
}

//++++++++++++++++++++++++++++++++++++++++++++++
// EXTRA STRUCTS AND ENUMS
//++++++++++++++++++++++++++++++++++++++++++++++

pub struct Player {
    addr: Address,
    cards: Vec<Card>,
    action: Action,
    playing: bool,
    bet: u64,
    balance: u64,
}

enum GameStage {
    Join,
    Play,
}

impl GameStage {
    fn to_string(&self) -> String {
        match self {
            GameStage::Join => return "Join",
            GameStage::Play => return "Play",
        }
    }

    fn from_string(string: &String) -> GameStage {
        match string {
            "Join" => GameStage::Join,
            "Play" => GameStage::Play,
        }
    }
}

pub enum Action {
    None,
    Check,
    Match,
    Raise,
    Fold,
}

impl Action {
    fn to_string(&self) -> String {
        match self {
            Action::None => "None",
            Action::Check => "Check",
            Action::Match => "Match",
            Action::Bet => "Bet",
            Action::Fold => "Fold",
        }
    }

    fn from_string(string: &String) -> Action {
        match string {
            "None" => Action::None,
            "Check" => Action::Check,
            "Match" => Action::Match,
            "Bet" => Action::Bet,
            "Fold" => Action::Fold,
        }
    }
}
