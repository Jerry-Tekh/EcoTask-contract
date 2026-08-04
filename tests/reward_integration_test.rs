use soroban_sdk::testutils::Address as _;
use soroban_sdk::testutils::BytesN;
use soroban_sdk::{Address, Env, String};

fn deploy_token(e: &Env, admin: &Address) -> Address {
    let token_id = e.register(eco_token::TokenContract, ());
    let client = eco_token::TokenContractClient::new(e, &token_id);
    client.initialize(
        admin,
        &String::from_str(e, "ECO"),
        &String::from_str(e, "ECO"),
        &7,
    );
    token_id
}

fn deploy_registry(e: &Env, admin: &Address) -> Address {
    let reg_id = e.register(task_registry::RegistryContract, ());
    let client = task_registry::RegistryContractClient::new(e, &reg_id);
    client.initialize(admin);
    reg_id
}

fn deploy_engine(
    e: &Env,
    admin: &Address,
    token_id: &Address,
    reg_id: &Address,
    oracle: &Address,
) -> Address {
    let engine_id = e.register(reward_engine::RewardEngine, ());
    let engine_client = reward_engine::RewardEngineClient::new(e, &engine_id);
    let reg_client = task_registry::RegistryContractClient::new(e, reg_id);
    reg_client.add_sponsor(admin, &engine_id);
    engine_client.initialize(admin, token_id, reg_id, oracle);
    engine_id
}

#[test]
fn test_full_payout_lifecycle() {
    let e = Env::default();
    e.mock_all_auths_allowing_non_root_auth();

    let admin = Address::generate(&e);
    let oracle = Address::generate(&e);
    let user1 = Address::generate(&e);
    let user2 = Address::generate(&e);

    let token_id = deploy_token(&e, &admin);
    let reg_id = deploy_registry(&e, &admin);
    let engine_id = deploy_engine(&e, &admin, &token_id, &reg_id, &oracle);

    let engine_client = reward_engine::RewardEngineClient::new(&e, &engine_id);
    let reg_client = task_registry::RegistryContractClient::new(&e, &reg_id);
    let token_client = eco_token::TokenContractClient::new(&e, &token_id);

    let loc_hash1 = soroban_sdk::BytesN::<32>::random(&e);
    let task_id1 = reg_client.create_task(
        &admin,
        &String::from_str(&e, "tree-planting"),
        &loc_hash1,
        &500,
        &2,
        &(e.ledger().timestamp() + 10000),
    );

    let loc_hash2 = soroban_sdk::BytesN::<32>::random(&e);
    let task_id2 = reg_client.create_task(
        &admin,
        &String::from_str(&e, "ocean-cleanup"),
        &loc_hash2,
        &300,
        &1,
        &(e.ledger().timestamp() + 10000),
    );

    let proof1 = String::from_str(&e, "QmUser1Proof");
    engine_client.submit_proof(&oracle, &user1, &task_id1, &proof1);
    engine_client.approve_proof(&oracle, &user1, &task_id1, &500);

    let proof2 = String::from_str(&e, "QmUser2Proof");
    engine_client.submit_proof(&oracle, &user2, &task_id2, &proof2);
    engine_client.approve_proof(&oracle, &user2, &task_id2, &300);

    assert_eq!(token_client.balance(&user1), 500);
    assert_eq!(token_client.balance(&user2), 300);
    assert_eq!(token_client.total_supply(), 800);
    assert_eq!(engine_client.total_paid(), 800);

    let task1 = reg_client.get_task(&task_id1);
    assert_eq!(task1.completions, 1);
    assert_eq!(task1.status, task_registry::TaskStatus::Active);

    let task2 = reg_client.get_task(&task_id2);
    assert_eq!(task2.completions, 1);
    assert_eq!(task2.status, task_registry::TaskStatus::Completed);
}

#[test]
fn test_dispute_reject_then_resolve() {
    let e = Env::default();
    e.mock_all_auths_allowing_non_root_auth();

    let admin = Address::generate(&e);
    let oracle = Address::generate(&e);
    let user = Address::generate(&e);

    let token_id = deploy_token(&e, &admin);
    let reg_id = deploy_registry(&e, &admin);
    let engine_id = deploy_engine(&e, &admin, &token_id, &reg_id, &oracle);

    let engine_client = reward_engine::RewardEngineClient::new(&e, &engine_id);
    let reg_client = task_registry::RegistryContractClient::new(&e, &reg_id);

    let loc_hash = soroban_sdk::BytesN::<32>::random(&e);
    let task_id = reg_client.create_task(
        &admin,
        &String::from_str(&e, "river-cleanup"),
        &loc_hash,
        &750,
        &1,
        &(e.ledger().timestamp() + 10000),
    );

    let proof = String::from_str(&e, "QmDisputed");
    engine_client.submit_proof(&oracle, &user, &task_id, &proof);
    engine_client.reject_proof(&oracle, &user, &task_id);

    let v = engine_client.get_verification(&task_id, &user);
    assert_eq!(v.status, reward_engine::VerificationStatus::Rejected);

    engine_client.dispute_proof(&admin, &user, &task_id);
    let v = engine_client.get_verification(&task_id, &user);
    assert_eq!(v.status, reward_engine::VerificationStatus::Disputed);

    engine_client.resolve_dispute(&admin, &user, &task_id, &true, &750);

    let token_client = eco_token::TokenContractClient::new(&e, &token_id);
    assert_eq!(token_client.balance(&user), 750);
    assert_eq!(engine_client.total_paid(), 750);
}

