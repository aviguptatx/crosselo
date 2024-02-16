use askama::Template;

use crate::models::{HeadToHeadData, LeaderboardEntry, ResultEntry, UserData};

mod filters {
    pub fn convert_time_to_mm_ss(seconds: &i32) -> ::askama::Result<String> {
        let minutes = seconds / 60;
        let seconds = seconds % 60;
        Ok(format!("{minutes:02}:{seconds:02}"))
    }

    pub fn round(f: &f64) -> ::askama::Result<i32> {
        Ok(f.round() as i32)
    }
}

#[derive(Template)]
#[template(path = "home.html")]
pub struct LeaderboardTemplate {
    pub data: Vec<LeaderboardEntry>,
}

#[derive(Template)]
#[template(path = "user.html")]
pub struct UserTemplate {
    pub username: String,
    pub plot_html: String,
    pub data: UserData,
}

#[derive(Template)]
#[template(path = "podium.html")]
pub struct PodiumTemplate {
    pub data: Vec<ResultEntry>,
}

#[derive(Template)]
#[template(path = "history.html")]
pub struct HistoryTemplate {
    pub date: String,
    pub data: Vec<ResultEntry>,
}

#[derive(Template)]
#[template(path = "recent.html")]
pub struct RecentTemplate {
    pub dates: Vec<String>,
}

#[derive(Template)]
#[template(path = "today.html")]
pub struct TodayTemplate {
    pub data: Vec<ResultEntry>,
}

#[derive(Template, Default)]
#[template(path = "h2h.html")]
pub struct HeadToHeadTemplate {
    pub populated: bool,
    pub users: Vec<String>,
    pub data: HeadToHeadData,
}
