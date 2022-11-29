// Find all our documentation at https://docs.near.org
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Serialize, Deserialize};
use near_sdk::{near_bindgen, AccountId, Timestamp, env, Balance, Promise, require};
use near_sdk::collections::{UnorderedMap, Vector, UnorderedSet};
use near_sdk::env::random_seed_array;

const  WAITING_TIME: u64 = 1800000000000;

 

// Define the contract structure
#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct CoinFlip {
    games: UnorderedMap<u64, Game>,
    users: UnorderedMap<AccountId, User>,
    active_games: UnorderedSet<u64>,
    white_list: UnorderedSet<AccountId>,
    owner_id: AccountId,
    web_id: Option<AccountId>,
    royalty: u128
}



#[derive(BorshSerialize, BorshDeserialize)]
pub struct Game{
    initiator: AccountId,
    enemy: Option<AccountId>,
    init_side: Side,
    bid: Balance,
    time_start: Timestamp,
    time_bid: Option<Timestamp>,
    rezult: Option<Side>,
    cooldown: bool
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct ActiveGames{
    initiator: AccountId,
    init_side: SideIn,
    bid: Balance,
    time_start: Timestamp,
    key: u64
}
#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct GetGame{
    initiator: AccountId,
    enemy: Option<AccountId>,
    init_side: SideIn,
    bid: Balance,
    time_start: Timestamp,
    time_bid: Option<Timestamp>,
    rezult: Option<SideIn>,
    cooldown: bool,
    key: u64

}

#[derive(BorshSerialize, BorshDeserialize,PartialEq)]
enum Side {
    Tail,
    Head
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub enum SideIn{
    Tail,
    Head
}

#[derive(BorshDeserialize,BorshSerialize)]
pub struct User{
    init_games: Vector<u64>,
    bids: Vector<u64>,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct  UserGames{
    init_games_list: Vec<GetGame>,
    bids_list: Vec<GetGame>
}

// Define the default, which automatically initializes the contract
impl Default for CoinFlip{
    fn default() -> Self{
        Self { games: UnorderedMap::new(b"g"), users: UnorderedMap::new(b"u"), active_games: UnorderedSet::new(b"a"), white_list: UnorderedSet::new(b"w"), owner_id: env::predecessor_account_id(), web_id: None, royalty: 3}    
    }
}

// Implement the contract structure
#[near_bindgen]
impl CoinFlip {
    // Public method - returns the greeting saved, defaulting to DEFAULT_MESSAGE
    
    fn play_game(&mut self, key: u64){
        let seed = random_seed_array().to_vec();
        let rezult = seed[0]%2;
        let data = self.games.get(&key).unwrap();
        match rezult {
            0=>self.games.get(&key).unwrap().rezult = Some(Side::Tail),
            1=>self.games.get(&key).unwrap().rezult = Some(Side::Head),
            default=> panic!("contract error")
        }
        self.active_games.remove(&key);
        if data.rezult.unwrap() == data.init_side{
            Promise::new(data.initiator.clone()).transfer(data.bid*(100-self.royalty)/50);
        }else{Promise::new(data.enemy.unwrap().clone()).transfer(data.bid*(100-self.royalty)/50);}
    }
    
