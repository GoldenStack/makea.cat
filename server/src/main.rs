use anyhow::Result;
use axum::{body::Body, http::{header::CONTENT_TYPE, Request, StatusCode}, routing::get, Router};
use chrono::{DateTime, Timelike, Utc};

pub mod draw;

const HOUR: u32 = 2; // 12-hour time
const MINUTE: u32 = 22;

const DATETIME_GRANULARITY: u32 = 30;
const CLIENT_LEEWAY: u32 = 1;

#[tokio::main]
async fn main() -> Result<()> {

    let paidcat = |request: Request<Body>| async move {

        let query = request.uri().query();
        
        let parts = query.and_then(|t| t.split_once("&"));

        let allowed = parts
            .and_then(|(time, offset)| {
                let time = time.parse::<i64>().ok()?;
                let offset = offset.parse::<i64>().ok()?;

                Some((time, offset))
            }).filter(|(time, offset)| verify_time(*time, *offset))
            .is_some();

        let image = if allowed {
            draw::purchase_cat()
        } else {
            draw::out_of_stock()
        };
        
        (
            StatusCode::OK,
            [(CONTENT_TYPE, "image/png")],
            image,
        )
    };

    let freecat = || async move {
        (
            StatusCode::OK,
            [(CONTENT_TYPE, "image/png")],
            draw::purchase_cat()
        )
    };

    let app = Router::new()
        .route("/cat", get(paidcat))
        .route("/freecat", get(freecat));
        // .fallback(get(routes::error404()));

    // port 1474 is the port for my previous project plus one
    let listener = tokio::net::TcpListener::bind("127.0.0.1:1474")
        .await
        .unwrap();

    println!("unfortunately we are listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();

    Ok(())
}

pub fn verify_time(time: i64, offset: i64) -> bool {

    let now = Utc::now();

    // The client must have a valid offset
    if offset % 15 != 0 || offset.abs() > 12 * 60 {
        return false;
    }

    // The client cannot be too desynced (here we chose 15s)
    if now.timestamp_millis().abs_diff(time) > 15_000 {
        return false;
    }

    // Make sure the client has a valid date
    let Some(n) = DateTime::from_timestamp_millis(time - offset * 60 * 1000) else {
        return false;
    };

    // The client must actually think it's 2:22
    if !(n.hour12().1 == HOUR && n.minute() == MINUTE) {
        return false;
    }

    // The secret sauce: verify that it's valid cat time somewhere.
    let now = Utc::now();
    let min = now.minute() % DATETIME_GRANULARITY;
    let sec = now.second();

 
    let min_before = (min - 1) % DATETIME_GRANULARITY;
    let min_after = (min + 1) % DATETIME_GRANULARITY;

    let valid_on_server = min == MINUTE % DATETIME_GRANULARITY
        || (min == min_before && sec >= 60 - CLIENT_LEEWAY)
        || (min == min_after && sec < CLIENT_LEEWAY);

    if !valid_on_server {
        return false;
    }

    // Must be good!
    true
}