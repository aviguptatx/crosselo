use chrono::{Datelike, NaiveDate, Weekday};
use postgrest::Postgrest;
use serde_json::Value;
use std::cmp::Ordering;
use std::error::Error;

use crate::models::{
    HeadToHeadData, LeaderboardEntry, ResultEntry, UserData, UsernameData, Wrapper,
};

/// Fetches the results for a given date from the database.
///
/// # Arguments
///
/// * `date` - A string representing the date in "YYYY-MM-DD" format.
/// * `client` - A reference to the Postgrest client.
///
/// # Returns
///
/// A `Result` containing a vector of `ResultEntry` structs, or an error if the database query fails.
pub async fn fetch_results(
    date: &str,
    client: &Postgrest,
) -> Result<Vec<ResultEntry>, Box<dyn Error>> {
    let body = client
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

/// Fetches the most recent crossword date from the database.
///
/// # Arguments
///
/// * `client` - A reference to the Postgrest client.
///
/// # Returns
///
/// A `Result` containing the most recent crossword date as a `NaiveDate`, or an error if the database query fails.
pub async fn fetch_most_recent_crossword_date(
    client: &Postgrest,
) -> Result<NaiveDate, Box<dyn Error>> {
    let body = client
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

/// Fetches the usernames sorted by ELO rating from the database.
///
/// # Arguments
///
/// * `client` - A reference to the Postgrest client.
///
/// # Returns
///
/// A `Result` containing a vector of usernames as strings, or an error if the database query fails.
pub async fn fetch_usernames_sorted_by_elo(
    client: &Postgrest,
) -> Result<Vec<String>, Box<dyn Error>> {
    let body = client
        .from("all_rust")
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

/// Fetches the top 10 results from the database, sorted by time.
///
/// # Arguments
///
/// * `client` - A reference to the Postgrest client.
///
/// # Returns
///
/// A `Result` containing a vector of `ResultEntry` structs, or an error if the database query fails.
pub async fn fetch_podium_data(client: &Postgrest) -> Result<Vec<ResultEntry>, Box<dyn Error>> {
    let body = client
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

/// Fetches the user data for a given username from the database.
///
/// # Arguments
///
/// * `username` - A reference to the username as a string.
/// * `client` - A reference to the Postgrest client.
///
/// # Returns
///
/// A `Result` containing a `UserData` struct, or an error if the database query fails.
pub async fn fetch_user_data(
    username: &str,
    client: &Postgrest,
) -> Result<UserData, Box<dyn Error>> {
    let body = client
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

    Ok(UserData {
        all_times,
        times_excluding_saturday,
    })
}

/// Fetches the leaderboard data from the database.
///
/// # Arguments
///
/// * `db_name` - A string representing the name of the database table to query.
/// * `client` - A reference to the Postgrest client.
///
/// # Returns
///
/// A `Result` containing a vector of `LeaderboardEntry` structs, or an error if the database query fails.
pub async fn fetch_leaderboard_from_db(
    db_name: &str,
    client: &Postgrest,
) -> Result<Vec<LeaderboardEntry>, Box<dyn Error>> {
    let body = client
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

/// Fetches the head-to-head data for two users from the database.
///
/// # Arguments
///
/// * `user1` - A string representing the username of the first user.
/// * `user2` - A string representing the username of the second user.
/// * `client` - A reference to the Postgrest client.
///
/// # Returns
///
/// A `Result` containing a `HeadToHeadData` struct, or an error if the database query fails.
pub async fn fetch_h2h_data(
    user1: String,
    user2: String,
    client: &Postgrest,
) -> Result<HeadToHeadData, Box<dyn Error>> {
    let body = client
        .rpc(
            "get_h2h_stats",
            serde_json::to_string(&serde_json::json!({
                "user1": user1,
                "user2": user2,
            }))?,
        )
        .execute()
        .await?
        .text()
        .await?;

    let h2h_data: HeadToHeadData = serde_json::from_str(&body)
        .map_err(|e| format!("JSON parsing error: {e}, body: {body}"))
        .map(|wrapper: Wrapper<HeadToHeadData>| wrapper.inner)?;

    let speed_verb = if h2h_data.avg_time_difference < 0.0 {
        "faster"
    } else {
        "slower"
    };

    let time_diff_description = format!(
        "On average, {} is {:.1} seconds {} than {}.",
        user1,
        h2h_data.avg_time_difference.abs(),
        speed_verb,
        user2,
    );

    Ok(HeadToHeadData {
        user1,
        user2,
        time_diff_description,
        ..h2h_data
    })
}