    pub fn create_game(&mut self, side_in: SideIn, bid: u128){
        require!(env::attached_deposit() == bid, "Attached deposit must be equal to bid");
        if self.users.get(&env::predecessor_account_id()).is_none() {
            let value = &User { init_games: Vector::new(b"l"), bids: Vector::new(b"p") };
            self.users.insert(&env::predecessor_account_id(), value);            
        }    
        
            let side: Side;
            
                match side_in {
                    SideIn::Tail=>side=Side::Tail,
                    SideIn::Head=>side=Side::Head
                }
                let mut x: bool = false;
                let mut key_true: Option<u64> = None;
                let mut cooldown_list: Vec<u64> = Vec::new();
                for k in self.active_games.iter() {
                    let el =  self.games.get(&k).expect("Game not found");
                    if env::block_timestamp()-el.time_start > WAITING_TIME{
                        self.games.get(&k).unwrap().cooldown = false;
                        cooldown_list.push(k);

                    }else{
                        if bid == el.bid && x == false{
                            if el.init_side != side{
                                self.games.get(&k).unwrap().enemy = Some(env::predecessor_account_id());
                                self.games.get(&k).unwrap().time_bid =Some(env::block_timestamp());
                                x = true;
                                key_true = Some(k);
                            }
                        }
                    }
                }
                for k in cooldown_list{
                    self.active_games.remove(&k);
                } 
            if x == true{    
                self.users.get(&env::predecessor_account_id()).unwrap().bids.push(&key_true.unwrap());
                self.play_game(key_true.unwrap());
                
            }else{
            let id: u64 = self.games.len();
            let new_game: Game = Game{
                initiator: env::predecessor_account_id(),
                enemy: None,
                init_side: side,
                bid: env::attached_deposit(),
                time_start: env::block_timestamp(),
                time_bid: None,
                rezult: None,
                cooldown: true
            };
            self.games.insert(&id, &new_game);
            self.users.get(&env::predecessor_account_id()).unwrap().init_games.push(&id);
            }
    
    }

    pub fn answer_to_bid(&mut self, key: u64, bid: u128){
        require!(self.active_games.contains(&key), "Game non active");
        if self.users.get(&env::predecessor_account_id()).is_none() {
            let value = &User { init_games: Vector::new(b"l"), bids: Vector::new(b"p") };
            self.users.insert(&env::predecessor_account_id(), value);            
        }
        let game = self.games.get(&key).expect("Game not found");
        require!(bid == env::attached_deposit() && bid == game.bid, "bid not correct"); 
            self.games.get(&key).unwrap().enemy = Some(env::predecessor_account_id());
            self.games.get(&key).unwrap().time_bid =Some(env::block_timestamp());
            self.users.get(&env::predecessor_account_id()).unwrap().bids.push(&key);
            self.play_game(key);
        
    }

    ///////////////////ADMIN COMANDS//////////////////////
    /// 
    /// 
    /// //////////////////////////////////////////////////

    pub fn set_webid(&mut self, web_id: String){
        require!(env::predecessor_account_id() == self.owner_id || self.white_list.contains(&env::predecessor_account_id()), "You are not admin");
        self.web_id = Some(AccountId::new_unchecked(String::from(web_id)));
    }
    
    pub fn add_admin_to_list(&mut self, admin: String){
        require!(env::predecessor_account_id() == self.owner_id || self.white_list.contains(&env::predecessor_account_id()), "You are not admin");
            self.white_list.insert(&AccountId::new_unchecked(String::from(admin)));
    }

    pub fn remove_admin_to_list(&mut self, admin: String){
        require!(env::predecessor_account_id() == self.owner_id || self.white_list.contains(&env::predecessor_account_id()), "You are not admin");
            self.white_list.remove(&AccountId::new_unchecked(String::from(admin)));
    }

    pub fn admin_withdraw(&mut self, ammount: u128){
        require!(self.white_list.contains(&env::predecessor_account_id()), "you are not admin");
            require!(ammount<=env::account_balance(), "it's to much to withdraw");
                Promise::new(env::predecessor_account_id().clone()).transfer(ammount);    
            
    }

    pub fn change_royalty(&mut self, royalty: u128){
        require!(env::predecessor_account_id() == self.owner_id || self.white_list.contains(&env::predecessor_account_id()), "You are not admin");
            require!(royalty < 100, "Royalty can't be more then 100%");    
                self.royalty = royalty;
    }

    //////////////////////Web Site command///////////////////////////////
    /// 
    /// 
    /////////////////////////////////////////////////////////////////////
    
    pub fn check_timer(&mut self){
        let mut cooldown_list = Vec::new();
        for k in self.active_games.iter() {
            let el =  self.games.get(&k).expect("Game not found");
            if env::block_timestamp()-el.time_start > WAITING_TIME{
                self.games.get(&k).unwrap().cooldown = false;
                cooldown_list.push(k);
            }
        }
        for k in cooldown_list{
            self.active_games.remove(&k);
        }
    }


    //////////////////////View functions/////////////////////////////////
    /// 
    /// 
    /////////////////////////////////////////////////////////////////////
    
