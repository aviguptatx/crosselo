use serde::Deserialize;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ResultEntry {
    pub date: String,
    pub time: i32,
    pub username: String,
    pub rank: i32,
}

#[derive(Debug, Deserialize)]
pub struct LeaderboardEntry {
    pub username: String,
    pub average_time: f64,
    pub num_wins: i32,
    pub num_played: i32,
    pub elo: f64,
}

#[derive(Deserialize)]
pub struct UsernameData {
    pub username: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct HeadToHeadData {
    #[serde(skip_deserializing)]
    pub user1: String,
    #[serde(skip_deserializing)]
    pub user2: String,
    pub wins_user1: i32,
    pub wins_user2: i32,
    pub ties: i32,
    pub total_matches: i32,
    pub avg_time_difference: f64,
    #[serde(skip_deserializing)]
    pub time_diff_description: String,
}

#[derive(Debug)]
pub struct UserData {
    pub percentiles: Vec<i32>,
    pub all_times: Vec<ResultEntry>,
    pub times_excluding_saturday: Vec<ResultEntry>,
    pub top_times: Vec<ResultEntry>,
}

#[derive(Debug, Deserialize)]
pub struct Wrapper<T> {
    pub inner: T,
}
