use std::{sync::OnceLock, time::Instant};

use anyhow::Result;
use axum::{body::Body, http::{header::CONTENT_TYPE, HeaderName, Request, StatusCode}, response::IntoResponse, routing::get, Router};
use chrono::{DateTime, TimeDelta, Timelike, Utc};
use log::{warn, info, debug};
use rand::Rng;

pub mod draw;

const HOUR: u32 = 2; // 12-hour time
const MINUTE: u32 = 22;

const CLIENT_LEEWAY: i64 = 1;

#[tokio::main]
async fn main() -> Result<()> {

    env_logger::builder()
        .format_target(false)
        .init();

    let index = || async move {

        let index = include_str!("../public/index.html");

        let mut rng = rand::thread_rng();

        let index = index.replace("{{ BACKGROUND-COLOR }}", &format!("{},{},{}",
            rng.gen_range(100..=255),
            rng.gen_range(100..=255),
            rng.gen_range(100..=255),
        ));

        let now = Utc::now();

        let valid = valid_time_offsets().iter().any(|&offset| valid_time_in_zone(now, offset));

        let js = concat!("<script>", include_str!("../public/main.js"), "</script>");

        let index = index.replace("{{ IMAGE_SRC }}", if valid { "" } else { "/cat?torna" });
        let index = index.replace("{{ JAVASCRIPT }}", if valid { js } else { "" });

        (
            StatusCode::OK,
            [(CONTENT_TYPE, "text/html")],
            index,
        )
    };

    let freecat = || async move {
        warn!("Free cat endpoint was hit - giving away a free cat!");
        purchase_cat()
    };

    let app = Router::new()
        .route("/", get(index))
        .route("/cat", get(verified_cat))
        .route("/discountcat", get(freecat));
        // .fallback(get(routes::error404()));

    // port 1474 is the port for my previous project plus one
    let listener = tokio::net::TcpListener::bind("127.0.0.1:1474")
        .await
        .unwrap();

    info!("unfortunately we are listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();

    Ok(())
}

pub async fn verified_cat(request: Request<Body>) -> impl IntoResponse {
    let query = request.uri().query();
    let parts = query.and_then(|t| t.split_once("&"))
        .and_then(|(time, offset)| {
            let time = time.parse::<i64>().ok()?;
            let offset = offset.parse::<i64>().ok()?;

            Some((time, offset))
        });

    let Some((time, offset)) = parts else {
        info!("Bad URI query {}", query.map(|q| format!("'{q}'")).unwrap_or("N/A".into()));
        return out_of_stock();
    };

    if !verify_time(time, offset).is_some() {
        info!("Bad time {time} and offset {offset}");
        return out_of_stock();
    }

    let t = Instant::now();
    
    let image = purchase_cat();

    info!("Made cat for time {time} and offset {offset} in {:?}", t.elapsed());
    
    image
}

fn purchase_cat() -> (StatusCode, [(HeaderName, &'static str); 1], Vec<u8>) {
    (
        StatusCode::OK,
        [(CONTENT_TYPE, "image/png")],
        draw::purchase_cat(),
    )
}

fn out_of_stock() -> (StatusCode, [(HeaderName, &'static str); 1], Vec<u8>) {
    (
        StatusCode::OK,
        [(CONTENT_TYPE, "image/png")],
        draw::out_of_stock()
    )
}

pub fn verify_time(time: i64, offset: i64) -> Option<()> {

    let now = Utc::now();

    // The client must have an offset that corresponds to a valid time zone
    if !valid_time_offsets().contains(&offset) {
        debug!("Offset {offset} not in IANA time zone database");
        return None;
    }

    // Make sure the local time is actually valid
    if !valid_time_in_zone(now, offset) {
        debug!("Not {HOUR}:{MINUTE:0>2} in time offset {offset}");
        return None;
    }

    // Client time checks

    // The client cannot be too desynced (here we chose 15s)
    if now.timestamp_millis().abs_diff(time) > 15_000 {
        debug!("Client system time {time} drifts too much ({}ms > 15000ms)", now.timestamp_millis().abs_diff(time));
        return None;
    }

    // Client must think it's actually the correct time
    let time = DateTime::from_timestamp_millis(time)?.checked_sub_signed(TimeDelta::minutes(offset))?;
    if time.hour12().1 != HOUR || time.minute() != MINUTE {
        debug!("Client thinks it's {}:{:0>2} instead of {HOUR}:{MINUTE:0>2}", time.hour12().1, time.minute());
        return None;
    }

    // Must be good!
    Some(())
}

fn valid_time_in_zone(now: DateTime<Utc>, offset: i64) -> bool {
    (|| {
        let offset = TimeDelta::try_minutes(offset)?;
        let time = now.checked_sub_signed(offset)?;

        let delta = TimeDelta::min(
            (time.with_hour(HOUR)?.with_minute(MINUTE)?.with_second(30)? - time).abs(),
            (time.with_hour(12 + HOUR)?.with_minute(MINUTE)?.with_second(30)? - time).abs(),
        );

        if delta <= TimeDelta::try_seconds(30 + CLIENT_LEEWAY)? {
            Some(())
        } else {
            None
        }
    })().is_some()
}

fn valid_time_offsets() -> &'static Vec<i64> {
    static OFFSETS: OnceLock<Vec<i64>> = OnceLock::new();
    OFFSETS.get_or_init(|| {
        let zones = include_str!("../time-zones.txt");

        zones.lines().map(|line| {
            let (sign, line) = line.split_at(1);
            let (hour, minute) = line.split_once(":").unwrap();

            let sign = match sign {
                "+" => 1,
                "-" => -1,
                c => panic!("Found invalid sign {c} while parsing line"),
            };

            let hour = hour.parse::<i64>().unwrap();
            let minute = minute.parse::<i64>().unwrap();

            // Multiply -1 because offsets are negated;
            // e.g. offset for UTC-06:00 is 360.
            -1 * sign * (hour * 60 + minute)
        }).collect::<Vec<_>>()
    })
}