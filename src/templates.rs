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
    pub scatter_plot_html: String,
    pub box_plot_html: String,
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
    pub box_plot_html: String,
}

pub const CSS_STYLES: &str = "
.navbar-custom {
    background-color: #ffffff;
    padding: 10px 20px;
}
.navbar-brand {
    font-size: 17px;
}
.btn-primary {
    font-size: 17px;
    background-color: #007bff;
    color: #fff;
    border: none;
    border-radius: 20px;
    padding: 8px 20px;
    text-decoration: none;
}
.btn-primary:hover {
    background-color: #0056b3;
}
@media (max-width: 768px) {
    .navbar-brand {
        font-size: 12px;
   }
    .btn-primary {
        font-size: 12px;
   }
}
.statistics-row {
    display: flex;
    flex-wrap: wrap;
    justify-content: space-around;
    margin-bottom: 20px;
}
.statistic {
    margin: 0 20px;
    text-align: center;
}
.podium {
    display: flex;
    justify-content: space-around;
    margin-top: 20px;
}
.podium-item {
    border: 1px solid #ddd;
    padding: 10px;
    margin-bottom: 10px;
    border-radius: 5px;
    background-color: #fff;
    text-align: center;
}
.podium-item.gold {
    background-color: gold;
}
.podium-item.silver {
    background-color: silver;
}
.podium-item.bronze {
    background-color: #CD7F32;
}
";
