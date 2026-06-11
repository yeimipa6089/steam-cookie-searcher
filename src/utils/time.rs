pub fn format_timestamp(ts: u64) -> String {
    let secs_per_day: u64 = 86400;
    let days_since_epoch = ts / secs_per_day;
    let secs_in_day = ts % secs_per_day;
    let hours = secs_in_day / 3600;
    let minutes = (secs_in_day % 3600) / 60;

    let mut y = 1970i64;
    let mut remaining = days_since_epoch as i64;
    loop {
        let days_in_year = if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) {
            366
        } else {
            365
        };
        if remaining < days_in_year {
            break;
        }
        remaining -= days_in_year;
        y += 1;
    }
    let months_days: &[i64] = if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) {
        &[31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        &[31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut m = 0usize;
    for &md in months_days {
        if remaining < md {
            break;
        }
        remaining -= md;
        m += 1;
    }
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02} UTC",
        y,
        m + 1,
        remaining + 1,
        hours,
        minutes
    )
}
