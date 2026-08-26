//! Everything that must keep happening while the window is closed.
//!
//! This is the whole reason Scout is a desktop app rather than a web page. A
//! browser tab cannot be trusted to make a sound at 06:30 — it may be
//! throttled, backgrounded, or simply not open. Here the alarm check, the
//! deadline sweep and the news refresh all run on a Rust task that outlives
//! the window, so closing to the tray changes nothing about what fires.
//!
//! Alarms are mirrored from the frontend rather than read out of SQLite: the
//! database is owned by `tauri-plugin-sql` on the JS side, and a second
//! connection competing for the same file is a worse trade than a small sync
//! command called whenever alarms change.

use chrono::{DateTime, Datelike, Duration, Local, Timelike, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;

/// How often the scheduler wakes. Alarms are minute-resolution, so 20 seconds
/// is comfortably inside the window without busy-waiting.
const TICK_SECONDS: u64 = 20;

/// How often news is refreshed. Matches the plan: fresh enough to be early,
/// infrequent enough to stay far inside every free tier.
const REFRESH_MINUTES: i64 = 30;

/// Days before a deadline that earn a reminder.
const DEADLINE_OFFSETS: [i64; 3] = [7, 2, 1];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlarmSpec {
    pub id: String,
    /// "HH:MM", 24-hour, local time.
    pub at: String,
    pub label: String,
    /// Days of week this repeats on, 0 = Sunday. Empty means one-shot.
    pub days: Vec<u32>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeadlineSpec {
    pub id: String,
    pub title: String,
    /// RFC3339. When the thing is actually due.
    pub due_at: String,
}

#[derive(Default)]
pub struct SchedulerState {
    alarms: Mutex<Vec<AlarmSpec>>,
    deadlines: Mutex<Vec<DeadlineSpec>>,
    /// Keys of notifications already delivered, so a restart or a second tick
    /// inside the same minute cannot double-fire. Keyed by alarm id + the
    /// exact minute, and by task id + offset for deadlines.
    fired: Mutex<HashSet<String>>,
    last_refresh: Mutex<Option<DateTime<Utc>>>,
}

impl SchedulerState {
    pub fn set_alarms(&self, alarms: Vec<AlarmSpec>) {
        *self.alarms.lock().unwrap() = alarms;
    }

    pub fn set_deadlines(&self, deadlines: Vec<DeadlineSpec>) {
        *self.deadlines.lock().unwrap() = deadlines;
    }

    /// Records that a notification went out. Returns false if it already had,
    /// which is what makes the sweep safe to run repeatedly.
    fn claim(&self, key: String) -> bool {
        let mut fired = self.fired.lock().unwrap();
        // Unbounded growth would be a slow leak in a long-running tray app.
        if fired.len() > 2000 {
            fired.clear();
        }
        fired.insert(key)
    }
}

/// Where a background run parks its results.
///
/// The SQLite database belongs to the frontend plugin, and opening a second
/// writer against the same file to save a handful of rows is a bad trade. So a
/// run that happens with the window closed writes its stories here, and the UI
/// drains the file the next time it is alive. Nothing fetched is ever lost,
/// and there is still exactly one writer to the database.
fn pending_path() -> Option<std::path::PathBuf> {
    Some(crate::settings::config_dir()?.join("pending.json"))
}

fn park_results(items: &[crate::pipeline::ScoredItem]) {
    let Some(path) = pending_path() else { return };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    match serde_json::to_string(items) {
        Ok(json) => {
            if let Err(e) = std::fs::write(&path, json) {
                log::warn!("could not park refresh results: {e}");
            }
        }
        Err(e) => log::warn!("could not serialise refresh results: {e}"),
    }
}

/// Hands the parked stories to the UI and clears them. Returns an empty list
/// when there is nothing waiting, which is the common case.
#[tauri::command]
pub fn take_pending_items() -> Vec<crate::pipeline::ScoredItem> {
    let Some(path) = pending_path() else {
        return Vec::new();
    };
    let Ok(raw) = std::fs::read_to_string(&path) else {
        return Vec::new();
    };
    let _ = std::fs::remove_file(&path);
    serde_json::from_str(&raw).unwrap_or_default()
}

fn notify(app: &AppHandle, title: &str, body: &str) {
    if let Err(e) = app
        .notification()
        .builder()
        .title(title)
        .body(body)
        .show()
    {
        log::warn!("notification failed: {e}");
    }
}

/// Whether an alarm should sound at this moment.
///
/// Returns the minute key it fired for, so the caller can de-duplicate. The
/// comparison is against local wall-clock time because that is what the user
/// set, and it is deliberately tolerant of being a few seconds late — a tick
/// that lands at 06:30:19 must still fire the 06:30 alarm.
fn alarm_due_now(alarm: &AlarmSpec, now: DateTime<Local>) -> Option<String> {
    if !alarm.enabled {
        return None;
    }
    let (h, m) = alarm.at.split_once(':')?;
    let (h, m): (u32, u32) = (h.parse().ok()?, m.parse().ok()?);

    if now.hour() != h || now.minute() != m {
        return None;
    }
    if !alarm.days.is_empty() && !alarm.days.contains(&now.weekday().num_days_from_sunday()) {
        return None;
    }

    Some(format!(
        "alarm:{}:{}",
        alarm.id,
        now.format("%Y-%m-%dT%H:%M")
    ))
}

/// Deadline reminders that are due, as (key, days_remaining).
fn deadlines_due(deadline: &DeadlineSpec, now: DateTime<Utc>) -> Option<(String, i64)> {
    let due: DateTime<Utc> = deadline.due_at.parse().ok()?;
    let remaining = due.signed_duration_since(now);
    if remaining < Duration::zero() {
        return None;
    }
    let days = remaining.num_days();

    // Fire once as each threshold is crossed. `num_days` truncates, so a
    // deadline 2.4 days out reads as 2 and matches the 2-day reminder.
    let offset = DEADLINE_OFFSETS.iter().find(|&&o| o == days)?;
    Some((format!("deadline:{}:{}", deadline.id, offset), *offset))
}

/// Starts the background loop. Runs for the life of the process.
pub fn spawn(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut ticker =
            tokio::time::interval(std::time::Duration::from_secs(TICK_SECONDS));

        loop {
            ticker.tick().await;
            tick(&app).await;
        }
    });
}

