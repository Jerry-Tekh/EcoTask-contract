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

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MaxSupplyUpdatedEvent {
    #[topic]
    pub admin: Address,
    pub max_supply: i128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MetadataUpdatedEvent {
    #[topic]
    pub admin: Address,
    pub name: String,
    pub symbol: String,
    pub decimal: u32,
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

        let supply = storage::read_supply(&e);
        if let Some(cap) = storage::read_max_supply(&e) {
            let new_supply = supply.checked_add(amount).expect("supply overflow");
            if new_supply > cap {
                panic!("token: supply cap exceeded");
            }
        }

        let balance = storage::read_balance(&e, &to);
        storage::write_balance(
            &e,
            &to,
            balance.checked_add(amount).expect("balance overflow"),
        );

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

    /// The hard cap on total supply, or `i128::MAX` if no cap has been set.
    /// Minting is rejected whenever it would push the supply past this bound.
    pub fn max_supply(e: Env) -> i128 {
        storage::read_max_supply(&e).unwrap_or(i128::MAX)
    }

    /// Sets the hard supply cap. Admin-only. The cap must be positive and not
    /// lower than the current supply, so an already-oversubscribed token can
    /// never be retroactively frozen into an invalid state.
    pub fn set_max_supply(e: Env, caller: Address, max_supply: i128) {
        caller.require_auth();
        let admin = storage::read_admin(&e);
        if caller != admin {
            panic!("token: unauthorized");
        }
        if max_supply <= 0 {
            panic!("token: max supply must be positive");
        }
        if max_supply < storage::read_supply(&e) {
            panic!("token: max supply below current supply");
        }
        storage::write_max_supply(&e, max_supply);

        MaxSupplyUpdatedEvent { admin, max_supply }.publish(&e);
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

    /// SEP-0041 alias for `decimal`.
    pub fn decimals(e: Env) -> u32 {
        storage::read_decimal(&e)
    }

    /// Updates token metadata (SEP-0041 `set_metadata`). Admin-only.
    pub fn set_metadata(e: Env, caller: Address, name: String, symbol: String, decimal: u32) {
        caller.require_auth();
        let admin = storage::read_admin(&e);
        if caller != admin {
            panic!("token: unauthorized");
        }
        storage::write_metadata(&e, &name, &symbol, &decimal);

        MetadataUpdatedEvent {
            admin,
            name,
            symbol,
            decimal,
        }
        .publish(&e);
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

        if amount < 0 {
            panic!("token: amount must be non-negative");
        }
        if expiration_ledger <= e.ledger().sequence() {
            panic!("token: expiration must be in the future");
        }

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
                    // Lazy cleanup: drop the expired allowance key from storage to
                    // reclaim ledger rent. The return value stays 0 per SEP-0041,
                    // so callers that only care about the amount are unaffected.
                    storage::remove_allowance(&e, &owner, &spender);
                    0
                } else {
                    a.amount
                }
            }
            None => 0,
        }
    }

    /// Returns `None` when no allowance exists for the (owner, spender) pair.
    /// Otherwise returns `Some((amount, expiration_ledger))`, where `amount` is
    /// `0` for an allowance that has already expired (so the caller can
    /// distinguish "never approved" from "approval expired") and the live
    /// remaining `amount` for an allowance that is still valid.
    pub fn allowance_with_expiry(e: Env, owner: Address, spender: Address) -> Option<(i128, u32)> {
        let current_sequence = e.ledger().sequence();
        match storage::read_allowance(&e, &owner, &spender) {
            Some(a) => {
                if a.expiration_ledger < current_sequence {
                    Some((0, a.expiration_ledger))
                } else {
                    Some((a.amount, a.expiration_ledger))
                }
            }
            None => None,
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
    use soroban_sdk::testutils::Ledger as _;
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
            owner.into_val(&e),
            recipient.into_val(&e),
            spender.into_val(&e),
        ];
        let transfer_from_data: Val = soroban_sdk::Map::<Symbol, Val>::from_array(
            &e,
            [(Symbol::new(&e, "amount"), 300i128.into_val(&e))],
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
    fn test_allowance_with_expiry_live() {
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
        let expiration_ledger = e.ledger().sequence() + 100;
        client.approve(&owner, &spender, &500, &expiration_ledger);

        // Live allowance: both allowance and allowance_with_expiry reflect the
        // live amount.
        assert_eq!(client.allowance(&owner, &spender), 500);
        assert_eq!(
            client.allowance_with_expiry(&owner, &spender),
            Some((500, expiration_ledger))
        );
    }

    #[test]
    fn test_allowance_with_expiry_expired() {
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
        let expiration_ledger = e.ledger().sequence() + 50;
        client.approve(&owner, &spender, &500, &expiration_ledger);

        // Advance past the expiration ledger so the allowance is expired.
        e.ledger().set_sequence_number(expiration_ledger + 1);

        // allowance_with_expiry exposes the fact that an allowance WAS set and
        // is now expired (note: this must be read before allowance(), because
        // allowance() lazily cleans up the expired storage key).
        assert_eq!(
            client.allowance_with_expiry(&owner, &spender),
            Some((0, expiration_ledger))
        );
        // allowance() must still return 0.
        assert_eq!(client.allowance(&owner, &spender), 0);
    }

    #[test]
    fn test_allowance_with_expiry_none() {
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

        // No allowance ever set: allowance is 0 and allowance_with_expiry is
        // None, distinguishing "never approved" from "expired".
        assert_eq!(client.allowance(&owner, &spender), 0);
        assert_eq!(client.allowance_with_expiry(&owner, &spender), None);
    }

    #[test]
    fn test_allowance_lazy_cleanup_of_expired() {
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
        let expiration_ledger = e.ledger().sequence() + 50;
        client.approve(&owner, &spender, &500, &expiration_ledger);
        e.ledger().set_sequence_number(expiration_ledger + 1);

        // Before cleanup, allowance_with_expiry still sees the expired entry.
        assert_eq!(
            client.allowance_with_expiry(&owner, &spender),
            Some((0, expiration_ledger))
        );

        // Calling allowance() on the expired entry lazily removes the storage
        // key, so the expired allowance is now indistinguishable from "none".
        assert_eq!(client.allowance(&owner, &spender), 0);
        assert_eq!(client.allowance_with_expiry(&owner, &spender), None);
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

    #[test]
    #[should_panic(expected = "token: amount must be non-negative")]
    fn test_approve_negative_amount() {
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
        client.approve(&owner, &spender, &-100, &(e.ledger().sequence() + 100));
    }

    #[test]
    #[should_panic(expected = "token: expiration must be in the future")]
    fn test_approve_past_expiration() {
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

        e.ledger().set_sequence_number(100);
        e.mock_all_auths();
        client.approve(&owner, &spender, &500, &50);
    }

    #[test]
    fn test_approve_zero_revokes_allowance() {
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

        client.approve(&owner, &spender, &0, &(e.ledger().sequence() + 100));
        assert_eq!(client.allowance(&owner, &spender), 0);
    }

    #[test]
    fn test_max_supply_defaults_to_no_cap() {
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

        assert_eq!(client.max_supply(), i128::MAX);
    }

    #[test]
    fn test_set_max_supply_and_mint_at_cap() {
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
        client.set_max_supply(&admin, &1000);
        assert_eq!(client.max_supply(), 1000);

        client.mint(&user, &1000);
        assert_eq!(client.balance(&user), 1000);
        assert_eq!(client.total_supply(), 1000);
    }

    #[test]
    #[should_panic(expected = "token: supply cap exceeded")]
    fn test_mint_exceeding_supply_cap_fails() {
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
        client.set_max_supply(&admin, &1000);
        client.mint(&user, &600);
        client.mint(&user, &500);
    }

    #[test]
    #[should_panic(expected = "token: supply cap exceeded")]
    fn test_mint_over_cap_on_first_issue_fails() {
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
        client.set_max_supply(&admin, &500);
        client.mint(&user, &501);
    }

    #[test]
    #[should_panic(expected = "token: max supply must be positive")]
    fn test_set_zero_max_supply_fails() {
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
        client.set_max_supply(&admin, &0);
    }

    #[test]
    #[should_panic(expected = "token: max supply below current supply")]
    fn test_set_max_supply_below_current_supply_fails() {
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
        client.set_max_supply(&admin, &500);
    }

    #[test]
    #[should_panic(expected = "token: unauthorized")]
    fn test_set_max_supply_non_admin_fails() {
        let e = Env::default();
        let admin = Address::generate(&e);
        let attacker = Address::generate(&e);
        let contract_id = e.register(TokenContract, ());
        let client = TokenContractClient::new(&e, &contract_id);

        client.initialize(
            &admin,
            &String::from_str(&e, "ECO"),
            &String::from_str(&e, "ECO"),
            &7,
        );

        e.mock_all_auths();
        client.set_max_supply(&attacker, &1000);
    }

    #[test]
    fn test_set_metadata_updates_token_info() {
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
        client.set_metadata(
            &admin,
            &String::from_str(&e, "EcoTask Token"),
            &String::from_str(&e, "ECOT"),
            &6,
        );

        assert_eq!(client.name(), String::from_str(&e, "EcoTask Token"));
        assert_eq!(client.symbol(), String::from_str(&e, "ECOT"));
        assert_eq!(client.decimal(), 6);
        assert_eq!(client.decimals(), 6);
    }

    #[test]
    #[should_panic(expected = "token: unauthorized")]
    fn test_set_metadata_non_admin_fails() {
        let e = Env::default();
        let admin = Address::generate(&e);
        let attacker = Address::generate(&e);
        let contract_id = e.register(TokenContract, ());
        let client = TokenContractClient::new(&e, &contract_id);

        client.initialize(
            &admin,
            &String::from_str(&e, "ECO"),
            &String::from_str(&e, "ECO"),
            &7,
        );

        e.mock_all_auths();
        client.set_metadata(
            &attacker,
            &String::from_str(&e, "Hijacked"),
            &String::from_str(&e, "HACK"),
            &7,
        );
    }

    #[test]
    fn test_decimals_matches_decimal() {
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

        assert_eq!(client.decimals(), 7);
        assert_eq!(client.decimal(), client.decimals());
    }
}
