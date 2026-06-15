use crate::domain::Reading;

const WEEKDAYS_RO: [&str; 7] = ["Duminică", "Luni", "Marți", "Miercuri", "Joi", "Vineri", "Sâmbătă"];

pub const MONTHS_RO: [&str; 12] = [
    "Ianuarie", "Februarie", "Martie", "Aprilie", "Mai", "Iunie", "Iulie", "August", "Septembrie",
    "Octombrie", "Noiembrie", "Decembrie",
];

pub const MONTHS_SHORT_RO: [&str; 12] =
    ["Ian", "Feb", "Mar", "Apr", "Mai", "Iun", "Iul", "Aug", "Sep", "Oct", "Noi", "Dec"];

pub fn pretty_date(iso: &str) -> String {
    let parts: Vec<&str> = iso.split('-').collect();
    match parts.as_slice() {
        [_year, month, day] => format!("{day}.{month}"),
        other => other.join("-"),
    }
}

pub fn weekday_ro(iso: &str) -> String {
    let parts: Vec<&str> = iso.split('-').collect();
    let [year, month, day] = parts.as_slice() else {
        return String::new();
    };
    let (Ok(y), Ok(m), Ok(d)) = (year.parse::<i64>(), month.parse::<i64>(), day.parse::<i64>()) else {
        return String::new();
    };
    let serial = days_from_civil(y, m, d);
    WEEKDAYS_RO[weekday_index(serial)].to_string()
}

pub fn day_before(iso: &str) -> String {
    let parts: Vec<&str> = iso.split('-').collect();
    let [year, month, day] = parts.as_slice() else {
        return String::new();
    };
    let (Ok(y), Ok(m), Ok(d)) = (year.parse::<i64>(), month.parse::<i64>(), day.parse::<i64>()) else {
        return String::new();
    };
    let (_yy, mm, dd) = civil_from_days(days_from_civil(y, m, d) - 1);
    format!("{dd:02}.{mm:02}")
}

fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

fn civil_from_days(serial: i64) -> (i64, i64, i64) {
    let z = serial + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { y + 1 } else { y };
    (year, month, day)
}

fn weekday_index(serial: i64) -> usize {
    let shifted = (serial % 7 + 11) % 7;
    shifted as usize
}

pub fn weekday_mon0(year: i64, month: i64, day: i64) -> usize {
    let sun0 = weekday_index(days_from_civil(year, month, day));
    (sun0 + 6) % 7
}

pub fn days_in_month(year: i64, month: i64) -> i64 {
    let next = if month >= 12 {
        days_from_civil(year + 1, 1, 1)
    } else {
        days_from_civil(year, month + 1, 1)
    };
    next - days_from_civil(year, month, 1)
}

pub fn next_day(year: i64, month: i64, day: i64) -> (i64, i64, i64) {
    civil_from_days(days_from_civil(year, month, day) + 1)
}

pub fn local_hm(iso: &str) -> String {
    let Some((_date, rest)) = iso.split_once('T') else {
        return String::new();
    };
    let Some((hh_text, after)) = rest.split_once(':') else {
        return String::new();
    };
    let Ok(hh) = hh_text.parse::<i64>() else {
        return String::new();
    };
    let minute: String = after.chars().take(2).collect();
    let local = (hh + 3) % 24;
    format!("{local:02}:{minute}")
}

pub fn reading_text(reading: Reading) -> String {
    match reading {
        Reading::Celsius(value) => format!("{value:.1} °C"),
        Reading::Missing => "indisponibil".to_string(),
    }
}

pub fn reading_short(reading: Reading) -> String {
    match reading {
        Reading::Celsius(value) => format!("{value:.0}°"),
        Reading::Missing => "—".to_string(),
    }
}
