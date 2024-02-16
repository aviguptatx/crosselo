use chrono::{Datelike, NaiveDate, Weekday};
use postgrest::Postgrest;
use serde_json::Value;
use std::cmp::Ordering;
use std::error::Error;

use crate::models::{HeadToHeadData, LeaderboardEntry, ResultEntry, UserData, UsernameData};
use crate::util::compute_percentiles;

fn client(url: String, key: String) -> Postgrest {
    Postgrest::new(url).insert_header("apikey", key)
}

pub async fn fetch_results(
    date: &str,
    url: String,
    key: String,
) -> Result<Vec<ResultEntry>, Box<dyn Error>> {
    let body = client(url, key)
        .from("results_rust")
        .select("*")
        .eq("date", date)
        .order("time")
        .execute()
        .await?
        .text()
        .await?;

    let result_entries: Vec<ResultEntry> =
        serde_json::from_str(&body).map_err(|e| format!("JSON parsing error: {e}"))?;

    Ok(result_entries)
}

pub async fn fetch_most_recent_crossword_date(
    url: String,
    key: String,
) -> Result<NaiveDate, Box<dyn Error>> {
    let body = client(url, key)
        .from("results_rust")
        .select("date")
        .order("date.desc")
        .limit(1)
        .execute()
        .await?
        .text()
        .await?;

    let v: Value = serde_json::from_str(&body[..])?;

    let date = NaiveDate::parse_from_str(
        v.as_array()
            .ok_or("Couldn't fetch most recent crossword date from database")?[0]["date"]
            .as_str()
            .ok_or("Failed to serialize most recent crossword date as string")?,
        "%Y-%m-%d",
    )?;

    Ok(date)
}

pub async fn fetch_usernames_sorted_by_elo(
    url: String,
    key: String,
) -> Result<Vec<String>, Box<dyn Error>> {
    let body = client(url, key)
        .from("all")
        .select("username")
        .order("elo.desc")
        .execute()
        .await?
        .text()
        .await?;

    let users: Vec<UsernameData> = serde_json::from_str(&body)?;
    let usernames: Vec<String> = users.into_iter().map(|user| user.username).collect();

    Ok(usernames)
}

pub async fn fetch_podium_data(
    url: String,
    key: String,
) -> Result<Vec<ResultEntry>, Box<dyn Error>> {
    let body = client(url, key)
        .from("results_rust")
        .select("*")
        .order("time")
        .execute()
        .await?
        .text()
        .await?;

    let mut result_entries: Vec<ResultEntry> =
        serde_json::from_str(&body).map_err(|e| format!("JSON parsing error: {e}"))?;

    result_entries.truncate(10);

    Ok(result_entries)
}

pub async fn fetch_user_data(
    username: &str,
    url: String,
    key: String,
) -> Result<UserData, Box<dyn Error>> {
    let percentiles = vec![10, 25, 50, 75, 90];
    let body = client(url, key)
        .from("results_rust")
        .select("*")
        .eq("username", username)
        .order("time")
        .execute()
        .await?
        .text()
        .await?;

    let all_times: Vec<ResultEntry> =
        serde_json::from_str(&body).map_err(|e| format!("JSON parsing error: {e}"))?;

    let times_excluding_saturday: Vec<ResultEntry> = all_times
        .iter()
        .filter(|entry| {
            NaiveDate::parse_from_str(entry.date.as_str(), "%Y-%m-%d")
                .map(|date| date.weekday() != Weekday::Sat)
                .unwrap_or(false)
        })
        .cloned()
        .collect();

    let percentiles = compute_percentiles(&times_excluding_saturday, &percentiles)?;
    let top_times = all_times.iter().take(3).cloned().collect();
    Ok(UserData {
        percentiles,
        all_times,
        top_times,
    })
}

pub async fn fetch_leaderboard_from_db(
    db_name: &str,
    url: String,
    key: String,
) -> Result<Vec<LeaderboardEntry>, Box<dyn Error>> {
    let body = client(url, key)
        .from(db_name)
        .select("*")
        .execute()
        .await?
        .text()
        .await?;

    let mut leaderboard_entries: Vec<LeaderboardEntry> =
        serde_json::from_str(&body).map_err(|e| format!("JSON parsing error: {e}"))?;

    leaderboard_entries.sort_by(|a, b| b.elo.partial_cmp(&a.elo).unwrap_or(Ordering::Equal));

    Ok(leaderboard_entries)
}

pub async fn fetch_h2h_data(
    user1: String,
    user2: String,
    url: String,
    key: String,
) -> Result<HeadToHeadData, Box<dyn Error>> {
    let body = client(url, key)
        .rpc(
            "get_head_to_head_stats",
            format!("{{\"user1\": \"{user1}\", \"user2\": \"{user2}\"}}"),
        )
        .execute()
        .await?
        .text()
        .await?;

    let v: Value = serde_json::from_str(&body[..])?;

    let mut wins_user1: i32 = 0;
    let mut wins_user2: i32 = 0;
    let mut ties: i32 = 0;
    let mut total_matches: i32 = 0;
    let mut total_time_diff: i32 = 0;

    for entry in v.as_array().ok_or("Database had no results")? {
        let time_player_1: i32 = entry["time_player1"]
            .as_i64()
            .ok_or("Failed to serialize time_player1 into i64")?
            as i32;
        let time_player_2: i32 = entry["time_player2"]
            .as_i64()
            .ok_or("Failed to serialize time_player2 into i64")?
            as i32;
        match time_player_1.cmp(&time_player_2) {
            std::cmp::Ordering::Less => {
                wins_user1 += 1;
            }
            std::cmp::Ordering::Greater => {
                wins_user2 += 1;
            }
            std::cmp::Ordering::Equal => {
                ties += 1;
            }
        }
        total_matches += 1;
        total_time_diff += time_player_1 - time_player_2;
    }

    let average_time_diff: f64 = total_time_diff as f64 / total_matches as f64;

    let (faster_user, slower_user) = if average_time_diff < 0.0 {
        (&user1, &user2)
    } else {
        (&user2, &user1)
    };

    let time_diff_description: String = format!(
        "On average, {} is {:.1} seconds faster than {}.",
        faster_user,
        average_time_diff.abs(),
        slower_user
    );

    Ok(HeadToHeadData {
        user1,
        user2,
        wins_user1,
        wins_user2,
        ties,
        total_matches,
        time_diff_description,
    })
}
