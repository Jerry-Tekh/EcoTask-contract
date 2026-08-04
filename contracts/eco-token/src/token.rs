use crate::storage;
use soroban_sdk::{contract, contractevent, contractimpl, Address, Env, String};

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MintEvent {
    #[topic]
    pub admin: Address,
    #[topic]
    pub to: Address,
    pub amount: i128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransferEvent {
    #[topic]
    pub from: Address,
    #[topic]
    pub to: Address,
    pub amount: i128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BurnEvent {
    #[topic]
    pub from: Address,
    pub amount: i128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransferFromEvent {
    #[topic]
    pub from: Address,
    #[topic]
    pub to: Address,
    #[topic]
    pub spender: Address,
    pub amount: i128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApproveEvent {
    #[topic]
    pub owner: Address,
    #[topic]
    pub spender: Address,
    pub amount: i128,
    pub expiration_ledger: u32,
}

#[contract]
pub struct TokenContract;

#[contractimpl]
impl TokenContract {
    pub fn initialize(e: Env, admin: Address, name: String, symbol: String, decimal: u32) {
        if storage::has_admin(&e) {
            panic!("token: already initialized");
        }
        storage::write_admin(&e, &admin);
        storage::write_minter(&e, &admin);
        storage::write_metadata(&e, &name, &symbol, &decimal);
        storage::write_supply(&e, 0);
    }

    pub fn mint(e: Env, to: Address, amount: i128) {
        let minter = storage::read_minter(&e);
        minter.require_auth();

        if amount <= 0 {
            panic!("token: amount must be positive");
        }

        let balance = storage::read_balance(&e, &to);
        storage::write_balance(
            &e,
            &to,
            balance.checked_add(amount).expect("balance overflow"),
        );

        let supply = storage::read_supply(&e);
        storage::write_supply(&e, supply.checked_add(amount).expect("supply overflow"));

        MintEvent {
            admin: minter,
            to: to.clone(),
            amount,
        }
        .publish(&e);
    }

    pub fn transfer(e: Env, from: Address, to: Address, amount: i128) {
        from.require_auth();

        if amount <= 0 {
            panic!("token: amount must be positive");
        }

        let from_balance = storage::read_balance(&e, &from);
        if from_balance < amount {
            panic!("token: insufficient balance");
        }

        storage::write_balance(
            &e,
            &from,
            from_balance.checked_sub(amount).expect("balance underflow"),
        );

        let to_balance = storage::read_balance(&e, &to);
        storage::write_balance(
            &e,
            &to,
            to_balance.checked_add(amount).expect("balance overflow"),
        );

        TransferEvent {
            from: from.clone(),
            to: to.clone(),
            amount,
        }
        .publish(&e);
    }

    pub fn balance(e: Env, id: Address) -> i128 {
        storage::read_balance(&e, &id)
    }

    pub fn total_supply(e: Env) -> i128 {
        storage::read_supply(&e)
    }

    pub fn name(e: Env) -> String {
        storage::read_name(&e)
    }

    pub fn symbol(e: Env) -> String {
        storage::read_symbol(&e)
    }

    pub fn decimal(e: Env) -> u32 {
        storage::read_decimal(&e)
    }

    pub fn admin(e: Env) -> Address {
        storage::read_admin(&e)
    }

    pub fn transfer_admin(e: Env, current_admin: Address, new_admin: Address) {
        current_admin.require_auth();
        let stored_admin = storage::read_admin(&e);
        if current_admin != stored_admin {
            panic!("token: unauthorized");
        }
        if new_admin == current_admin {
            panic!("token: new admin must be different");
        }
        storage::write_admin(&e, &new_admin);
    }

    pub fn minter(e: Env) -> Address {
        storage::read_minter(&e)
    }

    pub fn set_minter(e: Env, caller: Address, new_minter: Address) {
        caller.require_auth();
        let admin = storage::read_admin(&e);
        if caller != admin {
            panic!("token: unauthorized");
        }
        storage::write_minter(&e, &new_minter);
    }

    pub fn burn(e: Env, from: Address, amount: i128) {
        from.require_auth();

        if amount <= 0 {
            panic!("token: amount must be positive");
        }

        let balance = storage::read_balance(&e, &from);
        if balance < amount {
            panic!("token: insufficient balance");
        }

        storage::write_balance(
            &e,
            &from,
            balance.checked_sub(amount).expect("balance underflow"),
        );

        let supply = storage::read_supply(&e);
        storage::write_supply(&e, supply.checked_sub(amount).expect("supply underflow"));

        BurnEvent {
            from: from.clone(),
            amount,
        }
        .publish(&e);
    }

    pub fn approve(e: Env, owner: Address, spender: Address, amount: i128, expiration_ledger: u32) {
        owner.require_auth();

        let allowance = storage::Allowance {
            amount,
            expiration_ledger,
        };
        storage::write_allowance(&e, &owner, &spender, &allowance);

        ApproveEvent {
            owner,
            spender,
            amount,
            expiration_ledger,
        }
        .publish(&e);
    }

    pub fn allowance(e: Env, owner: Address, spender: Address) -> i128 {
        match storage::read_allowance(&e, &owner, &spender) {
            Some(a) => {
                if a.expiration_ledger < e.ledger().sequence() {
                    0
                } else {
                    a.amount
                }
            }
            None => 0,
        }
    }

    pub fn transfer_from(e: Env, spender: Address, from: Address, to: Address, amount: i128) {
        spender.require_auth();

        if amount <= 0 {
            panic!("token: amount must be positive");
        }

        let allowance = match storage::read_allowance(&e, &from, &spender) {
            Some(a) => {
                if a.expiration_ledger < e.ledger().sequence() {
                    panic!("token: allowance expired");
                }
                a
            }
            None => panic!("token: allowance not found"),
        };

        if allowance.amount < amount {
            panic!("token: insufficient allowance");
        }

        storage::spend_allowance(&e, &from, &spender, amount);

        let from_balance = storage::read_balance(&e, &from);
        if from_balance < amount {
            panic!("token: insufficient balance");
        }

        storage::write_balance(
            &e,
            &from,
            from_balance.checked_sub(amount).expect("balance underflow"),
        );

        let to_balance = storage::read_balance(&e, &to);
        storage::write_balance(
            &e,
            &to,
            to_balance.checked_add(amount).expect("balance overflow"),
        );

        TransferFromEvent {
            from,
            to,
            spender: spender.clone(),
            amount,
        }
        .publish(&e);
    }
}

#[cfg(test)]
mod test {
    use crate::{TokenContract, TokenContractClient};
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::{Address, Env, String};

    #[test]
    fn test_initialize_and_metadata() {
        let e = Env::default();
        let admin = Address::generate(&e);
        let contract_id = e.register(TokenContract, ());
        let client = TokenContractClient::new(&e, &contract_id);

        client.initialize(
            &admin,
            &String::from_str(&e, "ECO"),
            &String::from_str(&e, "ECO"),
            &7,
        );

        assert_eq!(client.name(), String::from_str(&e, "ECO"));
        assert_eq!(client.symbol(), String::from_str(&e, "ECO"));
        assert_eq!(client.decimal(), 7);
        assert_eq!(client.total_supply(), 0);
    }

    #[test]
    fn test_mint_and_balance() {
        let e = Env::default();
        let admin = Address::generate(&e);
        let user = Address::generate(&e);
        let contract_id = e.register(TokenContract, ());
        let client = TokenContractClient::new(&e, &contract_id);

        client.initialize(
            &admin,
            &String::from_str(&e, "ECO"),
            &String::from_str(&e, "ECO"),
            &7,
        );

        e.mock_all_auths();
        client.mint(&user, &1000);

        assert_eq!(client.balance(&user), 1000);
        assert_eq!(client.total_supply(), 1000);
    }

    #[test]
    fn test_transfer() {
        let e = Env::default();
        let admin = Address::generate(&e);
        let from = Address::generate(&e);
        let to = Address::generate(&e);
        let contract_id = e.register(TokenContract, ());
        let client = TokenContractClient::new(&e, &contract_id);

        client.initialize(
            &admin,
            &String::from_str(&e, "ECO"),
            &String::from_str(&e, "ECO"),
            &7,
        );

        e.mock_all_auths();
        client.mint(&from, &500);
        client.transfer(&from, &to, &300);

        assert_eq!(client.balance(&from), 200);
        assert_eq!(client.balance(&to), 300);
    }

    #[test]
    #[should_panic(expected = "token: amount must be positive")]
    fn test_mint_zero_amount() {
        let e = Env::default();
        let admin = Address::generate(&e);
        let user = Address::generate(&e);
        let contract_id = e.register(TokenContract, ());
        let client = TokenContractClient::new(&e, &contract_id);

        client.initialize(
            &admin,
            &String::from_str(&e, "ECO"),
            &String::from_str(&e, "ECO"),
            &7,
        );

        e.mock_all_auths();
        client.mint(&user, &0);
    }

    #[test]
    #[should_panic(expected = "token: insufficient balance")]
    fn test_transfer_insufficient_balance() {
        let e = Env::default();
        let admin = Address::generate(&e);
        let from = Address::generate(&e);
        let to = Address::generate(&e);
        let contract_id = e.register(TokenContract, ());
        let client = TokenContractClient::new(&e, &contract_id);

        client.initialize(
            &admin,
            &String::from_str(&e, "ECO"),
            &String::from_str(&e, "ECO"),
            &7,
        );

        e.mock_all_auths();
        client.transfer(&from, &to, &100);
    }

    #[test]
    #[should_panic]
    fn test_mint_requires_minter_auth() {
        let e = Env::default();
        let admin = Address::generate(&e);
        let user = Address::generate(&e);
        let contract_id = e.register(TokenContract, ());
        let client = TokenContractClient::new(&e, &contract_id);

        client.initialize(
            &admin,
            &String::from_str(&e, "ECO"),
            &String::from_str(&e, "ECO"),
            &7,
        );

        client.mint(&user, &1000);
    }

    #[test]
    fn test_initial_minter_is_admin() {
        let e = Env::default();
        let admin = Address::generate(&e);
        let contract_id = e.register(TokenContract, ());
        let client = TokenContractClient::new(&e, &contract_id);

        client.initialize(
            &admin,
            &String::from_str(&e, "ECO"),
            &String::from_str(&e, "ECO"),
            &7,
        );

        assert_eq!(client.minter(), admin);
    }

    #[test]
    fn test_set_minter_transfers_minting_rights() {
        let e = Env::default();
        let admin = Address::generate(&e);
        let minter = Address::generate(&e);
        let user = Address::generate(&e);
        let contract_id = e.register(TokenContract, ());
        let client = TokenContractClient::new(&e, &contract_id);

        client.initialize(
            &admin,
            &String::from_str(&e, "ECO"),
            &String::from_str(&e, "ECO"),
            &7,
        );

        e.mock_all_auths();
        client.set_minter(&admin, &minter);
        assert_eq!(client.minter(), minter);

        client.mint(&user, &1000);
        assert_eq!(client.balance(&user), 1000);
        assert_eq!(client.total_supply(), 1000);
    }

    #[test]
    #[should_panic(expected = "token: unauthorized")]
    fn test_set_minter_unauthorized() {
        let e = Env::default();
        let admin = Address::generate(&e);
        let attacker = Address::generate(&e);
        let minter = Address::generate(&e);
        let contract_id = e.register(TokenContract, ());
        let client = TokenContractClient::new(&e, &contract_id);

        client.initialize(
            &admin,
            &String::from_str(&e, "ECO"),
            &String::from_str(&e, "ECO"),
            &7,
        );

        e.mock_all_auths();
        client.set_minter(&attacker, &minter);
    }

    #[test]
    #[should_panic(expected = "token: already initialized")]
    fn test_double_initialize_fails() {
        let e = Env::default();
        let admin = Address::generate(&e);
        let contract_id = e.register(TokenContract, ());
        let client = TokenContractClient::new(&e, &contract_id);

        client.initialize(
            &admin,
            &String::from_str(&e, "ECO"),
            &String::from_str(&e, "ECO"),
            &7,
        );

        client.initialize(
            &admin,
            &String::from_str(&e, "ECO"),
            &String::from_str(&e, "ECO"),
            &7,
        );
    }

    #[test]
    fn test_transfer_admin() {
        let e = Env::default();
        let admin = Address::generate(&e);
        let new_admin = Address::generate(&e);
        let contract_id = e.register(TokenContract, ());
        let client = TokenContractClient::new(&e, &contract_id);

        client.initialize(
            &admin,
            &String::from_str(&e, "ECO"),
            &String::from_str(&e, "ECO"),
            &7,
        );

        e.mock_all_auths();
        client.transfer_admin(&admin, &new_admin);

        assert_eq!(client.admin(), new_admin);
    }

    #[test]
    #[should_panic(expected = "token: unauthorized")]
    fn test_transfer_admin_unauthorized() {
        let e = Env::default();
        let admin = Address::generate(&e);
        let attacker = Address::generate(&e);
        let new_admin = Address::generate(&e);
        let contract_id = e.register(TokenContract, ());
        let client = TokenContractClient::new(&e, &contract_id);

        client.initialize(
            &admin,
            &String::from_str(&e, "ECO"),
            &String::from_str(&e, "ECO"),
            &7,
        );

        e.mock_all_auths();
        client.transfer_admin(&attacker, &new_admin);
    }

    #[test]
    #[should_panic(expected = "token: new admin must be different")]
    fn test_transfer_admin_same_address() {
        let e = Env::default();
        let admin = Address::generate(&e);
        let contract_id = e.register(TokenContract, ());
        let client = TokenContractClient::new(&e, &contract_id);

        client.initialize(
            &admin,
            &String::from_str(&e, "ECO"),
            &String::from_str(&e, "ECO"),
            &7,
        );

        e.mock_all_auths();
        client.transfer_admin(&admin, &admin);
    }

    #[test]
    fn test_burn() {
        let e = Env::default();
        let admin = Address::generate(&e);
        let user = Address::generate(&e);
        let contract_id = e.register(TokenContract, ());
        let client = TokenContractClient::new(&e, &contract_id);

        client.initialize(
            &admin,
            &String::from_str(&e, "ECO"),
            &String::from_str(&e, "ECO"),
            &7,
        );

        e.mock_all_auths();
        client.mint(&user, &1000);
        client.burn(&user, &400);

        assert_eq!(client.balance(&user), 600);
        assert_eq!(client.total_supply(), 600);
    }

    #[test]
    #[should_panic(expected = "token: insufficient balance")]
    fn test_burn_more_than_balance() {
        let e = Env::default();
        let admin = Address::generate(&e);
        let user = Address::generate(&e);
        let contract_id = e.register(TokenContract, ());
        let client = TokenContractClient::new(&e, &contract_id);

        client.initialize(
            &admin,
            &String::from_str(&e, "ECO"),
            &String::from_str(&e, "ECO"),
            &7,
        );

        e.mock_all_auths();
        client.mint(&user, &100);
        client.burn(&user, &200);
    }

    #[test]
    #[should_panic(expected = "token: amount must be positive")]
    fn test_burn_zero_amount() {
        let e = Env::default();
        let admin = Address::generate(&e);
        let user = Address::generate(&e);
        let contract_id = e.register(TokenContract, ());
        let client = TokenContractClient::new(&e, &contract_id);

        client.initialize(
            &admin,
            &String::from_str(&e, "ECO"),
            &String::from_str(&e, "ECO"),
            &7,
        );

        e.mock_all_auths();
        client.burn(&user, &0);
    }

    #[test]
    fn test_approve_and_transfer_from() {
        let e = Env::default();
        let admin = Address::generate(&e);
        let owner = Address::generate(&e);
        let spender = Address::generate(&e);
        let recipient = Address::generate(&e);
        let contract_id = e.register(TokenContract, ());
        let client = TokenContractClient::new(&e, &contract_id);

        client.initialize(
            &admin,
            &String::from_str(&e, "ECO"),
            &String::from_str(&e, "ECO"),
            &7,
        );

        e.mock_all_auths();
        client.mint(&owner, &1000);
        client.approve(&owner, &spender, &500, &(e.ledger().sequence() + 100));

        assert_eq!(client.allowance(&owner, &spender), 500);

        client.transfer_from(&spender, &owner, &recipient, &300);

        assert_eq!(client.balance(&owner), 700);
        assert_eq!(client.balance(&recipient), 300);
        assert_eq!(client.allowance(&owner, &spender), 200);
    }

    #[test]
    fn test_transfer_from_emits_event() {
        use soroban_sdk::testutils::Events as _;
        use soroban_sdk::IntoVal;
        use soroban_sdk::{vec, Symbol, Val};

        let e = Env::default();
        let admin = Address::generate(&e);
        let owner = Address::generate(&e);
        let spender = Address::generate(&e);
        let recipient = Address::generate(&e);
        let contract_id = e.register(TokenContract, ());
        let client = TokenContractClient::new(&e, &contract_id);

        client.initialize(
            &admin,
            &String::from_str(&e, "ECO"),
            &String::from_str(&e, "ECO"),
            &7,
        );

        e.mock_all_auths();
        client.mint(&owner, &1000);
        client.approve(&owner, &spender, &500, &(e.ledger().sequence() + 100));

        client.transfer_from(&spender, &owner, &recipient, &300);

        let events = e.events().all();
        let transfer_from_topics: soroban_sdk::Vec<Val> = vec![
            &e,
            Symbol::new(&e, "transfer_from_event").to_val(),
            (&owner).into_val(&e),
            (&recipient).into_val(&e),
            (&spender).into_val(&e),
        ];
        let transfer_from_data: Val = soroban_sdk::Map::<Symbol, Val>::from_array(
            &e,
            [(Symbol::new(&e, "amount"), (&300i128).into_val(&e))],
        )
        .into_val(&e);

        assert_eq!(
            events,
            vec![&e, (contract_id, transfer_from_topics, transfer_from_data),]
        );
    }

    #[test]
    #[should_panic(expected = "token: insufficient allowance")]
    fn test_transfer_from_exceeds_allowance() {
        let e = Env::default();
        let admin = Address::generate(&e);
        let owner = Address::generate(&e);
        let spender = Address::generate(&e);
        let recipient = Address::generate(&e);
        let contract_id = e.register(TokenContract, ());
        let client = TokenContractClient::new(&e, &contract_id);

        client.initialize(
            &admin,
            &String::from_str(&e, "ECO"),
            &String::from_str(&e, "ECO"),
            &7,
        );

        e.mock_all_auths();
        client.mint(&owner, &1000);
        client.approve(&owner, &spender, &100, &(e.ledger().sequence() + 100));
        client.transfer_from(&spender, &owner, &recipient, &200);
    }

    #[test]
    #[should_panic(expected = "token: allowance not found")]
    fn test_transfer_from_no_allowance() {
        let e = Env::default();
        let admin = Address::generate(&e);
        let owner = Address::generate(&e);
        let spender = Address::generate(&e);
        let recipient = Address::generate(&e);
        let contract_id = e.register(TokenContract, ());
        let client = TokenContractClient::new(&e, &contract_id);

        client.initialize(
            &admin,
            &String::from_str(&e, "ECO"),
            &String::from_str(&e, "ECO"),
            &7,
        );

        e.mock_all_auths();
        client.mint(&owner, &1000);
        client.transfer_from(&spender, &owner, &recipient, &100);
    }

    #[test]
    fn test_zero_allowance_returns_zero() {
        let e = Env::default();
        let admin = Address::generate(&e);
        let owner = Address::generate(&e);
        let spender = Address::generate(&e);
        let contract_id = e.register(TokenContract, ());
        let client = TokenContractClient::new(&e, &contract_id);

        client.initialize(
            &admin,
            &String::from_str(&e, "ECO"),
            &String::from_str(&e, "ECO"),
            &7,
        );

        assert_eq!(client.allowance(&owner, &spender), 0);
    }

    #[test]
    #[should_panic(expected = "token: amount must be positive")]
    fn test_mint_negative_amount() {
        let e = Env::default();
        let admin = Address::generate(&e);
        let user = Address::generate(&e);
        let contract_id = e.register(TokenContract, ());
        let client = TokenContractClient::new(&e, &contract_id);

        client.initialize(
            &admin,
            &String::from_str(&e, "ECO"),
            &String::from_str(&e, "ECO"),
            &7,
        );

        e.mock_all_auths();
        client.mint(&user, &-100);
    }

    #[test]
    #[should_panic(expected = "token: amount must be positive")]
    fn test_transfer_negative_amount() {
        let e = Env::default();
        let admin = Address::generate(&e);
        let from = Address::generate(&e);
        let to = Address::generate(&e);
        let contract_id = e.register(TokenContract, ());
        let client = TokenContractClient::new(&e, &contract_id);

        client.initialize(
            &admin,
            &String::from_str(&e, "ECO"),
            &String::from_str(&e, "ECO"),
            &7,
        );

        e.mock_all_auths();
        client.mint(&from, &1000);
        client.transfer(&from, &to, &-50);
    }

    #[test]
    #[should_panic(expected = "token: amount must be positive")]
    fn test_burn_negative_amount() {
        let e = Env::default();
        let admin = Address::generate(&e);
        let user = Address::generate(&e);
        let contract_id = e.register(TokenContract, ());
        let client = TokenContractClient::new(&e, &contract_id);

        client.initialize(
            &admin,
            &String::from_str(&e, "ECO"),
            &String::from_str(&e, "ECO"),
            &7,
        );

        e.mock_all_auths();
        client.burn(&user, &-100);
    }

    #[test]
    fn test_transfer_to_self() {
        let e = Env::default();
        let admin = Address::generate(&e);
        let user = Address::generate(&e);
        let contract_id = e.register(TokenContract, ());
        let client = TokenContractClient::new(&e, &contract_id);

        client.initialize(
            &admin,
            &String::from_str(&e, "ECO"),
            &String::from_str(&e, "ECO"),
            &7,
        );

        e.mock_all_auths();
        client.mint(&user, &1000);
        client.transfer(&user, &user, &500);

        assert_eq!(client.balance(&user), 1000);
    }

    #[test]
    fn test_approve_overwrites_previous() {
        let e = Env::default();
        let admin = Address::generate(&e);
        let owner = Address::generate(&e);
        let spender = Address::generate(&e);
        let contract_id = e.register(TokenContract, ());
        let client = TokenContractClient::new(&e, &contract_id);

        client.initialize(
            &admin,
            &String::from_str(&e, "ECO"),
            &String::from_str(&e, "ECO"),
            &7,
        );

        e.mock_all_auths();
        client.approve(&owner, &spender, &500, &(e.ledger().sequence() + 100));
        assert_eq!(client.allowance(&owner, &spender), 500);

        client.approve(&owner, &spender, &200, &(e.ledger().sequence() + 50));
        assert_eq!(client.allowance(&owner, &spender), 200);
    }

    #[test]
    fn test_burn_entire_balance() {
        let e = Env::default();
        let admin = Address::generate(&e);
        let user = Address::generate(&e);
        let contract_id = e.register(TokenContract, ());
        let client = TokenContractClient::new(&e, &contract_id);

        client.initialize(
            &admin,
            &String::from_str(&e, "ECO"),
            &String::from_str(&e, "ECO"),
            &7,
        );

        e.mock_all_auths();
        client.mint(&user, &500);
        client.burn(&user, &500);

        assert_eq!(client.balance(&user), 0);
        assert_eq!(client.total_supply(), 0);
    }
}
