use std::sync::OnceLock;

use chrono::{DateTime, TimeDelta, Timelike, Utc};
use log::{debug, info};

use crate::{CLIENT_LEEWAY, HOUR, MINUTE};

/// Returns whether or not a cat should be returned for the provided URL query.
/// 
/// A valid query consists of the client's time, an ampersand (`&`), and the
/// client's time zone offset.
/// 
/// Client times are technically unnecessary, but prevent static URLS from
/// working between cats, which is beneficial. Client offsets consist of any
/// valid IANA tz database time, meaning that for most minutes it's not possible
/// anywhere for there to be a valid time.
pub async fn correct_time_for_query(query: Option<&str>) -> bool {
    let parts = query.and_then(|t| t.split_once("&"))
        .and_then(|(time, offset)| {
            let time = time.parse::<i64>().ok()?;
            let offset = offset.parse::<i64>().ok()?;

            Some((time, offset))
        });

    let Some((time, offset)) = parts else {
        info!("Bad URI query {}", query.map(|q| format!("'{q}'")).unwrap_or("N/A".into()));
        return false;
    };

    if !verify_time(time, offset).is_some() {
        info!("Bad time {time} and offset {offset}");
        return false;
    }

    info!("Good time {time} and offset {offset}");
    
    true
}

/// Verifies that the client time and offset are valid. This will perform a few
/// checks:
/// - The client must have a valid time zone offset according to the IANA tz
///   database
/// - It must be the correct time in the client's time zone (except for a small
///   [CLIENT_LEEWAY]).
/// 
/// There are a few more checks that are technically unnecessary for the
/// anticheat, but render static URLs useless and make it slightly harder to
/// reverse engineer:
/// - The client's time cannot have more than 15 seconds of drift from the
///   actual time
/// - The client's time, taking offset into account, must actually be the
///   correct time for them (no leeway here, because this is what the client
///   thinks).
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

/// Returns whether or not the provided date has the correct [HOUR] and [MINUTE]
/// in the given time zone offset. This will allow a leeway of [CLIENT_LEEWAY]
/// in either direction.
/// 
/// Failure of operations involving time is considered an invalid date and will
/// return false.
pub fn valid_time_in_zone(now: DateTime<Utc>, offset: i64) -> bool {
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

/// Returns the list of every valid time zone offset, per the time zone list.
/// This will panic on most errors because it's meant to run once and is not
/// some core function that requires incredible reliability.
pub fn valid_time_offsets() -> &'static Vec<i64> {
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