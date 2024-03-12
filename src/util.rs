use plotly::box_plot::BoxPoints;
use plotly::color::Rgb;
use plotly::common::{Line, Marker, Mode, Title};
use plotly::layout::{Axis, RangeSelector, RangeSlider, SelectorButton, SelectorStep, StepMode};
use plotly::{BoxPlot, Layout, Plot, Scatter};
use std::cmp::{max, min};
use std::error::Error;
use std::fmt;

use crate::models::{NytApiResponse, NytResultEntry, ResultEntry};

/// Custom error for plotting operations.
#[derive(Debug, Clone)]
struct PlottingError {
    message: String,
}

impl PlottingError {
    fn new(message: &str) -> Self {
        Self {
            message: message.to_string(),
        }
    }
}

impl fmt::Display for PlottingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Plotting error: {}", self.message)
    }
}

impl std::error::Error for PlottingError {}

/// Computes the moving average for a given slice of `ResultEntry` values.
///
/// # Arguments
///
/// * `entries` - A slice of `ResultEntry` values.
/// * `interval` - The window size for the moving average calculation.
/// * `include_partial` - A boolean indicating whether to include partial averages for the first `interval - 1` entries.
///
/// # Returns
///
/// A tuple containing a vector of dates and a vector of corresponding moving averages.
fn compute_moving_averages(
    entries: &[ResultEntry],
    interval: usize,
    include_partial: bool,
) -> (Vec<String>, Vec<i32>) {
    let mut dates = Vec::new();
    let mut moving_averages = Vec::new();

    for i in 0..entries.len() {
        if include_partial || i >= interval - 1 {
            let start = i.saturating_sub(interval - 1);
            let end = i;
            let sum: i32 = entries[start..=end].iter().map(|entry| entry.time).sum();
            let average = sum / (end - start + 1) as i32;
            dates.push(entries[i].date.clone());
            moving_averages.push(average);
        }
    }

    (dates, moving_averages)
}

/// Computes the average time for a given slice of `ResultEntry` values.
///
/// # Arguments
///
/// * `entries` - A slice of `ResultEntry` values.
///
/// # Returns
///
/// The average time as an `i32` value.
fn compute_average_time(entries: &[ResultEntry]) -> i32 {
    entries.iter().map(|entry| entry.time).sum::<i32>() / entries.len() as i32
}

