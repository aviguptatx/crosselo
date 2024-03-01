use linreg::linear_regression;
use plotly::box_plot::BoxPoints;
use plotly::color::Rgb;
use plotly::common::{Line, Marker, Mode, Title};
use plotly::layout::{Axis, RangeSelector, RangeSlider, SelectorButton, SelectorStep, StepMode};
use plotly::{BoxPlot, Layout, Plot, Scatter};
use std::error::Error;

use crate::models::{NytApiResponse, ResultEntry};

fn get_average_time(entries: &[ResultEntry]) -> i32 {
    entries.iter().map(|entry| entry.time).sum::<i32>() / entries.len() as i32
}

pub fn generate_scatter_plot_html(all_entries: Vec<&mut [ResultEntry]>) -> String {
    let average_time = all_entries
        .iter()
        .map(|user_entries| get_average_time(user_entries))
        .max()
        .unwrap_or(0);
    let layout = Layout::new()
        .title(Title::new("Line Plot"))
        .x_axis(
            Axis::new()
                .range_slider(RangeSlider::new().visible(true))
                .range_selector(RangeSelector::new().buttons(vec![
                    SelectorButton::new()
                        .count(1)
                        .label("1M")
                        .step(SelectorStep::Month)
                        .step_mode(StepMode::Backward),
                    SelectorButton::new()
                        .count(6)
                        .label("6M")
                        .step(SelectorStep::Month)
                        .step_mode(StepMode::Backward),
                    SelectorButton::new()
                        .count(1)
                        .label("YTD")
                        .step(SelectorStep::Year)
                        .step_mode(StepMode::ToDate),
                    SelectorButton::new()
                        .count(1)
                        .label("1Y")
                        .step(SelectorStep::Year)
                        .step_mode(StepMode::Backward),
                    SelectorButton::new().label("MAX").step(SelectorStep::All),
                ]))
                .title(Title::from("Date")),
        )
        .y_axis(
            Axis::new()
                .title(Title::from("Time (seconds)"))
                .grid_color(Rgb::new(243, 243, 243))
                .range(vec![0, 2 * average_time]),
        )
        .show_legend(false)
        .auto_size(true);
    let mut plot = Plot::new();

    for user_entries in all_entries {
        if user_entries.is_empty() {
            return String::from("Need more times before we can plot!");
        }

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

        let trendline_values: Vec<f64> = x.iter().map(|&i| slope.mul_add(i, intercept)).collect();

        let trace_times = Scatter::new(dates.clone(), times)
            .mode(Mode::Lines)
            .opacity(0.7);
        let trace_trendline = Scatter::new(dates, trendline_values)
            .mode(Mode::Lines)
            .opacity(0.7);

        plot.add_trace(trace_times);
        plot.add_trace(trace_trendline);
    }

    plot.set_layout(layout);

    plot.to_inline_html(Some("scatter-plot"))
}

pub fn generate_box_plot_html(all_entries: Vec<&mut Vec<ResultEntry>>) -> String {
    let average_time = all_entries
        .iter()
        .map(|user_entries| get_average_time(user_entries))
        .max()
        .unwrap_or(0);
    let layout = Layout::new()
        .title(Title::new("Boxplot (Excluding Saturdays)"))
        .y_axis(
            Axis::new()
                .title(Title::from("Time (seconds)"))
                .show_grid(true)
                .zero_line(true)
                .dtick(10.0)
                .grid_color(Rgb::new(200, 200, 200))
                .grid_width(1)
                .zero_line_color(Rgb::new(200, 200, 200))
                .zero_line_width(2)
                .range(vec![0, 3 * average_time]),
        )
        .paper_background_color(Rgb::new(255, 255, 255))
        .plot_background_color(Rgb::new(255, 255, 255))
        .show_legend(false)
        .auto_size(true);

    let mut plot = Plot::new();

    for user_entries in all_entries {
        let username = match user_entries.first() {
            Some(entry) => &entry.username,
            _ => return String::from("Need more times before we can plot!"),
        };

        let times: Vec<i32> = user_entries.iter().map(|entry| entry.time).collect();

        let trace = BoxPlot::new(times)
            .name(username)
            .box_points(BoxPoints::All)
            .jitter(0.6)
            .whisker_width(0.2)
            .marker(Marker::new().size(6))
            .line(Line::new().width(2.0));
        plot.add_trace(trace);
    }

    plot.set_layout(layout);

    plot.to_inline_html(Some("box-plot"))
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

    let api_response: NytApiResponse = serde_json::from_str(&body)?;

    let mut result_entries: Vec<ResultEntry> = Vec::new();

    let mut prev_rank: Option<i32> = None;
    for entry in api_response.data {
        let Some(time) = entry.score.map(|score| score.seconds_spent_solving) else {
            break; // All future entries are incomplete solves, so we can stop here
        };

        let rank: i32 = match entry.rank.and_then(|rank_str| rank_str.parse::<i32>().ok()) {
            Some(rank) => rank,
            None => prev_rank.ok_or("err")?, // This is a tie; NYT API doesn't provide rank for ties
        };
        prev_rank = Some(rank);

        result_entries.push(ResultEntry {
            username: entry.name,
            time,
            rank,
            ..Default::default()
        });
    }

    Ok(result_entries)
}
