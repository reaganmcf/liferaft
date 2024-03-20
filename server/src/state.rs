pub struct State {
    // persistent state on all servers
    current_term: u128,
    voted_for: Option<u128>,

    // volatile state on all servers
    commit_index: u128,
    last_applied: u128,

    // volatile state on leaders
    // (Reinitialized after election)
    next_index: Vec<u128>,
    match_index: Vec<u128>,
}
