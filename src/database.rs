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

    let result_data: Vec<ResultEntry> =
        serde_json::from_str(&body).map_err(|e| format!("JSON parsing error: {e}"))?;

    Ok(result_data)
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

    let date_data: Value = serde_json::from_str(&body[..])?;

    Ok(NaiveDate::parse_from_str(
        date_data
            .as_array()
            .ok_or("Couldn't fetch most recent crossword date from database")?[0]["date"]
            .as_str()
            .ok_or("Failed to serialize most recent crossword date as string")?,
        "%Y-%m-%d",
    )?)
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

    let username_data: Vec<UsernameData> = serde_json::from_str(&body)?;

    Ok(username_data
        .into_iter()
        .map(|user| user.username)
        .collect())
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

    let mut podium_data: Vec<ResultEntry> =
        serde_json::from_str(&body).map_err(|e| format!("JSON parsing error: {e}"))?;

    podium_data.truncate(10);

    Ok(podium_data)
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

    let mut leaderboard_data: Vec<LeaderboardEntry> =
        serde_json::from_str(&body).map_err(|e| format!("JSON parsing error: {e}"))?;

    leaderboard_data.sort_by(|a, b| b.elo.partial_cmp(&a.elo).unwrap_or(Ordering::Equal));

    Ok(leaderboard_data)
}

pub async fn fetch_h2h_data(
    user1: String,
    user2: String,
    url: String,
    key: String,
) -> Result<HeadToHeadData, Box<dyn Error>> {
    let body = client(url, key)
        .rpc(
            "get_h2h_stats",
            format!("{{\"user1\": \"{user1}\", \"user2\": \"{user2}\"}}"),
        )
        .execute()
        .await?
        .text()
        .await?;

    let h2h_data: Vec<HeadToHeadData> = serde_json::from_str(&body)
        .map_err(|e| format!("JSON parsing error: {e}, body: {body}"))?;
    let stats = h2h_data.first().ok_or("H2H RPC returned empty array")?;

    let (faster_user, slower_user) = if stats.avg_time_difference < 0.0 {
        (&user1, &user2)
    } else {
        (&user2, &user1)
    };

    let time_diff_description = format!(
        "On average, {} is {:.1} seconds faster than {}.",
        faster_user,
        stats.avg_time_difference.abs(),
        slower_user
    );

    Ok(HeadToHeadData {
        user1,
        user2,
        time_diff_description,
        ..*stats
    })
}
