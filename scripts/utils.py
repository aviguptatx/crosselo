import os
import time
from datetime import datetime, timedelta

import pytz
import requests

from db import supabase_client


def to_iso(datetime_obj):
    return datetime_obj.strftime("%Y-%m-%d")


def today_eastern():
    utc_now = datetime.utcnow()
    et_now = utc_now.replace(tzinfo=pytz.utc).astimezone(pytz.timezone("US/Mountain"))
    return et_now


def daterange(start_date, end_date):
    for n in range(int((end_date - start_date).days) + 1):
        yield start_date + timedelta(n)


def get_usernames_sorted_by_elo():
    data = (
        supabase_client.table("all_rust")
        .select("*")
        .order("elo", desc=True)
        .execute()
        .data
    )

    return [row["username"] for row in data]


def get_most_recent_crossword_date():
    data = (
        supabase_client.table("results_rust")
        .select("*")
        .order("date", desc=True)
        .limit(1)
        .execute()
        .data
    )
    return datetime.strptime(data[0]["date"], "%Y-%m-%d")


def fetch_leaderboard(date_str):
    data = (
        supabase_client.table("results_rust")
        .select("*")
        .eq("date", date_str)
        .order("time")
        .execute()
        .data
    )

    times = [entry["time"] for entry in data]
    leaderboard = []

    for entry in data:
        leaderboard.append(
            {
                "Rank": times.index(entry["time"]) + 1,
                "Username": entry["username"],
                "Time": entry["time"],
            }
        )

    return leaderboard


def fetch_today_leaderboard(num_retries=3, retry_delay_seconds=5):
    today_iso = to_iso(today_eastern())

    # The NYT API can be intermittent, sometimes failing to authenticate.
    for _ in range(num_retries):
        try:
            response = requests.get(
                f"https://www.nytimes.com/svc/crosswords/v6/leaderboard/mini/{today_iso}.json",
                headers={
                    "accept": "application/json",
                },
                cookies={
                    "nyt-s": os.environ.get("NYT_S_TOKEN"),
                },
            )

            print(response)

            return [
                entry
                for entry in response.json()["data"]
                if entry.get("score", {}).get("secondsSpentSolving", 0)
            ]
        except:
            time.sleep(retry_delay_seconds)


def fetch_live_leaderboard():
    response = requests.get(
        f"https://www.nytimes.com/svc/crosswords/v6/leaderboard/mini.json",
        headers={
            "accept": "application/json",
            "nyt-s": os.environ.get("NYT_S_TOKEN"),
        },
    )

    return [
        entry
        for entry in response.json()["data"]
        if entry.get("score", {}).get("secondsSpentSolving", 0)
    ]


def format_time(seconds):
    minutes, seconds = divmod(seconds, 60)
    return f"{int(minutes):02d}:{int(seconds):02d}"