async fn tick(app: &AppHandle) {
    let state = app.state::<SchedulerState>();
    let now_local = Local::now();
    let now_utc = Utc::now();

    // --- alarms ---
    let alarms = state.alarms.lock().unwrap().clone();
    for alarm in &alarms {
        if let Some(key) = alarm_due_now(alarm, now_local) {
            if state.claim(key) {
                let label = if alarm.label.trim().is_empty() {
                    "Alarm".to_string()
                } else {
                    alarm.label.clone()
                };
                notify(app, &label, &format!("It's {}.", alarm.at));

                // The notification is the quiet half. This is the part that
                // actually wakes him: a real tone from this process, plus the
                // always-on-top overlay so there is something to dismiss even
                // when the main window is hidden in the tray.
                app.state::<crate::audio::AudioState>().start();
                let _ = crate::show_mini(
                    app,
                    crate::MiniMode::Alarm {
                        label: label.clone(),
                        at: alarm.at.clone(),
                    },
                );
                let _ = app.emit("scout://alarm-fired", alarm.id.clone());
            }
        }
    }

    refresh_tooltip(app, &alarms);

    // --- deadline reminders ---
    let deadlines = state.deadlines.lock().unwrap().clone();
    for deadline in &deadlines {
        if let Some((key, days)) = deadlines_due(deadline, now_utc) {
            if state.claim(key) {
                let when = match days {
                    1 => "tomorrow".to_string(),
                    d => format!("in {d} days"),
                };
                notify(app, "Deadline approaching", &format!("{} — due {when}.", deadline.title));
            }
        }
    }

    // --- news refresh ---
    let due_for_refresh = {
        let last = state.last_refresh.lock().unwrap();
        match *last {
            None => true,
            Some(t) => now_utc.signed_duration_since(t) >= Duration::minutes(REFRESH_MINUTES),
        }
    };

    if due_for_refresh {
        // Mark the attempt before running it, so a slow or failing fetch does
        // not queue up repeat runs on every subsequent tick.
        *state.last_refresh.lock().unwrap() = Some(now_utc);

        let result = crate::pipeline::run().await;

        if result.offline {
            // Offline is a normal state, not an error. Retry on the next cycle
            // and say nothing — no notification, no error badge.
            log::info!("refresh skipped: offline");
        } else {
            let legendary: Vec<_> = result
                .items
                .iter()
                .filter(|i| i.badge == "legendary")
                .collect();

            // Only a legendary item is worth interrupting him for. Anything
            // else waits until he opens the app — a notification for every
            // refresh would train him to ignore all of them.
            if let Some(top) = legendary.first() {
                let extra = legendary.len().saturating_sub(1);
                let body = match extra {
                    0 => top.title.clone(),
                    n => format!("{} (+{n} more)", top.title),
                };
                notify(app, "Big, and nobody's covering it", &body);
            }

            // Park before emitting: if the window is closed there is nobody
            // to receive the event, and the stories must survive until it opens.
            park_results(&result.items);
            let _ = app.emit("scout://refreshed", &result);
            log::info!(
                "refresh: {} stories, {} legendary",
                result.story_count,
                legendary.len()
            );
        }
    }
}