#[test]
fn test_multi_user_max_completions() {
    let e = Env::default();
    e.mock_all_auths_allowing_non_root_auth();

    let admin = Address::generate(&e);
    let oracle = Address::generate(&e);
    let users: Vec<Address> = (0..5).map(|_| Address::generate(&e)).collect();

    let token_id = deploy_token(&e, &admin);
    let reg_id = deploy_registry(&e, &admin);
    let engine_id = deploy_engine(&e, &admin, &token_id, &reg_id, &oracle);

    let engine_client = reward_engine::RewardEngineClient::new(&e, &engine_id);
    let reg_client = task_registry::RegistryContractClient::new(&e, &reg_id);

    let loc_hash = soroban_sdk::BytesN::<32>::random(&e);
    let task_id = reg_client.create_task(
        &admin,
        &String::from_str(&e, "beach-cleanup"),
        &loc_hash,
        &100,
        &5,
        &(e.ledger().timestamp() + 10000),
    );

    for user in users.iter() {
        let proof = String::from_str(&e, "QmBeach");
        engine_client.submit_proof(&oracle, user, &task_id, &proof);
        engine_client.approve_proof(&oracle, user, &task_id, &100);
    }

    let token_client = eco_token::TokenContractClient::new(&e, &token_id);
    for user in users.iter() {
        assert_eq!(token_client.balance(user), 100);
    }
    assert_eq!(token_client.total_supply(), 500);
    assert_eq!(engine_client.total_paid(), 500);

    let task = reg_client.get_task(&task_id);
    assert_eq!(task.completions, 5);
    assert_eq!(task.status, task_registry::TaskStatus::Completed);
}

#[test]
fn test_reward_cap_enforced_cross_contract() {
    let e = Env::default();
    e.mock_all_auths_allowing_non_root_auth();

    let admin = Address::generate(&e);
    let oracle = Address::generate(&e);
    let user = Address::generate(&e);

    let token_id = deploy_token(&e, &admin);
    let reg_id = deploy_registry(&e, &admin);
    let engine_id = deploy_engine(&e, &admin, &token_id, &reg_id, &oracle);

    let engine_client = reward_engine::RewardEngineClient::new(&e, &engine_id);
    let reg_client = task_registry::RegistryContractClient::new(&e, &reg_id);

    let loc_hash = soroban_sdk::BytesN::<32>::random(&e);
    let task_id = reg_client.create_task(
        &admin,
        &String::from_str(&e, "recycling"),
        &loc_hash,
        &200,
        &1,
        &(e.ledger().timestamp() + 10000),
    );

    let proof = String::from_str(&e, "QmCap");
    engine_client.submit_proof(&oracle, &user, &task_id, &proof);

    let result = engine_client.try_approve_proof(&oracle, &user, &task_id, &999);
    assert!(result.is_err());
}

#[test]
fn test_minter_is_reward_engine_after_setup() {
    let e = Env::default();
    e.mock_all_auths_allowing_non_root_auth();

    let admin = Address::generate(&e);
    let oracle = Address::generate(&e);

    let token_id = deploy_token(&e, &admin);
    let reg_id = deploy_registry(&e, &admin);
    let engine_id = deploy_engine(&e, &admin, &token_id, &reg_id, &oracle);

    let token_client = eco_token::TokenContractClient::new(&e, &token_id);

    assert_eq!(token_client.minter(), admin);

    token_client.set_minter(&admin, &engine_id);
    assert_eq!(token_client.minter(), engine_id);
}

#[test]
fn test_admin_cancel_and_payout_rejection() {
    let e = Env::default();
    e.mock_all_auths_allowing_non_root_auth();

    let admin = Address::generate(&e);
    let oracle = Address::generate(&e);
    let user = Address::generate(&e);

    let token_id = deploy_token(&e, &admin);
    let reg_id = deploy_registry(&e, &admin);
    let engine_id = deploy_engine(&e, &admin, &token_id, &reg_id, &oracle);

    let engine_client = reward_engine::RewardEngineClient::new(&e, &engine_id);
    let reg_client = task_registry::RegistryContractClient::new(&e, &reg_id);

    let loc_hash = soroban_sdk::BytesN::<32>::random(&e);
    let task_id = reg_client.create_task(
        &admin,
        &String::from_str(&e, "composting"),
        &loc_hash,
        &400,
        &1,
        &(e.ledger().timestamp() + 10000),
    );

    let proof = String::from_str(&e, "QmAdmin");
    engine_client.submit_proof(&oracle, &user, &task_id, &proof);

    reg_client.admin_cancel_task(&admin, &task_id);

    let result = engine_client.try_approve_proof(&oracle, &user, &task_id, &400);
    assert!(result.is_err());
}