    pub fn get_active_list(&self) -> Vec<ActiveGames>{
        let mut list: Vec<ActiveGames> = Vec::new();
        for k in self.active_games.iter(){
            let game = self.games.get(&k).unwrap(); 
            let side: SideIn;
            
                match game.init_side {
                    Side::Tail=>side=SideIn::Tail,
                    Side::Head=>side=SideIn::Head
                }
            let el: ActiveGames = ActiveGames{
                initiator: game.initiator,
                init_side: side,
                bid: game.bid,
                time_start: game.time_start,
                key: k
            };
            list.push(el);
        }
        return list;
    }

    
    pub fn get_game_datails(&self, key: u64)->GetGame{
        let wgame = self.games.get(&key);
        require!(wgame.is_some() , "game not found");
        let side: SideIn;
        let game = wgame.unwrap();
        match game.init_side {
            Side::Tail=>side=SideIn::Tail,
            Side::Head=>side=SideIn::Head
        }
        let mut rezult: Option<SideIn>=None;
        if game.rezult.is_some(){
            match game.rezult.unwrap() {
                Side::Tail=>rezult=Some(SideIn::Tail),
                Side::Head=>rezult=Some(SideIn::Head)
            }   
        }

        let get_game = GetGame { 
            initiator: game.initiator,
            enemy: game.enemy, 
            init_side: side, 
            bid: game.bid, 
            time_start: game.time_start, 
            time_bid: game.time_bid, 
            rezult: rezult, 
            cooldown: game.cooldown, 
            key: key 
        };

        return get_game;
    }

    /// .
    ///
    /// # Panics
    ///
    /// Panics if .
   pub fn get_user_games_list(&self)->UserGames{
        let mut games: UserGames = UserGames{
            init_games_list: Vec::new(),
            bids_list: Vec::new()
        };
        if self.users.get(&env::predecessor_account_id()).is_some(){
            let inits = self.users.get(&env::predecessor_account_id()).unwrap().init_games;
            let bids = self.users.get(&env::predecessor_account_id()).unwrap().bids;
            for k in inits.iter(){
                let game = self.games.get(&k).unwrap();
                let side;
                match game.init_side {
                    Side::Tail=>side=SideIn::Tail,
                    Side::Head=>side=SideIn::Head
                }
                let mut rezult: Option<SideIn>=None;
                if game.rezult.is_some(){
                    match game.rezult.unwrap() {
                        Side::Tail=>rezult=Some(SideIn::Tail),
                        Side::Head=>rezult=Some(SideIn::Head)
                    }   
                }
                games.init_games_list.push(GetGame { 
                    initiator: game.initiator,
                    enemy: game.enemy, 
                    init_side: side, 
                    bid: game.bid, 
                    time_start: game.time_start, 
                    time_bid: game.time_bid, 
                    rezult: rezult, 
                    cooldown: game.cooldown, 
                    key: k 
                });
            }
            for k in bids.iter(){
                let game = self.games.get(&k).unwrap();
                let side;
                match game.init_side {
                    Side::Tail=>side=SideIn::Tail,
                    Side::Head=>side=SideIn::Head
                }
                let mut rezult: Option<SideIn>=None;
                if game.rezult.is_some(){
                    match game.rezult.unwrap() {
                        Side::Tail=>rezult=Some(SideIn::Tail),
                        Side::Head=>rezult=Some(SideIn::Head)
                    }   
                }
                games.bids_list.push(GetGame { 
                    initiator: game.initiator,
                    enemy: game.enemy, 
                    init_side: side, 
                    bid: game.bid, 
                    time_start: game.time_start, 
                    time_bid: game.time_bid, 
                    rezult: rezult, 
                    cooldown: game.cooldown, 
                    key: k 
                });
            }
        }else{
            games.bids_list.clear();
            games.init_games_list.clear();
        }
        return games;
    }
}
    // Public method - accepts a greeting, such as "howdy", and records it
    


/*
 * The rest of this file holds the inline tests for the code above
 * Learn more about Rust tests: https://doc.rust-lang.org/book/ch11-01-writing-tests.html
 */