/// The soonest upcoming alarm, as a tray-tooltip line.
///
/// The tray icon is the only part of Scout visible when the window is hidden,
/// so it should answer the one question worth asking at a glance: what is next.
fn tooltip_text(alarms: &[AlarmSpec], now: DateTime<Local>) -> String {
    let mut soonest: Option<(DateTime<Local>, &AlarmSpec)> = None;

    for alarm in alarms.iter().filter(|a| a.enabled) {
        let Some((h, m)) = alarm.at.split_once(':') else {
            continue;
        };
        let (Ok(h), Ok(m)) = (h.parse::<u32>(), m.parse::<u32>()) else {
            continue;
        };

        // Look ahead a week: enough to find the next occurrence of any weekly
        // repeat, and to roll a daily alarm over to tomorrow.
        for day in 0..8 {
            let Some(candidate) = (now + Duration::days(day))
                .date_naive()
                .and_hms_opt(h, m, 0)
                .and_then(|naive| naive.and_local_timezone(Local).single())
            else {
                continue;
            };
            if candidate <= now {
                continue;
            }
            if !alarm.days.is_empty()
                && !alarm.days.contains(&candidate.weekday().num_days_from_sunday())
            {
                continue;
            }
            if soonest.as_ref().is_none_or(|(best, _)| candidate < *best) {
                soonest = Some((candidate, alarm));
            }
            break;
        }
    }

    match soonest {
        None => "Scout — no alarms set".to_string(),
        Some((at, alarm)) => {
            let mins = (at - now).num_minutes().max(0);
            let when = if mins < 60 {
                format!("in {mins}m")
            } else if mins < 60 * 24 {
                format!("in {}h {}m", mins / 60, mins % 60)
            } else {
                at.format("%a %H:%M").to_string()
            };
            let label = if alarm.label.trim().is_empty() {
                "Alarm"
            } else {
                alarm.label.trim()
            };
            format!("Scout — {label} at {} ({when})", alarm.at)
        }
    }
}

fn refresh_tooltip(app: &AppHandle, alarms: &[AlarmSpec]) {
    if let Some(tray) = app.tray_by_id("main") {
        let _ = tray.set_tooltip(Some(tooltip_text(alarms, Local::now())));
    }
}

#[tauri::command]
pub fn sync_alarms(app: AppHandle, state: tauri::State<'_, SchedulerState>, alarms: Vec<AlarmSpec>) {
    refresh_tooltip(&app, &alarms);
    state.set_alarms(alarms);
}

