use askama::Template;
use chrono::Duration;
use postgrest::Postgrest;
use worker::{event, Context, Env, Request, Response, Result, RouteContext, Router};

mod database;
mod models;
mod templates;
mod util;

use crate::database::{
    fetch_h2h_data, fetch_leaderboard_from_db, fetch_most_recent_crossword_date, fetch_podium_data,
    fetch_results, fetch_user_data, fetch_usernames_sorted_by_elo,
};
use crate::templates::{
    HeadToHeadTemplate, HistoryTemplate, LeaderboardTemplate, PodiumTemplate, RecentTemplate,
    TodayTemplate, UserTemplate, CSS_STYLES,
};
use crate::util::{fetch_live_leaderboard, generate_box_plot_html, generate_scatter_plot_html};

fn get_db_client<T>(ctx: &RouteContext<T>) -> Result<Postgrest> {
    let url = ctx.secret("SUPABASE_API_URL")?.to_string();
    let key = ctx.secret("SUPABASE_API_KEY")?.to_string();

    let client = Postgrest::new(url).insert_header("apikey", key);

    Ok(client)
}

#[event(fetch)]
async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    let router = Router::new();
    router
        .get_async("/", |_req, ctx| async move {
            handle_index(&ctx, &get_db_client(&ctx)?).await
        })
        .get_async("/index/:db_name", |_req, ctx| async move {
            handle_index(&ctx, &get_db_client(&ctx)?).await
        })
        .get_async("/podium", |_req, ctx| async move {
            handle_podium(&get_db_client(&ctx)?).await
        })
        .get_async("/user/:username", |_req, ctx| async move {
            handle_user(&ctx, &get_db_client(&ctx)?).await
        })
        .get_async("/history/:date", |_req, ctx| async move {
            handle_history(&ctx, &get_db_client(&ctx)?).await
        })
        .get_async(
            "/today",
            |_req, ctx| async move { handle_today(&ctx).await },
        )
        .get_async("/recent", |_req, ctx| async move {
            handle_recent(&get_db_client(&ctx)?).await
        })
        .get_async("/h2h", |_req, ctx| async move {
            handle_h2h(&ctx, &get_db_client(&ctx)?).await
        })
        .get_async("/h2h/:user1/:user2", |_req, ctx| async move {
            handle_h2h(&ctx, &get_db_client(&ctx)?).await
        })
        .get_async("/styles/styles.css", |_req, _ctx| async move {
            Response::ok(CSS_STYLES)
        })
        .run(req, env)
        .await
}

async fn handle_index<T>(ctx: &RouteContext<T>, client: &Postgrest) -> Result<Response> {
    let db_name = ctx.param("db_name").map_or("all", |str| str).to_string() + "_rust";

    let data = fetch_leaderboard_from_db(&db_name, client)
        .await
        .map_err(|e| format!("Couldn't fetch leaderboard from database: {e}"))?;

    Response::from_html(LeaderboardTemplate { data }.render().unwrap())
}

async fn handle_podium(client: &Postgrest) -> Result<Response> {
    let podium_data = fetch_podium_data(client)
        .await
        .map_err(|e| format!("Couldn't fetch results from database: {e}"))?;

    Response::from_html(PodiumTemplate { data: podium_data }.render().unwrap())
}

async fn handle_user<T>(ctx: &RouteContext<T>, client: &Postgrest) -> Result<Response> {
    let username = match ctx.param("username") {
        Some(username) => username.replace("%20", " "),
        None => return Err("Couldn't process username parameter".into()),
    };

    let mut data = fetch_user_data(&username, client)
        .await
        .map_err(|e| format!("Couldn't fetch user data from database: {e}"))?;

    let scatter_plot_html = generate_scatter_plot_html(vec![&mut data.times_excluding_saturday])
        .unwrap_or_else(|_| String::from("Need more times before we can plot!"));

    let box_plot_html = generate_box_plot_html(vec![&mut data.times_excluding_saturday])
        .unwrap_or_else(|_| String::from("Need more times before we can plot!"));

    Response::from_html(
        UserTemplate {
            username,
            scatter_plot_html,
            box_plot_html,
            top_times: data.all_times[..3].to_vec(),
        }
        .render()
        .unwrap(),
    )
}

async fn handle_history<T>(ctx: &RouteContext<T>, client: &Postgrest) -> Result<Response> {
    let date = ctx
        .param("date")
        .ok_or("Couldn't process date parameter")?
        .to_string();
    let data = fetch_results(&date, client)
        .await
        .map_err(|e| format!("Couldn't fetch results from database: {e}"))?;

    Response::from_html(HistoryTemplate { date, data }.render().unwrap())
}

async fn handle_today<T>(ctx: &RouteContext<T>) -> Result<Response> {
    let data = fetch_live_leaderboard(ctx.secret("NYT_S_TOKEN")?.to_string())
        .await
        .map_err(|e| format!("Couldn't fetch live leaderboard from NYT API: {e}"))?;

    Response::from_html(TodayTemplate { data }.render().unwrap())
}

async fn handle_recent(client: &Postgrest) -> Result<Response> {
    let most_recent_date = fetch_most_recent_crossword_date(client)
        .await
        .map_err(|e| format!("Couldn't fetch most recent crossword date from database: {e}"))?;

    let dates: Vec<String> = (0..10)
        .map(|i| {
            (most_recent_date - Duration::days(i))
                .format("%Y-%m-%d")
                .to_string()
        })
        .collect();

    Response::from_html(RecentTemplate { dates }.render().unwrap())
}

async fn handle_h2h<T>(ctx: &RouteContext<T>, client: &Postgrest) -> Result<Response> {
    let users = fetch_usernames_sorted_by_elo(client)
        .await
        .map_err(|e| format!("Couldn't fetch usernames from database: {e}"))?;

    let (user1, user2) = match (ctx.param("user1"), ctx.param("user2")) {
        (Some(u1), Some(u2)) => (u1.replace("%20", " "), u2.replace("%20", " ")),
        _ => {
            return Response::from_html(
                HeadToHeadTemplate {
                    users,
                    ..Default::default()
                }
                .render()
                .unwrap(),
            )
        }
    };

    let mut user1_data = fetch_user_data(&user1, client)
        .await
        .map_err(|e| format!("Couldn't fetch user1 data from database: {e}"))?;

    let mut user2_data = fetch_user_data(&user2, client)
        .await
        .map_err(|e| format!("Couldn't fetch user2 data from database: {e}"))?;

    let box_plot_html =
        generate_box_plot_html(vec![&mut user1_data.all_times, &mut user2_data.all_times])
            .unwrap_or_else(|_| String::from("Need more times before we can generate box plot!"));

    let scatter_plot_html =
        generate_scatter_plot_html(vec![&mut user1_data.all_times, &mut user2_data.all_times])
            .unwrap_or_else(|_| {
                String::from("Need more times before we can generate scatter plot!")
            });

    let data = fetch_h2h_data(user1, user2, client).await.ok();

    Response::from_html(
        HeadToHeadTemplate {
            users,
            data,
            box_plot_html,
            scatter_plot_html,
        }
        .render()
        .unwrap(),
    )
}
