pub fn hh_mm_ss(duration_ms: u64) -> String {
    let seconds_total = duration_ms / 1000;
    let minutes_total = seconds_total / 60;
    let hours_total = minutes_total / 60;

    let seconds = seconds_total % 60;
    let minutes = minutes_total % 60;

    if hours_total > 0 {
        format!("{hours_total:02}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes:02}:{seconds:02}")
    }
}