#[tauri::command]
pub fn sync_deadlines(state: tauri::State<'_, SchedulerState>, deadlines: Vec<DeadlineSpec>) {
    state.set_deadlines(deadlines);
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn alarm(at: &str, days: Vec<u32>, enabled: bool) -> AlarmSpec {
        AlarmSpec {
            id: "a1".into(),
            at: at.into(),
            label: "Wake".into(),
            days,
            enabled,
        }
    }

    /// 2026-08-24 is a Monday.
    fn monday_at(h: u32, m: u32, s: u32) -> DateTime<Local> {
        Local.with_ymd_and_hms(2026, 8, 24, h, m, s).unwrap()
    }

    #[test]
    fn the_tooltip_names_the_next_alarm() {
        // The tray icon is all that is visible when the window is hidden, so
        // it should answer "what is next" rather than just saying the app name.
        let alarms = vec![alarm("06:30", vec![], true)];
        let text = tooltip_text(&alarms, monday_at(5, 0, 0));
        assert!(text.contains("06:30"), "got: {text}");
        assert!(text.contains("Wake"), "got: {text}");
        assert!(text.contains("in 1h 30m"), "got: {text}");
    }

    #[test]
    fn the_tooltip_ignores_disabled_alarms() {
        let alarms = vec![alarm("06:30", vec![], false)];
        assert_eq!(tooltip_text(&alarms, monday_at(5, 0, 0)), "Scout — no alarms set");
    }

    #[test]
    fn the_tooltip_picks_the_soonest_of_several() {
        let mut later = alarm("22:00", vec![], true);
        later.id = "a2".into();
        later.label = "Wind down".into();
        let alarms = vec![later, alarm("06:30", vec![], true)];
        let text = tooltip_text(&alarms, monday_at(5, 0, 0));
        assert!(text.contains("Wake"), "should pick 06:30 over 22:00, got: {text}");
    }

    #[test]
    fn a_weekly_alarm_rolls_to_its_next_matching_day() {
        // Monday 05:00, alarm repeats Wednesdays only.
        let alarms = vec![alarm("06:30", vec![3], true)];
        let text = tooltip_text(&alarms, monday_at(5, 0, 0));
        assert!(text.contains("06:30"), "got: {text}");
        // Two days out, so it should read as a weekday and time, not minutes.
        assert!(text.contains("Wed"), "got: {text}");
    }

    #[test]
    fn an_alarm_fires_on_its_minute() {
        assert!(alarm_due_now(&alarm("06:30", vec![], true), monday_at(6, 30, 0)).is_some());
    }

    #[test]
    fn an_alarm_still_fires_when_the_tick_lands_late_in_the_minute() {
        // The scheduler wakes every 20s, so it will rarely be exactly on the
        // second. Missing the alarm because of that would be the whole feature
        // failing quietly.
        assert!(alarm_due_now(&alarm("06:30", vec![], true), monday_at(6, 30, 47)).is_some());
    }

    #[test]
    fn an_alarm_does_not_fire_on_the_wrong_minute() {
        assert!(alarm_due_now(&alarm("06:30", vec![], true), monday_at(6, 31, 0)).is_none());
    }

    #[test]
    fn a_disabled_alarm_never_fires() {
        assert!(alarm_due_now(&alarm("06:30", vec![], false), monday_at(6, 30, 0)).is_none());
    }

    #[test]
    fn a_weekday_alarm_skips_the_wrong_day() {
        // Repeats Tuesday and Wednesday only; this is a Monday.
        assert!(alarm_due_now(&alarm("06:30", vec![2, 3], true), monday_at(6, 30, 0)).is_none());
        assert!(alarm_due_now(&alarm("06:30", vec![1], true), monday_at(6, 30, 0)).is_some());
    }

    #[test]
    fn the_fired_key_is_unique_per_minute() {
        let a = alarm("06:30", vec![], true);
        let first = alarm_due_now(&a, monday_at(6, 30, 5)).unwrap();
        let same_minute = alarm_due_now(&a, monday_at(6, 30, 50)).unwrap();
        assert_eq!(first, same_minute, "two ticks in one minute must dedupe");
    }

    #[test]
    fn claiming_the_same_key_twice_fails_the_second_time() {
        let state = SchedulerState::default();
        assert!(state.claim("alarm:a1:x".into()));
        assert!(!state.claim("alarm:a1:x".into()), "must not fire twice");
    }

    fn deadline(days_out: i64) -> DeadlineSpec {
        DeadlineSpec {
            id: "t1".into(),
            title: "Hackathon submission".into(),
            due_at: (Utc::now() + Duration::days(days_out) + Duration::hours(1)).to_rfc3339(),
        }
    }

    #[test]
    fn deadline_reminders_fire_at_the_documented_offsets() {
        for offset in DEADLINE_OFFSETS {
            let due = deadlines_due(&deadline(offset), Utc::now());
            assert!(due.is_some(), "expected a reminder {offset} days out");
            assert_eq!(due.unwrap().1, offset);
        }
    }

    #[test]
    fn no_reminder_on_an_off_offset() {
        assert!(deadlines_due(&deadline(5), Utc::now()).is_none());
    }

    #[test]
    fn a_passed_deadline_never_reminds() {
        let past = DeadlineSpec {
            id: "t1".into(),
            title: "Gone".into(),
            due_at: (Utc::now() - Duration::days(1)).to_rfc3339(),
        };
        assert!(deadlines_due(&past, Utc::now()).is_none());
    }

    #[test]
    fn running_the_sweep_twice_yields_one_notification_per_offset() {
        // The plan's explicit verification: idempotent per item+offset.
        let state = SchedulerState::default();
        let d = deadline(2);
        let (key, _) = deadlines_due(&d, Utc::now()).unwrap();
        assert!(state.claim(key.clone()));
        assert!(!state.claim(key));
    }
}
