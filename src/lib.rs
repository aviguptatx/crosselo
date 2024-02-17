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
    TodayTemplate, UserTemplate, CSS_STYLES,
};
use util::{fetch_live_leaderboard, generate_plot_html};

use worker::*;

#[event(fetch)]
async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    let router = Router::new();
    router
        .get_async("/", |_req, ctx| async move { handle_index(&ctx).await })
        .get_async("/index/:db_name", |_req, ctx| async move {
            handle_index(&ctx).await
        })
        .get_async(
            "/podium",
            |_req, ctx| async move { handle_podium(&ctx).await },
        )
        .get_async("/user/:username", |_req, ctx| async move {
            handle_user(&ctx).await
        })
        .get_async("/history/:date", |_req, ctx| async move {
            handle_history(&ctx).await
        })
        .get_async(
            "/today",
            |_req, ctx| async move { handle_today(&ctx).await },
        )
        .get_async(
            "/recent",
            |_req, ctx| async move { handle_recent(&ctx).await },
        )
        .get_async("/h2h", |_req, ctx| async move { handle_h2h(&ctx).await })
        .get_async("/h2h/:user1/:user2", |_req, ctx| async move {
            handle_h2h(&ctx).await
        })
        .get_async("/styles/styles.css", |_req, _ctx| async move {
            Response::ok(CSS_STYLES)
        })
        .run(req, env)
        .await
}

async fn handle_index<T>(ctx: &RouteContext<T>) -> Result<Response> {
    let db_name = match ctx.param("db_name") {
        Some(str) => str,
        None => "all",
    };

    let Ok(leaderboard_entries) = fetch_leaderboard_from_db(
        db_name,
        ctx.secret("SUPABASE_API_URL")?.to_string(),
        ctx.secret("SUPABASE_API_KEY")?.to_string(),
    )
    .await
    else {
        return Response::error("Couldn't fetch leaderboard from database", 500);
    };
    Response::from_html(
        LeaderboardTemplate {
            data: leaderboard_entries,
        }
        .render()
        .unwrap(),
    )
}

async fn handle_podium<T>(ctx: &RouteContext<T>) -> Result<Response> {
    let Ok(podium_data) = fetch_podium_data(
        ctx.secret("SUPABASE_API_URL")?.to_string(),
        ctx.secret("SUPABASE_API_KEY")?.to_string(),
    )
    .await
    else {
        return Response::error("Couldn't fetch results from database", 500);
    };
    Response::from_html(PodiumTemplate { data: podium_data }.render().unwrap())
}

async fn handle_user<T>(ctx: &RouteContext<T>) -> Result<Response> {
    let username = match ctx.param("username") {
        Some(username) => username.replace("%20", " "),
        None => return Response::error("Couldn't process username parameter", 500),
    };
    let Ok(mut data) = fetch_user_data(
        &username,
        ctx.secret("SUPABASE_API_URL")?.to_string(),
        ctx.secret("SUPABASE_API_KEY")?.to_string(),
    )
    .await
    else {
        return Response::error("Couldn't fetch user data from database", 500);
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
}

async fn handle_history<T>(ctx: &RouteContext<T>) -> Result<Response> {
    let Some(date) = ctx.param("date") else {
        return Response::error("Couldn't process date parameter", 500);
    };
    let Ok(data) = fetch_results(
        date,
        ctx.secret("SUPABASE_API_URL")?.to_string(),
        ctx.secret("SUPABASE_API_KEY")?.to_string(),
    )
    .await
    else {
        return Response::error("Couldn't fetch results from database", 500);
    };

    Response::from_html(
        HistoryTemplate {
            date: date.to_string(),
            data,
        }
        .render()
        .unwrap(),
    )
}

async fn handle_today<T>(ctx: &RouteContext<T>) -> Result<Response> {
    let Ok(data) = fetch_live_leaderboard(ctx.secret("NYT_S_TOKEN")?.to_string()).await else {
        return Response::error("Couldn't fetch live leaderboard from NYT API", 500);
    };
    Response::from_html(TodayTemplate { data }.render().unwrap())
}

async fn handle_recent<T>(ctx: &RouteContext<T>) -> Result<Response> {
    let Ok(most_recent_date) = fetch_most_recent_crossword_date(
        ctx.secret("SUPABASE_API_URL")?.to_string(),
        ctx.secret("SUPABASE_API_KEY")?.to_string(),
    )
    .await
    else {
        return Response::error(
            "Couldn't fetch most recent crossword date from database",
            500,
        );
    };
    let dates: Vec<String> = (0..10)
        .map(|i| {
            (most_recent_date - Duration::days(i))
                .format("%Y-%m-%d")
                .to_string()
        })
        .collect();

    Response::from_html(RecentTemplate { dates }.render().unwrap())
}

async fn handle_h2h<T>(ctx: &RouteContext<T>) -> Result<Response> {
    let Ok(users) = fetch_usernames_sorted_by_elo(
        ctx.secret("SUPABASE_API_URL")?.to_string(),
        ctx.secret("SUPABASE_API_KEY")?.to_string(),
    )
    .await
    else {
        return Response::error("Couldn't fetch usernames from database", 500);
    };

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

    let h2h_data: HeadToHeadData = match fetch_h2h_data(
        user1,
        user2,
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
}
