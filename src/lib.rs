use askama::Template;

use chrono::Duration;

mod database;
mod models;
mod templates;
mod util;

use database::{
    fetch_h2h_data, fetch_leaderboard_from_db, fetch_most_recent_crossword_date, fetch_podium_data,
    fetch_results, fetch_user_data, fetch_usernames_sorted_by_elo,
};
use models::HeadToHeadData;
use templates::{
    HeadToHeadTemplate, HistoryTemplate, LeaderboardTemplate, PodiumTemplate, RecentTemplate,
    TodayTemplate, UserTemplate,
};
use util::{fetch_live_leaderboard, generate_plot_html};

use worker::*;

#[event(fetch)]
async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    let router = Router::new();
    router
        .get_async("/", |_req, ctx| async move {
            let Ok(leaderboard_entries) = fetch_leaderboard_from_db(
                "all",
                ctx.secret("SUPABASE_API_URL")?.to_string(),
                ctx.secret("SUPABASE_API_KEY")?.to_string(),
            )
            .await
            else {
                return Response::error(String::from("Could not fetch leaderboard!"), 500);
            };
            Response::from_html(
                LeaderboardTemplate {
                    data: leaderboard_entries,
                }
                .render()
                .unwrap(),
            )
        })
        .get_async("/index/:data_source", |_req, ctx| async move {
            let Some(data_source) = ctx.param("data_source") else {
                return Response::error(String::from("Could not fetch leaderboard!"), 500);
            };
            let Ok(leaderboard_entries) = fetch_leaderboard_from_db(
                data_source,
                ctx.secret("SUPABASE_API_URL")?.to_string(),
                ctx.secret("SUPABASE_API_KEY")?.to_string(),
            )
            .await
            else {
                return Response::error(String::from("Could not fetch leaderboard!"), 500);
            };

            Response::from_html(
                LeaderboardTemplate {
                    data: leaderboard_entries,
                }
                .render()
                .unwrap(),
            )
        })
        .get_async("/podium", |_req, ctx| async move {
            let Ok(podium_data) = fetch_podium_data(
                ctx.secret("SUPABASE_API_URL")?.to_string(),
                ctx.secret("SUPABASE_API_KEY")?.to_string(),
            )
            .await
            else {
                return Response::error("Bad Request", 400);
            };
            Response::from_html(PodiumTemplate { data: podium_data }.render().unwrap())
        })
        .get_async("/user/:username", |_req, ctx| async move {
            let username = match ctx.param("username") {
                Some(username) => username.replace("%20", " "),
                None => return Response::error(String::from("Could not fetch leaderboard!"), 500),
            };
            let Ok(mut data) = fetch_user_data(
                &username,
                ctx.secret("SUPABASE_API_URL")?.to_string(),
                ctx.secret("SUPABASE_API_KEY")?.to_string(),
            )
            .await
            else {
                return Response::error("Bad Request", 400);
            };

            let plot_html = generate_plot_html(&mut data.all_times);

            Response::from_html(
                UserTemplate {
                    username: username.to_string(),
                    plot_html,
                    data,
                }
                .render()
                .unwrap(),
            )
        })
        .get_async("/history/:date", |_req, ctx| async move {
            let Some(date) = ctx.param("date") else {
                return Response::error(String::from("Could not fetch leaderboard!"), 500);
            };
            let Ok(data) = fetch_results(
                date,
                ctx.secret("SUPABASE_API_URL")?.to_string(),
                ctx.secret("SUPABASE_API_KEY")?.to_string(),
            )
            .await
            else {
                return Response::error("Bad Request", 400);
            };

            Response::from_html(
                HistoryTemplate {
                    date: date.to_string(),
                    data,
                }
                .render()
                .unwrap(),
            )
        })
        .get_async("/today", |_req, ctx| async move {
            let Ok(data) = fetch_live_leaderboard(ctx.secret("NYT_S_TOKEN")?.to_string()).await
            else {
                return Response::error("Bad Request", 400);
            };
            Response::from_html(TodayTemplate { data }.render().unwrap())
        })
        .get_async("/recent", |_req, ctx| async move {
            let Ok(most_recent_date) = fetch_most_recent_crossword_date(
                ctx.secret("SUPABASE_API_URL")?.to_string(),
                ctx.secret("SUPABASE_API_KEY")?.to_string(),
            )
            .await
            else {
                return Response::error("Bad Request", 400);
            };
            let dates: Vec<String> = (0..10)
                .map(|i| {
                    (most_recent_date - Duration::days(i))
                        .format("%Y-%m-%d")
                        .to_string()
                })
                .collect();

            Response::from_html(RecentTemplate { dates }.render().unwrap())
        })
        .get_async("/h2h", |_req, ctx| async move {
            let Ok(users) = fetch_usernames_sorted_by_elo(
                ctx.secret("SUPABASE_API_URL")?.to_string(),
                ctx.secret("SUPABASE_API_KEY")?.to_string(),
            )
            .await
            else {
                return Response::error("Bad Request", 400);
            };

            Response::from_html(
                HeadToHeadTemplate {
                    users,
                    ..Default::default()
                }
                .render()
                .unwrap(),
            )
        })
        .get_async("/h2h/:user1/:user2", |_req, ctx| async move {
            let (user1, user2) = match (ctx.param("user1"), ctx.param("user2")) {
                (Some(u1), Some(u2)) => (u1.replace("%20", " "), u2.replace("%20", " ")),
                _ => {
                    return Response::error(String::from("Couldn't fetch head to head stats!"), 500)
                }
            };

            let Ok(users) = fetch_usernames_sorted_by_elo(
                ctx.secret("SUPABASE_API_URL")?.to_string(),
                ctx.secret("SUPABASE_API_KEY")?.to_string(),
            )
            .await
            else {
                return Response::error("Bad Request", 400);
            };
            let h2h_data: HeadToHeadData = match fetch_h2h_data(
                user1.to_string(),
                user2.to_string(),
                ctx.secret("SUPABASE_API_URL")?.to_string(),
                ctx.secret("SUPABASE_API_KEY")?.to_string(),
            )
            .await
            {
                Ok(data) => data,
                _ => models::HeadToHeadData {
                    time_diff_description: String::from("Couldn't fetch head to head stats!"),
                    ..Default::default()
                },
            };

            Response::from_html(
                HeadToHeadTemplate {
                    populated: true,
                    users,
                    data: h2h_data,
                }
                .render()
                .unwrap(),
            )
        })
        .get_async("/styles/styles.css", |_req, _ctx| async move {
            Response::ok(
                ".navbar-custom {
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
            }",
            )
        })
        .run(req, env)
        .await
}
