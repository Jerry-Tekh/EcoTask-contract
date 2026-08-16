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

#[test]
fn test_full_submit_approve_mint_lifecycle() {
    let e = Env::default();
    e.mock_all_auths_allowing_non_root_auth();

    let admin = Address::generate(&e);
    let oracle = Address::generate(&e);
    let user1 = Address::generate(&e);
    let user2 = Address::generate(&e);

    let token_id = deploy_token(&e, &admin);
    let reg_id = deploy_registry(&e, &admin);

    let engine_id = e.register(reward_engine::RewardEngine, ());
    let engine_client = reward_engine::RewardEngineClient::new(&e, &engine_id);

    let reg_client = task_registry::RegistryContractClient::new(&e, &reg_id);
    reg_client.add_sponsor(&admin, &engine_id);

    engine_client.initialize(&admin, &token_id, &reg_id, &oracle);

    let loc_hash1 = soroban_sdk::BytesN::<32>::random(&e);
    let task_id1 = reg_client.create_task(
        &admin,
        &String::from_str(&e, "tree-planting"),
        &loc_hash1,
        &1000,
        &2,
        &(e.ledger().timestamp() + 10000),
    );

    let loc_hash2 = soroban_sdk::BytesN::<32>::random(&e);
    let task_id2 = reg_client.create_task(
        &admin,
        &String::from_str(&e, "ocean-cleanup"),
        &loc_hash2,
        &500,
        &1,
        &(e.ledger().timestamp() + 10000),
    );

    let proof1 = String::from_str(&e, "QmProofUser1");
    engine_client.submit_proof(&oracle, &user1, &task_id1, &proof1);
    engine_client.approve_proof(&oracle, &user1, &task_id1, &1000);

    let token_client = eco_token::TokenContractClient::new(&e, &token_id);
    assert_eq!(token_client.balance(&user1), 1000);
    assert_eq!(token_client.total_supply(), 1000);

    let proof2 = String::from_str(&e, "QmProofUser2");
    engine_client.submit_proof(&oracle, &user2, &task_id2, &proof2);
    engine_client.approve_proof(&oracle, &user2, &task_id2, &500);

    assert_eq!(token_client.balance(&user2), 500);
    assert_eq!(token_client.total_supply(), 1500);

    let task1 = reg_client.get_task(&task_id1);
    assert_eq!(task1.completions, 1);

    let task2 = reg_client.get_task(&task_id2);
    assert_eq!(task2.completions, 1);
}

#[test]
fn test_reject_then_dispute_resolve_flow() {
    let e = Env::default();
    e.mock_all_auths_allowing_non_root_auth();

    let admin = Address::generate(&e);
    let oracle = Address::generate(&e);
    let user = Address::generate(&e);

    let token_id = deploy_token(&e, &admin);
    let reg_id = deploy_registry(&e, &admin);

    let engine_id = e.register(reward_engine::RewardEngine, ());
    let engine_client = reward_engine::RewardEngineClient::new(&e, &engine_id);

    let reg_client = task_registry::RegistryContractClient::new(&e, &reg_id);
    reg_client.add_sponsor(&admin, &engine_id);

    engine_client.initialize(&admin, &token_id, &reg_id, &oracle);

    let loc_hash = soroban_sdk::BytesN::<32>::random(&e);
    let task_id = reg_client.create_task(
        &admin,
        &String::from_str(&e, "river-cleanup"),
        &loc_hash,
        &750,
        &1,
        &(e.ledger().timestamp() + 10000),
    );

    let proof = String::from_str(&e, "QmRejected");
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
}

#[test]
fn test_multi_user_task_completions() {
    let e = Env::default();
    e.mock_all_auths_allowing_non_root_auth();

    let admin = Address::generate(&e);
    let oracle = Address::generate(&e);
    let user1 = Address::generate(&e);
    let user2 = Address::generate(&e);
    let user3 = Address::generate(&e);

    let token_id = deploy_token(&e, &admin);
    let reg_id = deploy_registry(&e, &admin);

    let engine_id = e.register(reward_engine::RewardEngine, ());
    let engine_client = reward_engine::RewardEngineClient::new(&e, &engine_id);

    let reg_client = task_registry::RegistryContractClient::new(&e, &reg_id);
    reg_client.add_sponsor(&admin, &engine_id);

    engine_client.initialize(&admin, &token_id, &reg_id, &oracle);

    let loc_hash = soroban_sdk::BytesN::<32>::random(&e);
    let task_id = reg_client.create_task(
        &admin,
        &String::from_str(&e, "beach-cleanup"),
        &loc_hash,
        &200,
        &3,
        &(e.ledger().timestamp() + 10000),
    );

    for (user, cid) in [
        (&user1, "QmMulti1"),
        (&user2, "QmMulti2"),
        (&user3, "QmMulti3"),
    ] {
        let proof = String::from_str(&e, cid);
        engine_client.submit_proof(&oracle, user, &task_id, &proof);
        engine_client.approve_proof(&oracle, user, &task_id, &200);
    }

    let token_client = eco_token::TokenContractClient::new(&e, &token_id);
    assert_eq!(token_client.balance(&user1), 200);
    assert_eq!(token_client.balance(&user2), 200);
    assert_eq!(token_client.balance(&user3), 200);
    assert_eq!(token_client.total_supply(), 600);

    let task = reg_client.get_task(&task_id);
    assert_eq!(task.completions, 3);
    assert_eq!(task.status, task_registry::TaskStatus::Completed);
}

#[test]
fn test_reward_range_enforced_in_lifecycle() {
    let e = Env::default();
    e.mock_all_auths_allowing_non_root_auth();

    let admin = Address::generate(&e);
    let oracle = Address::generate(&e);
    let user = Address::generate(&e);

    let token_id = deploy_token(&e, &admin);
    let reg_id = deploy_registry(&e, &admin);

    let engine_id = e.register(reward_engine::RewardEngine, ());
    let engine_client = reward_engine::RewardEngineClient::new(&e, &engine_id);

    let reg_client = task_registry::RegistryContractClient::new(&e, &reg_id);
    reg_client.add_sponsor(&admin, &engine_id);

    engine_client.initialize(&admin, &token_id, &reg_id, &oracle);
    engine_client.set_reward_range(&admin, &100, &500);

    let loc_hash = soroban_sdk::BytesN::<32>::random(&e);
    let task_id = reg_client.create_task(
        &admin,
        &String::from_str(&e, "recycling"),
        &loc_hash,
        &300,
        &1,
        &(e.ledger().timestamp() + 10000),
    );

    let proof = String::from_str(&e, "QmRange");
    engine_client.submit_proof(&oracle, &user, &task_id, &proof);
    engine_client.approve_proof(&oracle, &user, &task_id, &300);

    let token_client = eco_token::TokenContractClient::new(&e, &token_id);
    assert_eq!(token_client.balance(&user), 300);
}
