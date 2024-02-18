use crate::models::ResultEntry;
use linreg::linear_regression;
use plotly::common::{Mode, Title};
use plotly::layout::Axis;
use plotly::{Layout, Plot, Scatter};
use serde_json::Value;
use std::error::Error;

pub fn generate_plot_html(all_entries: Vec<&mut Vec<ResultEntry>>) -> String {
    let layout = Layout::new()
        .x_axis(Axis::new().title(Title::from("Date")))
        .y_axis(Axis::new().title(Title::from("Time (seconds)")))
        .auto_size(true);
    let mut plot = Plot::new();

    for user_entries in all_entries {
        user_entries.sort_by(|a, b| a.date.cmp(&b.date));

        let dates: Vec<String> = user_entries
            .iter()
            .map(|entry| entry.date.clone())
            .collect();
        let times: Vec<i32> = user_entries.iter().map(|entry| entry.time).collect();

        let x: Vec<f64> = (0..dates.len()).map(|i| i as f64).collect();
        let y: Vec<f64> = times.iter().map(|i| *i as f64).collect();

        let (slope, intercept): (f64, f64) = match linear_regression(&x, &y) {
            Ok((slope, intercept)) => (slope, intercept),
            _ => return String::from("Need more times before we can plot!"),
        };

        let username = match user_entries.first() {
            Some(entry) => &entry.username,
            _ => return String::from("Need more times before we can plot!"),
        };

        let trendline_values: Vec<f64> = x.iter().map(|&i| slope.mul_add(i, intercept)).collect();

        let trace_times = Scatter::new(dates.clone(), times)
            .name(format!("{username}'s Times"))
            .mode(Mode::Lines);
        let trace_trendline = Scatter::new(dates, trendline_values)
            .name(format!("{username}'s Trendline"))
            .mode(Mode::Lines);

        plot.add_trace(trace_times);
        plot.add_trace(trace_trendline);
    }

    plot.set_layout(layout);

    plot.to_inline_html(None)
}

pub fn compute_percentiles(
    data: &[ResultEntry],
    percentiles_to_compute: &Vec<i32>,
) -> Result<Vec<i32>, Box<dyn Error>> {
    let mut result: Vec<i32> = Vec::new();
    for &percentile in percentiles_to_compute {
        let index = (percentile as f64 / 100.0 * data.len() as f64) as usize;
        let percentile_value = data.get(index).ok_or("Index out of bounds")?.time;
        result.push(percentile_value);
    }
    result.reverse();
    Ok(result)
}

pub async fn fetch_live_leaderboard(token: String) -> Result<Vec<ResultEntry>, Box<dyn Error>> {
    let client = reqwest::Client::new();

    let body = client
        .get("https://www.nytimes.com/svc/crosswords/v6/leaderboard/mini.json")
        .header("accept", "application/json")
        .header("nyt-s", token)
        .send()
        .await?
        .text()
        .await?;

    let v: Value = serde_json::from_str(&body[..])?;

    let mut result_entries: Vec<ResultEntry> = Vec::new();

    for entry in v["data"].as_array().ok_or("NYT API had no results")? {
        let rank: i32 = match entry["rank"]
            .as_str()
            .and_then(|rank_str| rank_str.parse::<i32>().ok())
        {
            Some(rank) => rank,
            None => continue,
        };

        result_entries.push(ResultEntry {
            time: entry["score"]["secondsSpentSolving"]
                .as_i64()
                .ok_or("Failed to serialize time into i64")? as i32,
            username: entry["name"]
                .as_str()
                .ok_or("Failed to serialize most recent crossword date as string")?
                .to_string(),
            rank,
            ..Default::default()
        });
    }

    Ok(result_entries)
}
