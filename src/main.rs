use std::time::Instant;

use anyhow::Result;
use axum::{body::Body, http::{header::CONTENT_TYPE, Request, StatusCode}, response::IntoResponse, routing::get, Router};
use chrono::Utc;
use log::{warn, info};
use rand::Rng;
use time::{correct_time_for_query, valid_time_in_zone, valid_time_offsets};

pub mod time;
pub mod draw;

/// The hour at which cats can be generated.
/// [HOUR] and [HOUR] + 12 are both allowed hours for the client. 
const HOUR: u32 = 2;

/// The minute at which cats can be generated.
const MINUTE: u32 = 22;

/// The number of seconds of leeway for clients that think it's 2:22.
/// This means cats can technically be generated [CLIENT_LEEWAY] seconds before
/// and after it's 2:22 somewhere.
const CLIENT_LEEWAY: i64 = 1;

#[tokio::main]
async fn main() -> Result<()> {

    env_logger::init();

    // Generate the app with all the routes
    let app = Router::new()
        .route("/", get(index))
        .route("/cat", get(|request: Request<Body>| async move {
            let make_cat = correct_time_for_query(request.uri().query()).await;

            cat(make_cat)
        }))
        .route("/torna", get(|| async move { cat(false) }))
        .route("/discountcat", get(|| async move {
            // I changed the actual URL for this endpoint on the version I'm hosting.
            // Don't try to cheat cats in >:3
        
            warn!("Free cat endpoint was hit - giving away a free cat!");
            cat(true)
        }));
        // .fallback(get(routes::error404()));

    // port 1474 is the port for my previous project plus one
    let listener = tokio::net::TcpListener::bind("127.0.0.1:1474")
        .await?;
    
    info!("unfortunately we are listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;

    Ok(())
}

/// The index page. This will generate a random background color for the client,
/// and will send JavaScript only if it is a valid time somewhere.
async fn index() -> impl IntoResponse {
    // Figure out if it's the correct time anywhere
    let now = Utc::now();
    let valid = valid_time_offsets().iter().any(|&offset| valid_time_in_zone(now, offset));

    // Generate the background color
    let mut rng = rand::thread_rng();
    let background = (rng.gen_range(100..=255u32) << 16) + (rng.gen_range(100..=255) << 8) + (rng.gen_range(100..=255));

    // Generate index.html (with inline JS).
    // The JS and HTML were somewhat code golfed, but they were kept looking
    // somewhat normal in case further changes need to be made :)
    let index = if valid {
        let js = &format!(r#"<script>a=new Date();d.src={HOUR}-a.getHours()%12|{MINUTE}-a.getMinutes()?"/torna":(e.textContent="{HOUR}:{MINUTE:0>2} make a cat / {HOUR}:{MINUTE:0>2} fer un gat",`/cat?${{a.getTime()}}&`+a.getTimezoneOffset())</script>"#);

        format!(r#"<!DOCTYPE html><html><head><title>makea.cat</title></head><body style="text-align:center;background-color:#{background:x}"><p>make a cat / fer un gat</p><div style="margin:0 auto;width:400px;height:256px;border:1px solid#000"><img src="" id="d"></div><p id="e">come back at {HOUR}:{MINUTE:0>2} / torna a {HOUR}:{MINUTE:0>2}</p>{js}</body></html>"#)
    } else {
        format!(r#"<!DOCTYPE html><html><head><title>makea.cat</title></head><body style="text-align:center;background-color:#{background:x}"><p>make a cat / fer un gat</p><div style="margin:0 auto;width:400px;height:256px;border:1px solid#000"><img src="/torna"></div><p>come back at {HOUR}:{MINUTE:0>2} / torna a {HOUR}:{MINUTE:0>2}</p></body></html>"#)
    };

    // Turn it into a response
    (
        StatusCode::OK,
        [(CONTENT_TYPE, "text/html")],
        index,
    )
}

/// Makes a cat if `cat` is true, telling them to come back later otherwise.
fn cat(cat: bool) -> impl IntoResponse {

    // Render the image
    let png = if cat {
        let start = Instant::now();

        let cat = draw::purchase_cat();

        info!("Made cat in {:?}", start.elapsed());

        cat        
    } else {
        draw::out_of_stock()
    };

    // Turn it into a response
    (
        StatusCode::OK,
        [(CONTENT_TYPE, "image/png")],
        png
    )
}
