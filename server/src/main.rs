use std::time::Instant;

use anyhow::Result;
use axum::{body::Body, http::{header::CONTENT_TYPE, HeaderName, Request, StatusCode}, routing::get, Router};
use chrono::{DateTime, Timelike, Utc};
use log::{warn, info, debug};

pub mod draw;

const HOUR: u32 = 2; // 12-hour time
const MINUTE: u32 = 22;

const DATETIME_GRANULARITY: u32 = 30;
const CLIENT_LEEWAY: u32 = 1;

#[tokio::main]
async fn main() -> Result<()> {
    
    env_logger::builder()
        .format_target(false)
        .init();

    let paidcat = |request: Request<Body>| async move {

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

        if !verify_time(time, offset) {
            info!("Bad time {time} and offset {offset}");
            return out_of_stock();
        }

        let t = Instant::now();
        
        let image = purchase_cat();

        info!("Made cat for time {time} and offset {offset} in {:?}", t.elapsed());
        
        image
    };

    let freecat = || async move {
        warn!("Free cat endpoint was hit - giving away a free cat!");
        purchase_cat()
    };

    let app = Router::new()
        .route("/cat", get(paidcat))
        .route("/freecat", get(freecat));
        // .fallback(get(routes::error404()));

    // port 1474 is the port for my previous project plus one
    let listener = tokio::net::TcpListener::bind("127.0.0.1:1474")
        .await
        .unwrap();

    info!("unfortunately we are listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();

    Ok(())
}

pub fn purchase_cat() -> (StatusCode, [(HeaderName, &'static str); 1], Vec<u8>) {
    (
        StatusCode::OK,
        [(CONTENT_TYPE, "image/png")],
        draw::purchase_cat(),
    )
}

pub fn out_of_stock() -> (StatusCode, [(HeaderName, &'static str); 1], Vec<u8>) {
    (
        StatusCode::OK,
        [(CONTENT_TYPE, "image/png")],
        draw::out_of_stock()
    )
}

pub fn verify_time(time: i64, offset: i64) -> bool {

    let now = Utc::now();

    // The client must have a valid offset
    if offset % 15 != 0 || offset.abs() > 12 * 60 {
        debug!("Offset {offset} is not multiple of 15 or not in [-720, 720]");
        return false;
    }

    // The client cannot be too desynced (here we chose 15s)
    if now.timestamp_millis().abs_diff(time) > 15_000 {
        debug!("Client system time {time} drifts too much ({}ms > 15000ms)", now.timestamp_millis().abs_diff(time));
        return false;
    }

    // Make sure the client has a valid date
    let Some(n) = DateTime::from_timestamp_millis(time - offset * 60 * 1000) else {
        debug!("Could not construct date for ms {} with time {time} and offset {offset}", time - offset * 60 * 1000);
        return false;
    };

    // The client must actually think it's 2:22
    if !(n.hour12().1 == HOUR && n.minute() == MINUTE) {
        debug!("Client thinks it's {}:{} instead of {HOUR}:{MINUTE}", n.hour12().1, n.minute());
        return false;
    }

    // The secret sauce: verify that it's valid cat time somewhere.
    let min = now.minute() % DATETIME_GRANULARITY;
    let sec = now.second();
 
    let min_before = (min - 1) % DATETIME_GRANULARITY;
    let min_after = (min + 1) % DATETIME_GRANULARITY;

    let valid_on_server = min == MINUTE % DATETIME_GRANULARITY
        || (min == min_before && sec >= 60 - CLIENT_LEEWAY)
        || (min == min_after && sec < CLIENT_LEEWAY);

    if !valid_on_server {
        debug!("It's not {HOUR}:{MINUTE} anywhere");
        return false;
    }

    // Must be good!
    true
}