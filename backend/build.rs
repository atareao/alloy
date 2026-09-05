fn main() {
    // Set BUILD_DATE to current UTC time in ISO 8601 format
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap();
    let secs = now.as_secs();
    // Format as ISO 8601 date
    let days = secs / 86400;
    // Simple date calculation from Unix epoch
    let mut y = 1970i64;
    let mut remaining = days as i64;
    loop {
        let days_in_year = if is_leap(y) { 366 } else { 365 };
        if remaining < days_in_year {
            break;
        }
        remaining -= days_in_year;
        y += 1;
    }
    let months_days: &[(i64, i64)] = if is_leap(y) {
        &[
            (31, 0),
            (29, 31),
            (31, 60),
            (30, 91),
            (31, 121),
            (30, 152),
            (31, 182),
            (31, 213),
            (30, 244),
            (31, 274),
            (30, 305),
            (31, 335),
        ]
    } else {
        &[
            (31, 0),
            (28, 31),
            (31, 59),
            (30, 90),
            (31, 120),
            (30, 151),
            (31, 181),
            (31, 212),
            (30, 243),
            (31, 273),
            (30, 304),
            (31, 334),
        ]
    };
    let mut m = 1u32;
    for &(days_in_month, _accum) in months_days {
        if remaining < days_in_month {
            break;
        }
        remaining -= days_in_month;
        m += 1;
    }
    let d = remaining + 1;
    let hours = (secs % 86400) / 3600;
    let minutes = (secs % 3600) / 60;
    let seconds = secs % 60;
    let date_str = format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        y, m, d, hours, minutes, seconds
    );
    println!("cargo:rustc-env=BUILD_DATE={}", date_str);
}

fn is_leap(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}
