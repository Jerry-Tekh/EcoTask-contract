use crate::{access, storage};
use soroban_sdk::{contract, contractevent, contractimpl, Address, BytesN, Env, String};
pub use storage::{Task, TaskStatus};

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskCreatedEvent {
    #[topic]
    pub creator: Address,
    pub task_id: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskCompletedEvent {
    #[topic]
    pub user: Address,
    pub task_id: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskExpiredEvent {
    pub task_id: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskCancelledEvent {
    #[topic]
    pub creator: Address,
    pub task_id: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SponsorAddedEvent {
    #[topic]
    pub sponsor: Address,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SponsorRemovedEvent {
    #[topic]
    pub sponsor: Address,
}

#[contract]
pub struct RegistryContract;

#[contractimpl]
impl RegistryContract {
    pub fn initialize(e: Env, admin: Address) {
        if storage::has_admin(&e) {
            panic!("registry: already initialized");
        }
        storage::write_admin(&e, &admin);
    }

    pub fn add_sponsor(e: Env, caller: Address, sponsor: Address) {
        caller.require_auth();
        access::require_admin(&e, &caller);
        storage::add_sponsor(&e, &sponsor);
        SponsorAddedEvent { sponsor }.publish(&e);
    }

    pub fn remove_sponsor(e: Env, caller: Address, sponsor: Address) {
        caller.require_auth();
        access::require_admin(&e, &caller);
        storage::remove_sponsor(&e, &sponsor);
        SponsorRemovedEvent { sponsor }.publish(&e);
    }

    pub fn create_task(
        e: Env,
        creator: Address,
        task_type: String,
        location_hash: BytesN<32>,
        reward_amount: i128,
        max_completions: u32,
        expires_at: u64,
    ) -> u64 {
        creator.require_auth();
        access::require_sponsor(&e, &creator);

        if reward_amount <= 0 {
            panic!("registry: reward must be positive");
        }
        if max_completions == 0 {
            panic!("registry: max completions must be positive");
        }
        if expires_at <= e.ledger().timestamp() {
            panic!("registry: expiry must be in the future");
        }

        let task_id = storage::next_task_id(&e);

        let task = Task {
            id: task_id,
            creator: creator.clone(),
            task_type,
            location_hash,
            reward_amount,
            max_completions,
            completions: 0,
            status: TaskStatus::Active,
            created_at: e.ledger().timestamp(),
            expires_at,
        };

        storage::write_task(&e, &task);

        storage::push_creator_task(&e, &creator, task_id);

        TaskCreatedEvent { creator, task_id }.publish(&e);

        task_id
    }

    pub fn get_task(e: Env, task_id: u64) -> Task {
        match storage::read_task(&e, task_id) {
            Some(task) => task,
            None => panic!("registry: task not found"),
        }
    }

    pub fn complete_task(e: Env, caller: Address, task_id: u64, user: Address) {
        caller.require_auth();
        access::require_sponsor(&e, &caller);

        let mut task = match storage::read_task(&e, task_id) {
            Some(task) => task,
            None => panic!("registry: task not found"),
        };

        if task.status != TaskStatus::Active {
            panic!("registry: task is not active");
        }
        if task.expires_at < e.ledger().timestamp() {
            panic!("registry: task expired");
        }
        if storage::is_completed(&e, task_id, &user) {
            panic!("registry: already completed");
        }
        if task.completions >= task.max_completions {
            panic!("registry: max completions reached");
        }

        task.completions += 1;
        if task.completions >= task.max_completions {
            task.status = TaskStatus::Completed;
        }

        storage::write_task(&e, &task);
        storage::mark_completed(&e, task_id, &user);

        TaskCompletedEvent { user, task_id }.publish(&e);
    }

    pub fn expire_task(e: Env, caller: Address, task_id: u64) {
        caller.require_auth();
        access::require_admin(&e, &caller);

        let mut task = match storage::read_task(&e, task_id) {
            Some(task) => task,
            None => panic!("registry: task not found"),
        };

        if task.status != TaskStatus::Active {
            panic!("registry: task is not active");
        }

        task.status = TaskStatus::Expired;
        storage::write_task(&e, &task);
    }

    pub fn cancel_task(e: Env, caller: Address, task_id: u64) {
        caller.require_auth();

        let mut task = match storage::read_task(&e, task_id) {
            Some(task) => task,
            None => panic!("registry: task not found"),
        };

        if task.creator != caller {
            panic!("registry: unauthorized");
        }
        if task.status != TaskStatus::Active {
            panic!("registry: task is not active");
        }

        task.status = TaskStatus::Cancelled;
        storage::write_task(&e, &task);

        TaskCancelledEvent {
            creator: task.creator,
            task_id,
        }
        .publish(&e);
    }

    pub fn task_count(e: Env) -> u64 {
        storage::next_task_id(&e)
    }

    pub fn is_task_completed(e: Env, task_id: u64, user: Address) -> bool {
        storage::is_completed(&e, task_id, &user)
    }

    pub fn get_tasks_by_creator(e: Env, creator: Address) -> soroban_sdk::Vec<u64> {
        storage::read_creator_tasks(&e, &creator)
    }

    pub fn transfer_admin(e: Env, current_admin: Address, new_admin: Address) {
        current_admin.require_auth();
        let stored_admin = storage::read_admin(&e);
        if current_admin != stored_admin {
            panic!("registry: unauthorized");
        }
        if new_admin == current_admin {
            panic!("registry: new admin must be different");
        }
        storage::write_admin(&e, &new_admin);
    }
}

#[cfg(test)]
mod test {
    use crate::{RegistryContract, RegistryContractClient, TaskStatus};
    use soroban_sdk::testutils::{Address as _, BytesN as _, Ledger as _};
    use soroban_sdk::{Address, BytesN, Env, String};

    fn setup() -> (Env, Address, RegistryContractClient<'static>) {
        let e = Env::default();
        let admin = Address::generate(&e);
        let contract_id = e.register(RegistryContract, ());
        let client = RegistryContractClient::new(&e, &contract_id);

        client.initialize(&admin);
        (e, admin, client)
    }

    fn create_test_task(
        client: &RegistryContractClient<'static>,
        creator: &Address,
        task_type: &String,
        max_completions: u32,
        expires_in: u64,
    ) -> u64 {
        let loc_hash: BytesN<32> = BytesN::random(&client.env);
        client.create_task(
            creator,
            task_type,
            &loc_hash,
            &1000,
            &max_completions,
            &(client.env.ledger().timestamp() + expires_in),
        )
    }

    #[test]
    fn test_create_and_get_task() {
        let (e, admin, client) = setup();
        e.mock_all_auths();

        let task_type = String::from_str(&e, "tree-planting");
        let task_id = create_test_task(&client, &admin, &task_type, 1, 1000);

        let task = client.get_task(&task_id);
        assert_eq!(task.id, task_id);
        assert_eq!(task.creator, admin);
        assert_eq!(task.task_type, task_type);
        assert_eq!(task.reward_amount, 1000);
        assert_eq!(task.max_completions, 1);
        assert_eq!(task.completions, 0);
        assert_eq!(task.status, TaskStatus::Active);
    }

    #[test]
    fn test_complete_task() {
        let (e, admin, client) = setup();
        e.mock_all_auths();

        let user = Address::generate(&e);
        let task_id = create_test_task(
            &client,
            &admin,
            &String::from_str(&e, "trash-collection"),
            1,
            1000,
        );

        client.complete_task(&admin, &task_id, &user);

        let task = client.get_task(&task_id);
        assert_eq!(task.completions, 1);
        assert_eq!(task.status, TaskStatus::Completed);
        assert!(client.is_task_completed(&task_id, &user));
    }

    #[test]
    #[should_panic(expected = "registry: already completed")]
    fn test_double_claim_prevention() {
        let (e, admin, client) = setup();
        e.mock_all_auths();

        let user = Address::generate(&e);
        let task_id = create_test_task(
            &client,
            &admin,
            &String::from_str(&e, "ocean-cleanup"),
            2,
            1000,
        );

        client.complete_task(&admin, &task_id, &user);
        client.complete_task(&admin, &task_id, &user);
    }

    #[test]
    fn test_max_completions() {
        let (e, admin, client) = setup();
        e.mock_all_auths();

        let user1 = Address::generate(&e);
        let user2 = Address::generate(&e);
        let task_id = create_test_task(
            &client,
            &admin,
            &String::from_str(&e, "tree-planting"),
            2,
            1000,
        );

        client.complete_task(&admin, &task_id, &user1);
        let task = client.get_task(&task_id);
        assert_eq!(task.completions, 1);
        assert_eq!(task.status, TaskStatus::Active);

        client.complete_task(&admin, &task_id, &user2);
        let task = client.get_task(&task_id);
        assert_eq!(task.completions, 2);
        assert_eq!(task.status, TaskStatus::Completed);
    }

    #[test]
    fn test_expire_task() {
        let (e, admin, client) = setup();
        e.mock_all_auths();

        let task_id = create_test_task(
            &client,
            &admin,
            &String::from_str(&e, "tree-planting"),
            1,
            1000,
        );

        client.expire_task(&admin, &task_id);

        let task = client.get_task(&task_id);
        assert_eq!(task.status, TaskStatus::Expired);
    }

    #[test]
    #[should_panic(expected = "registry: unauthorized")]
    fn test_unauthorized_creator() {
        let (e, _admin, client) = setup();
        e.mock_all_auths();

        let attacker = Address::generate(&e);

        let loc_hash: BytesN<32> = BytesN::random(&e);
        client.create_task(
            &attacker,
            &String::from_str(&e, "tree-planting"),
            &loc_hash,
            &1000,
            &1,
            &(e.ledger().timestamp() + 1000),
        );
    }

    #[test]
    #[should_panic(expected = "registry: unauthorized")]
    fn test_unauthorized_expire() {
        let (e, _admin, client) = setup();
        e.mock_all_auths();

        let attacker = Address::generate(&e);
        client.expire_task(&attacker, &0);
    }

    #[test]
    fn test_add_sponsor() {
        let (e, admin, client) = setup();
        e.mock_all_auths();

        let sponsor = Address::generate(&e);
        client.add_sponsor(&admin, &sponsor);

        let loc_hash: BytesN<32> = BytesN::random(&e);
        let task_id = client.create_task(
            &sponsor,
            &String::from_str(&e, "tree-planting"),
            &loc_hash,
            &1000,
            &1,
            &(e.ledger().timestamp() + 1000),
        );

        let task = client.get_task(&task_id);
        assert_eq!(task.creator, sponsor);
    }

    #[test]
    fn test_remove_sponsor() {
        let (e, admin, client) = setup();
        e.mock_all_auths();

        let sponsor = Address::generate(&e);
        client.add_sponsor(&admin, &sponsor);

        let loc_hash: BytesN<32> = BytesN::random(&e);
        client.create_task(
            &sponsor,
            &String::from_str(&e, "tree-planting"),
            &loc_hash,
            &1000,
            &1,
            &(e.ledger().timestamp() + 1000),
        );

        client.remove_sponsor(&admin, &sponsor);
    }

    #[test]
    #[should_panic(expected = "registry: unauthorized")]
    fn test_removed_sponsor_cannot_create_task() {
        let (e, admin, client) = setup();
        e.mock_all_auths();

        let sponsor = Address::generate(&e);
        client.add_sponsor(&admin, &sponsor);
        client.remove_sponsor(&admin, &sponsor);

        let loc_hash: BytesN<32> = BytesN::random(&e);
        client.create_task(
            &sponsor,
            &String::from_str(&e, "tree-planting"),
            &loc_hash,
            &1000,
            &1,
            &(e.ledger().timestamp() + 1000),
        );
    }

    #[test]
    #[should_panic(expected = "registry: unauthorized")]
    fn test_remove_sponsor_non_admin() {
        let (e, admin, client) = setup();
        e.mock_all_auths();

        let sponsor = Address::generate(&e);
        client.add_sponsor(&admin, &sponsor);

        let non_admin = Address::generate(&e);
        client.remove_sponsor(&non_admin, &sponsor);
    }

    #[test]
    #[should_panic(expected = "registry: task is not active")]
    fn test_expire_already_expired_task() {
        let (e, admin, client) = setup();
        e.mock_all_auths();

        let task_id = create_test_task(
            &client,
            &admin,
            &String::from_str(&e, "tree-planting"),
            1,
            1000,
        );

        client.expire_task(&admin, &task_id);
        client.expire_task(&admin, &task_id);
    }

    #[test]
    #[should_panic(expected = "registry: task expired")]
    fn test_expired_task_cannot_be_completed() {
        let (e, admin, client) = setup();
        e.mock_all_auths();

        let user = Address::generate(&e);

        e.ledger().set_timestamp(1000);
        let task_id = create_test_task(
            &client,
            &admin,
            &String::from_str(&e, "tree-planting"),
            1,
            1000,
        );

        e.ledger().set_timestamp(3000);
        client.complete_task(&admin, &task_id, &user);
    }

    #[test]
    fn test_cancel_task_by_creator() {
        let (e, admin, client) = setup();
        e.mock_all_auths();

        let task_id = create_test_task(
            &client,
            &admin,
            &String::from_str(&e, "tree-planting"),
            1,
            1000,
        );

        client.cancel_task(&admin, &task_id);

        let task = client.get_task(&task_id);
        assert_eq!(task.status, TaskStatus::Cancelled);
    }

    #[test]
    #[should_panic(expected = "registry: unauthorized")]
    fn test_cancel_task_not_creator() {
        let (e, admin, client) = setup();
        e.mock_all_auths();

        let task_id = create_test_task(
            &client,
            &admin,
            &String::from_str(&e, "tree-planting"),
            1,
            1000,
        );

        let other = Address::generate(&e);
        client.cancel_task(&other, &task_id);
    }

    #[test]
    #[should_panic(expected = "registry: task is not active")]
    fn test_cancel_already_completed_task() {
        let (e, admin, client) = setup();
        e.mock_all_auths();

        let user = Address::generate(&e);
        let task_id = create_test_task(
            &client,
            &admin,
            &String::from_str(&e, "tree-planting"),
            1,
            1000,
        );

        client.complete_task(&admin, &task_id, &user);
        client.cancel_task(&admin, &task_id);
    }

    #[test]
    #[should_panic(expected = "registry: task is not active")]
    fn test_complete_cancelled_task() {
        let (e, admin, client) = setup();
        e.mock_all_auths();

        let user = Address::generate(&e);
        let task_id = create_test_task(
            &client,
            &admin,
            &String::from_str(&e, "tree-planting"),
            2,
            1000,
        );

        client.cancel_task(&admin, &task_id);
        client.complete_task(&admin, &task_id, &user);
    }

    #[test]
    #[should_panic(expected = "registry: task is not active")]
    fn test_expire_cancelled_task() {
        let (e, admin, client) = setup();
        e.mock_all_auths();

        let task_id = create_test_task(
            &client,
            &admin,
            &String::from_str(&e, "tree-planting"),
            1,
            1000,
        );

        client.cancel_task(&admin, &task_id);
        client.expire_task(&admin, &task_id);
    }

    #[test]
    fn test_get_tasks_by_creator() {
        let (e, admin, client) = setup();
        e.mock_all_auths();

        let task_type = String::from_str(&e, "tree-planting");
        let id0 = create_test_task(&client, &admin, &task_type, 1, 1000);
        let id1 = create_test_task(&client, &admin, &task_type, 1, 1000);
        let id2 = create_test_task(&client, &admin, &task_type, 1, 1000);

        let ids = client.get_tasks_by_creator(&admin);
        assert_eq!(ids.len(), 3);
        assert_eq!(ids.get(0).unwrap(), id0);
        assert_eq!(ids.get(1).unwrap(), id1);
        assert_eq!(ids.get(2).unwrap(), id2);

        // A different creator should have an empty list
        let other = Address::generate(&e);
        let empty = client.get_tasks_by_creator(&other);
        assert_eq!(empty.len(), 0);
    }

    #[test]
    fn test_task_survives_ledger_advancement() {
        let (e, admin, client) = setup();
        e.mock_all_auths();

        let task_type = String::from_str(&e, "tree-planting");
        let task_id = create_test_task(&client, &admin, &task_type, 2, 100_000);

        let user1 = Address::generate(&e);
        client.complete_task(&admin, &task_id, &user1);

        // Advance the ledger well past the default instance storage TTL
        // (instance TTL is ~100 ledgers; persistent TTL is ~4096).
        e.ledger().set_sequence_number(5000);

        let task = client.get_task(&task_id);
        assert_eq!(task.id, task_id);
        assert_eq!(task.creator, admin);
        assert_eq!(task.task_type, task_type);
        assert_eq!(task.completions, 1);
        assert_eq!(task.status, TaskStatus::Active);
        assert!(client.is_task_completed(&task_id, &user1));

        let ids = client.get_tasks_by_creator(&admin);
        assert_eq!(ids.len(), 1);
        assert_eq!(ids.get(0).unwrap(), task_id);
    }

    #[test]
    fn test_transfer_admin() {
        let (e, admin, client) = setup();
        e.mock_all_auths();

        let new_admin = Address::generate(&e);
        client.transfer_admin(&admin, &new_admin);

        let loc_hash: BytesN<32> = BytesN::random(&e);
        let task_id = client.create_task(
            &new_admin,
            &String::from_str(&e, "tree-planting"),
            &loc_hash,
            &1000,
            &1,
            &(e.ledger().timestamp() + 1000),
        );

        let task = client.get_task(&task_id);
        assert_eq!(task.creator, new_admin);
    }

    #[test]
    #[should_panic(expected = "registry: unauthorized")]
    fn test_transfer_admin_unauthorized() {
        let (e, _admin, client) = setup();
        e.mock_all_auths();

        let attacker = Address::generate(&e);
        let new_admin = Address::generate(&e);
        client.transfer_admin(&attacker, &new_admin);
    }

    #[test]
    #[should_panic(expected = "registry: new admin must be different")]
    fn test_transfer_admin_same_address() {
        let (e, admin, client) = setup();
        e.mock_all_auths();

        client.transfer_admin(&admin, &admin);
    }
}