/// Generates an HTML scatter plot for the given `ResultEntry` data.
///
/// # Arguments
///
/// * `user_entries` - A vector of mutable slices of `ResultEntry` values, representing data for different users.
///
/// # Returns
///
/// A `Result` containing the HTML string for the scatter plot, or a `PlottingError` if an error occurs.
pub fn generate_scatter_plot_html(
    all_user_entries: Vec<&mut [ResultEntry]>,
) -> Result<String, Box<dyn Error>> {
    let mut plot = Plot::new();
    let mut min_moving_average = i32::MAX;
    let mut max_moving_average = i32::MIN;

    let min_user_entries_length = all_user_entries
        .iter()
        .map(|user_entries| user_entries.len())
        .min()
        .ok_or_else(|| {
            PlottingError::new("User doesn't have enough entries to generate scatter plot")
        })?;

    let include_partial = match min_user_entries_length {
        0 => {
            return Err(Box::new(PlottingError::new(
                "User doesn't have enough entries to generate scatter plot",
            )))
        }
        1..=60 => true,
        _ => false,
    };

    for user_entries in all_user_entries {
        user_entries.sort_by(|a, b| a.date.cmp(&b.date));

        let (dates, times) = compute_moving_averages(user_entries, 30, include_partial);

        min_moving_average = min(
            min_moving_average,
            *times
                .iter()
                .min()
                .ok_or_else(|| PlottingError::new("Couldn't find min moving average"))?,
        );

        max_moving_average = max(
            max_moving_average,
            *times
                .iter()
                .max()
                .ok_or_else(|| PlottingError::new("Couldn't find max moving average"))?,
        );

        let trace_times = Scatter::new(dates.clone(), times)
            .mode(Mode::Lines)
            .opacity(0.7);
        plot.add_trace(trace_times);
    }

    plot.set_layout(
        Layout::new()
            .title(Title::new("30-Crossword Moving Average"))
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

/// Generates an HTML box plot for the given `ResultEntry` data.
///
/// # Arguments
///
/// * `user_entries` - A vector of mutable vectors of `ResultEntry` values, representing data for different users.
///
/// # Returns
///
/// A `Result` containing the HTML string for the box plot, or a `PlottingError` if an error occurs.
pub fn generate_box_plot_html(
    all_user_entries: Vec<&mut Vec<ResultEntry>>,
) -> Result<String, Box<dyn Error>> {
    let max_average_time = all_user_entries
        .iter()
        .filter(|user_entries| !user_entries.is_empty())
        .map(|user_entries| compute_average_time(user_entries))
        .max()
        .ok_or_else(|| {
            PlottingError::new("User doesn't have enough entries to generate box plot")
        })?;

    let mut plot = Plot::new();

    for user_entries in all_user_entries {
        let username = user_entries
            .first()
            .ok_or_else(|| {
                PlottingError::new("User doesn't have enough entries to generate box plot")
            })?
            .username
            .clone();

        let times: Vec<i32> = user_entries.iter().map(|entry| entry.time).collect();

        let trace = BoxPlot::new(times)
            .name(&username)
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
                    .range(vec![0, 3 * max_average_time]),
            )
            .paper_background_color(Rgb::new(255, 255, 255))
            .plot_background_color(Rgb::new(255, 255, 255))
            .show_legend(false)
            .auto_size(true),
    );

    Ok(plot.to_inline_html(Some("box-plot")))
}

/// Fetches the live leaderboard data from the New York Times API.
///
/// # Arguments
///
/// * `token` - The authentication token for the New York Times API.
///
/// # Returns
///
/// A `Result` containing a vector of `NytResultEntry` values, or an error if the API request fails.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_moving_averages_with_partial_averages() {
        let entries = vec![
            ResultEntry {
                date: "2023-10-25".to_string(),
                time: 100,
                ..Default::default()
            },
            ResultEntry {
                date: "2023-10-26".to_string(),
                time: 120,
                ..Default::default()
            },
            ResultEntry {
                date: "2023-10-27".to_string(),
                time: 110,
                ..Default::default()
            },
        ];

        let (dates, averages) = compute_moving_averages(&entries, 2, true);

        assert_eq!(dates, vec!["2023-10-25", "2023-10-26", "2023-10-27"]);
        assert_eq!(averages, vec![100, 110, 115]);
    }

    #[test]
    fn test_compute_moving_averages_without_partial_averages() {
        let entries = vec![
            ResultEntry {
                date: "2023-10-25".to_string(),
                time: 100,
                ..Default::default()
            },
            ResultEntry {
                date: "2023-10-26".to_string(),
                time: 120,
                ..Default::default()
            },
            ResultEntry {
                date: "2023-10-27".to_string(),
                time: 110,
                ..Default::default()
            },
        ];

        let (dates, averages) = compute_moving_averages(&entries, 2, false);

        assert_eq!(dates, vec!["2023-10-26", "2023-10-27"]);
        assert_eq!(averages, vec![110, 115]);
    }

    #[test]
    fn test_compute_moving_averages_with_empty_input() {
        let entries: Vec<ResultEntry> = Vec::new();

        let (dates, averages) = compute_moving_averages(&entries, 2, true);

        assert!(dates.is_empty());
        assert!(averages.is_empty());
    }

    #[test]
    fn test_compute_average_time() {
        let entries = vec![
            ResultEntry {
                date: "2023-10-25".to_string(),
                time: 100,
                ..Default::default()
            },
            ResultEntry {
                date: "2023-10-26".to_string(),
                time: 120,
                ..Default::default()
            },
            ResultEntry {
                date: "2023-10-27".to_string(),
                time: 110,
                ..Default::default()
            },
        ];

        let average_time = compute_average_time(&entries);

        assert_eq!(average_time, 110);
    }

    #[test]
    fn test_generate_scatter_plot_html_with_no_user_entries() {
        let all_user_entries: Vec<&mut [ResultEntry]> = vec![];

        let result = generate_scatter_plot_html(all_user_entries);

        assert!(result.is_err());
        assert_eq!(
            result.err().unwrap().to_string(),
            "Plotting error: User doesn't have enough entries to generate scatter plot"
        );
    }

    #[test]
    fn test_generate_scatter_plot_html_with_empty_user_entries() {
        let mut entries1: Vec<ResultEntry> = vec![];
        let mut entries2: Vec<ResultEntry> = vec![];
        let all_user_entries: Vec<&mut [ResultEntry]> = vec![&mut entries1, &mut entries2];

        let result = generate_scatter_plot_html(all_user_entries);

        assert!(result.is_err());
        assert_eq!(
            result.err().unwrap().to_string(),
            "Plotting error: User doesn't have enough entries to generate scatter plot"
        );
    }

    #[test]
    fn test_generate_box_plot_html_with_no_user_entries() {
        let all_user_entries: Vec<&mut Vec<ResultEntry>> = vec![];

        let result = generate_box_plot_html(all_user_entries);

        assert!(result.is_err());
        assert_eq!(
            result.err().unwrap().to_string(),
            "Plotting error: User doesn't have enough entries to generate box plot"
        );
    }

    #[test]
    fn test_generate_box_plot_html_with_empty_user_entries() {
        let mut entries1: Vec<ResultEntry> = vec![];
        let mut entries2: Vec<ResultEntry> = vec![];
        let all_user_entries: Vec<&mut Vec<ResultEntry>> = vec![&mut entries1, &mut entries2];

        let result = generate_box_plot_html(all_user_entries);

        assert!(result.is_err());
        assert_eq!(
            result.err().unwrap().to_string(),
            "Plotting error: User doesn't have enough entries to generate box plot"
        );
    }
}
