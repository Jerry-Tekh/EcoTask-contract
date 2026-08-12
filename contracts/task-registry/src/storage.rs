use soroban_sdk::{contracttype, Address, BytesN, Env, String, Vec};

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub enum TaskStatus {
    Active,
    Completed,
    Expired,
    Cancelled,
}

#[derive(Clone, Debug)]
#[contracttype]
pub struct Task {
    pub id: u64,
    pub creator: Address,
    pub task_type: String,
    pub location_hash: BytesN<32>,
    pub reward_amount: i128,
    pub max_completions: u32,
    pub completions: u32,
    pub status: TaskStatus,
    pub created_at: u64,
    pub expires_at: u64,
}

#[derive(Clone, Debug)]
#[contracttype]
pub enum DataKey {
    Task(u64),
    TaskCount,
    Admin,
    Sponsor(Address),
    Completion(u64, Address),
    CreatorTasks(Address),
}

pub fn write_task(e: &Env, task: &Task) {
    let key = DataKey::Task(task.id);
    e.storage().persistent().set(&key, task);
}

pub fn read_task(e: &Env, task_id: u64) -> Option<Task> {
    let key = DataKey::Task(task_id);
    e.storage().persistent().get(&key)
}

pub fn next_task_id(e: &Env) -> u64 {
    let key = DataKey::TaskCount;
    let count: u64 = e.storage().instance().get(&key).unwrap_or(0);
    e.storage().instance().set(&key, &(count + 1));
    count
}

/// Reads the number of tasks created so far (the next available task id)
/// without mutating any storage. This is safe to call repeatedly.
pub fn read_task_count(e: &Env) -> u64 {
    let key = DataKey::TaskCount;
    e.storage().instance().get(&key).unwrap_or(0)
}

pub fn write_admin(e: &Env, admin: &Address) {
    let key = DataKey::Admin;
    e.storage().instance().set(&key, admin);
}

pub fn read_admin(e: &Env) -> Address {
    let key = DataKey::Admin;
    e.storage().instance().get(&key).unwrap()
}

pub fn has_admin(e: &Env) -> bool {
    let key = DataKey::Admin;
    e.storage().instance().has(&key)
}

pub fn add_sponsor(e: &Env, sponsor: &Address) {
    let key = DataKey::Sponsor(sponsor.clone());
    e.storage().persistent().set(&key, &true);
}

pub fn remove_sponsor(e: &Env, sponsor: &Address) {
    let key = DataKey::Sponsor(sponsor.clone());
    e.storage().persistent().remove(&key);
}

pub fn is_sponsor(e: &Env, addr: &Address) -> bool {
    let key = DataKey::Sponsor(addr.clone());
    e.storage().persistent().has(&key)
}

pub fn mark_completed(e: &Env, task_id: u64, user: &Address) {
    let key = DataKey::Completion(task_id, user.clone());
    e.storage().persistent().set(&key, &true);
}

pub fn is_completed(e: &Env, task_id: u64, user: &Address) -> bool {
    let key = DataKey::Completion(task_id, user.clone());
    e.storage().persistent().get(&key).unwrap_or(false)
}

pub fn push_creator_task(e: &Env, creator: &Address, task_id: u64) {
    let key = DataKey::CreatorTasks(creator.clone());
    let mut ids: Vec<u64> = e.storage().persistent().get(&key).unwrap_or(Vec::new(e));
    ids.push_back(task_id);
    e.storage().persistent().set(&key, &ids);
}

pub fn read_creator_tasks(e: &Env, creator: &Address) -> Vec<u64> {
    let key = DataKey::CreatorTasks(creator.clone());
    e.storage().persistent().get(&key).unwrap_or(Vec::new(e))
}
