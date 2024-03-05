use plotly::box_plot::BoxPoints;
use plotly::color::Rgb;
use plotly::common::{Line, Marker, Mode, Title};
use plotly::layout::{Axis, RangeSelector, RangeSlider, SelectorButton, SelectorStep, StepMode};
use plotly::{BoxPlot, Layout, Plot, Scatter};
use std::cmp::{max, min};
use std::error::Error;
use std::fmt;

use crate::models::{NytApiResponse, NytResultEntry, ResultEntry};

#[derive(Debug, Clone)]
struct PlottingError;

impl fmt::Display for PlottingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Plotting error. Most likely not enough times to generate plot."
        )
    }
}

impl std::error::Error for PlottingError {}

fn get_moving_averages(entries: &[ResultEntry], interval: usize) -> (Vec<String>, Vec<i32>) {
    let mut dates = Vec::new();
    let mut moving_averages = Vec::new();

    for i in 0..entries.len() {
        if i >= interval - 1 {
            let sum: i32 = entries[(i - (interval - 1))..=i]
                .iter()
                .map(|entry| entry.time)
                .sum();
            let average = sum / interval as i32;
            dates.push(entries[i].date.clone());
            moving_averages.push(average);
        }
    }

    (dates, moving_averages)
}

fn get_average_time(entries: &[ResultEntry]) -> i32 {
    entries.iter().map(|entry| entry.time).sum::<i32>() / entries.len() as i32
}

pub fn generate_scatter_plot_html(
    all_entries: Vec<&mut [ResultEntry]>,
) -> Result<String, Box<dyn Error>> {
    let mut plot = Plot::new();

    let mut min_moving_average = i32::MAX;
    let mut max_moving_average = i32::MIN;

    let min_user_entries_length = all_entries
        .iter()
        .map(|user_entries| user_entries.len())
        .min()
        .ok_or("Couldn't calculate min user_entries length")?;

    let interval: usize = match min_user_entries_length {
        0 => return Err(Box::new(PlottingError)),
        1..=29 => 2,
        _ => 30,
    };

    for user_entries in all_entries {
        user_entries.sort_by(|a, b| a.date.cmp(&b.date));

        let (dates, times) = get_moving_averages(user_entries, interval);

        min_moving_average = min(
            min_moving_average,
            *times
                .iter()
                .min()
                .ok_or("Couldn't find min moving average")?,
        );

        max_moving_average = max(
            max_moving_average,
            *times
                .iter()
                .max()
                .ok_or("Couldn't find max moving average")?,
        );

        let trace_times = Scatter::new(dates.clone(), times)
            .mode(Mode::Lines)
            .opacity(0.7);
        plot.add_trace(trace_times);
    }

    plot.set_layout(
        Layout::new()
            .title(Title::new(
                format!("{interval}-crossword Moving Average").as_str(),
            ))
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
                    .range(vec![
                        (min_moving_average as f64 * 0.9) as i32,
                        (max_moving_average as f64 * 1.1) as i32,
                    ]),
            )
            .show_legend(false)
            .auto_size(true),
    );

    Ok(plot.to_inline_html(Some("scatter-plot")))
}

pub fn generate_box_plot_html(
    all_entries: Vec<&mut Vec<ResultEntry>>,
) -> Result<String, Box<dyn Error>> {
    let average_time = all_entries
        .iter()
        .map(|user_entries| get_average_time(user_entries))
        .max()
        .ok_or("Couldn't calculate average")?;

    let mut plot = Plot::new();

    for user_entries in all_entries {
        let username = &user_entries
            .first()
            .ok_or("User doesn't have enough entries to generate box plot")?
            .username;

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

    plot.set_layout(
        Layout::new()
            .title(Title::new("Boxplot"))
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
            .auto_size(true),
    );

    Ok(plot.to_inline_html(Some("box-plot")))
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

pub async fn fetch_live_leaderboard(token: String) -> Result<Vec<NytResultEntry>, Box<dyn Error>> {
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

    Ok(api_response.data)
}
