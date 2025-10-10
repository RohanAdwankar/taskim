use std::collections::HashSet;

use chrono::{DateTime, Datelike, Duration, NaiveDate, TimeZone, Utc, Weekday};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::task::Task;

const DEFAULT_PREVIEW_LIMIT: usize = 12;
const MAX_SEARCH_DAYS: i32 = 365 * 10; // Safety guard to prevent infinite loops

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecurrenceSpawnMode {
    OnCompletion,
    AllAtOnce,
}

impl Default for RecurrenceSpawnMode {
    fn default() -> Self {
        RecurrenceSpawnMode::OnCompletion
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecurrenceDefinition {
    pub base_start: DateTime<Utc>,
    pub base_end: DateTime<Utc>,
    #[serde(default)]
    pub weekdays: Vec<Weekday>,
    #[serde(default)]
    pub month_days: Vec<u32>,
    #[serde(default)]
    pub months: Vec<u32>,
}

impl RecurrenceDefinition {
    pub fn from_task(task: &Task) -> Self {
        Self {
            base_start: task.start,
            base_end: task.end,
            weekdays: Vec::new(),
            month_days: Vec::new(),
            months: Vec::new(),
        }
    }

    pub fn duration(&self) -> Duration {
        self.base_end - self.base_start
    }

    fn matches_date(&self, date: NaiveDate) -> bool {
        if !self.weekdays.is_empty() && !self.weekdays.contains(&date.weekday()) {
            return false;
        }
        if !self.months.is_empty() && !self.months.contains(&date.month()) {
            return false;
        }
        if !self.month_days.is_empty() && !self.month_days.contains(&date.day()) {
            return false;
        }
        true
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecurrenceSeries {
    pub id: String,
    pub definition: RecurrenceDefinition,
    pub spawn_mode: RecurrenceSpawnMode,
    pub total_occurrences: Option<u32>,
    pub generated_occurrences: u32,
    pub active: bool,
    #[serde(default)]
    pub description: Option<String>,
}

impl RecurrenceSeries {
    pub fn new(
        definition: RecurrenceDefinition,
        spawn_mode: RecurrenceSpawnMode,
        total_occurrences: Option<u32>,
        description: Option<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            definition,
            spawn_mode,
            total_occurrences,
            generated_occurrences: 1, // The source task already exists
            active: true,
            description,
        }
    }

    pub fn occurrences_remaining(&self) -> Option<u32> {
        self.total_occurrences
            .map(|total| total.saturating_sub(self.generated_occurrences))
    }

    pub fn has_remaining(&self) -> bool {
        self.occurrences_remaining().map(|r| r > 0).unwrap_or(true)
    }

    pub fn next_occurrence_after(&self, after: DateTime<Utc>) -> Option<Occurrence> {
        let mut date = after.date_naive();
        for _ in 0..MAX_SEARCH_DAYS {
            date = match date.succ_opt() {
                Some(next) => next,
                None => break,
            };

            if self.definition.matches_date(date) {
                if let Some(occurrence) = self.build_occurrence(date) {
                    return Some(occurrence);
                }
            }
        }
        None
    }

    pub fn preview(&self, limit: usize) -> Vec<Occurrence> {
        let mut occurrences = Vec::new();
        let mut cursor = self.definition.base_start;
        let mut generated = self.generated_occurrences;

        while occurrences.len() < limit {
            if let Some(total) = self.total_occurrences {
                if generated >= total {
                    break;
                }
            }

            if let Some(next) = self.next_occurrence_after(cursor) {
                occurrences.push(next.clone());
                cursor = next.start;
                generated += 1;
            } else {
                break;
            }
        }

        occurrences
    }

    fn build_occurrence(&self, date: NaiveDate) -> Option<Occurrence> {
        let time = self.definition.base_start.time();
        let naive_start = date.and_time(time);
        let start = Utc.from_utc_datetime(&naive_start);
        let end = start + self.definition.duration();
        Some(Occurrence { start, end })
    }
}

#[derive(Debug, Clone)]
pub struct Occurrence {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct RecurrenceDraft {
    pub series: RecurrenceSeries,
    pub occurrences: Vec<Occurrence>,
    pub description: String,
}

#[derive(Debug, Clone)]
pub enum ParsedCommand {
    Draft(RecurrenceDraft),
    Clear,
}

#[derive(Debug, Clone)]
pub struct RecurrenceError {
    pub message: String,
}

impl RecurrenceError {
    fn new<S: Into<String>>(msg: S) -> Self {
        Self {
            message: msg.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecurrenceMotion {
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

impl RecurrenceMotion {
    pub fn from_char(ch: char) -> Option<Self> {
        match ch {
            'd' | 'D' => Some(RecurrenceMotion::Daily),
            'w' | 'W' => Some(RecurrenceMotion::Weekly),
            'm' | 'M' => Some(RecurrenceMotion::Monthly),
            'y' | 'Y' => Some(RecurrenceMotion::Yearly),
            _ => None,
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            RecurrenceMotion::Daily => "Daily",
            RecurrenceMotion::Weekly => "Weekly",
            RecurrenceMotion::Monthly => "Monthly",
            RecurrenceMotion::Yearly => "Yearly",
        }
    }
}

pub fn build_draft_from_motion(task: &Task, motion: RecurrenceMotion) -> RecurrenceDraft {
    let mut definition = RecurrenceDefinition::from_task(task);
    let mut description = motion.description().to_string();

    match motion {
        RecurrenceMotion::Daily => {
            // No extra filters needed
        }
        RecurrenceMotion::Weekly => {
            definition.weekdays.push(task.start.weekday());
            description = format!("Weekly on {}", weekday_name(task.start.weekday()));
        }
        RecurrenceMotion::Monthly => {
            definition.month_days.push(task.start.day());
            description = format!("Monthly on day {}", task.start.day());
        }
        RecurrenceMotion::Yearly => {
            definition.month_days.push(task.start.day());
            definition.months.push(task.start.month());
            description = format!(
                "Yearly on {} {}",
                month_name(task.start.month()),
                task.start.day()
            );
        }
    }

    let series = RecurrenceSeries::new(
        definition,
        RecurrenceSpawnMode::OnCompletion,
        None,
        Some(description.clone()),
    );

    let preview = series.preview(DEFAULT_PREVIEW_LIMIT);

    RecurrenceDraft {
        series,
        occurrences: preview,
        description,
    }
}

pub fn parse_command(command: &str, task: &Task) -> Result<ParsedCommand, RecurrenceError> {
    if !command.starts_with('r') {
        return Err(RecurrenceError::new("Unknown recurrence command"));
    }

    let body = command[1..].trim_start_matches('/');

    if body.is_empty() {
        return Err(RecurrenceError::new(
            "Usage: :r/<pattern>. Try :r/d, :r/w, :r/m, :r/y, or :r/clear",
        ));
    }

    if body.eq_ignore_ascii_case("clear") {
        return Ok(ParsedCommand::Clear);
    }

    if body.len() == 1 {
        if let Some(motion) = RecurrenceMotion::from_char(body.chars().next().unwrap()) {
            return Ok(ParsedCommand::Draft(build_draft_from_motion(task, motion)));
        }
    }

    let segments: Vec<&str> = body
        .split('/')
        .filter(|seg| !seg.trim().is_empty())
        .collect();

    if segments.is_empty() {
        return Err(RecurrenceError::new(
            "Invalid recurrence command. Expected at least one segment.",
        ));
    }

    let mut definition = RecurrenceDefinition::from_task(task);
    let mut spawn_mode = RecurrenceSpawnMode::OnCompletion;
    let mut total_occurrences: Option<u32> = None;
    let mut description_parts: Vec<String> = Vec::new();
    let mut months_set = false;
    let mut weekdays_set = false;
    let mut month_days_set = false;
    let mut count_set = false;

    for (index, segment) in segments.iter().enumerate() {
        let lower = segment.to_ascii_lowercase();

        if lower == "a" {
            spawn_mode = RecurrenceSpawnMode::AllAtOnce;
            continue;
        }

        if lower == "clear" {
            return Ok(ParsedCommand::Clear);
        }

        if lower
            .chars()
            .all(|c| matches!(c, 'm' | 't' | 'w' | 'r' | 'f' | 's' | 'u'))
        {
            if weekdays_set {
                return Err(RecurrenceError::new(
                    "Weekdays specified more than once in recurrence command.",
                ));
            }
            let weekdays = parse_weekday_sequence(&lower)?;
            definition.weekdays = weekdays.clone();
            weekdays_set = true;
            description_parts.push(format!(
                "Weekdays: {}",
                weekdays
                    .into_iter()
                    .map(weekday_name)
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
            continue;
        }

        if lower.chars().all(|c| c.is_ascii_digit() || c == ',') {
            let numbers = parse_numeric_list(&lower)?;

            if numbers.len() > 1 {
                if month_days_set {
                    return Err(RecurrenceError::new(
                        "Multiple day-of-month segments detected in recurrence command.",
                    ));
                }
                definition.month_days = numbers.clone();
                month_days_set = true;
                description_parts.push(format!(
                    "Days of month: {}",
                    numbers
                        .into_iter()
                        .map(|n| n.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            } else {
                let value = numbers[0];
                if !month_days_set && !weekdays_set && index == 0 {
                    definition.month_days = vec![value];
                    month_days_set = true;
                    description_parts.push(format!("Days of month: {}", value));
                } else if !count_set {
                    total_occurrences = Some(value);
                    count_set = true;
                    description_parts.push(format!("Occurrences: {}", value));
                } else if !month_days_set {
                    definition.month_days = vec![value];
                    month_days_set = true;
                    description_parts.push(format!("Days of month: {}", value));
                } else {
                    return Err(RecurrenceError::new(
                        "Multiple occurrence limit segments detected in recurrence command.",
                    ));
                }
            }
            continue;
        }

        if lower.chars().all(|c| c.is_ascii_alphabetic()) {
            // Allow shorthand months like jan,feb if needed later
            if !months_set {
                if let Some(parsed_months) = try_parse_months(&lower) {
                    definition.months = parsed_months.into_iter().map(|m| m as u32).collect();
                    months_set = true;
                    description_parts.push(format!(
                        "Months: {}",
                        definition
                            .months
                            .iter()
                            .map(|m| month_name(*m))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                    continue;
                }
            }
        }

        return Err(RecurrenceError::new(format!(
            "Unrecognized recurrence segment: {}",
            segment
        )));
    }

    if definition.weekdays.is_empty()
        && definition.month_days.is_empty()
        && definition.months.is_empty()
    {
        // Default to daily recurrence if no filters supplied
        description_parts.push("Daily".to_string());
    }

    let description = if description_parts.is_empty() {
        "Recurring task".to_string()
    } else {
        description_parts.join(" · ")
    };

    let series = RecurrenceSeries::new(
        definition,
        spawn_mode,
        total_occurrences,
        Some(description.clone()),
    );

    let limit = total_occurrences
        .map(|total| total.saturating_sub(series.generated_occurrences) as usize)
        .filter(|count| *count > 0)
        .unwrap_or(DEFAULT_PREVIEW_LIMIT);

    let preview = series.preview(limit);

    Ok(ParsedCommand::Draft(RecurrenceDraft {
        series,
        occurrences: preview,
        description,
    }))
}

fn parse_weekday_sequence(input: &str) -> Result<Vec<Weekday>, RecurrenceError> {
    let mut weekdays = Vec::new();
    let mut seen = HashSet::new();

    for ch in input.chars() {
        let weekday = match ch {
            'm' => Weekday::Mon,
            't' => Weekday::Tue,
            'w' => Weekday::Wed,
            'r' => Weekday::Thu,
            'f' => Weekday::Fri,
            's' => Weekday::Sat,
            'u' => Weekday::Sun,
            _ => {
                return Err(RecurrenceError::new(format!(
                    "Unsupported weekday code '{}' in recurrence command.",
                    ch
                )));
            }
        };

        if seen.insert(weekday) {
            weekdays.push(weekday);
        }
    }

    if weekdays.is_empty() {
        return Err(RecurrenceError::new(
            "Weekday sequence must contain at least one valid day (mtwrfsu).",
        ));
    }

    weekdays.sort_by_key(|w| w.number_from_monday());
    Ok(weekdays)
}

fn parse_numeric_list(input: &str) -> Result<Vec<u32>, RecurrenceError> {
    let mut values = Vec::new();

    for segment in input.split(',') {
        if segment.is_empty() {
            continue;
        }
        let value: u32 = segment.parse().map_err(|_| {
            RecurrenceError::new(format!(
                "Invalid numeric value '{}' in recurrence command.",
                segment
            ))
        })?;

        if value == 0 {
            return Err(RecurrenceError::new(
                "Numeric values in recurrence command must be greater than zero.",
            ));
        }

        values.push(value);
    }

    if values.is_empty() {
        return Err(RecurrenceError::new(
            "Failed to parse numeric segment in recurrence command.",
        ));
    }

    Ok(values)
}

fn try_parse_months(input: &str) -> Option<Vec<chrono::Month>> {
    let mut months = Vec::new();
    let mut buffer = String::new();

    for ch in input.chars() {
        buffer.push(ch);
        if buffer.len() >= 3 {
            if let Ok(month) = match buffer.as_str() {
                "jan" => Ok(chrono::Month::January),
                "feb" => Ok(chrono::Month::February),
                "mar" => Ok(chrono::Month::March),
                "apr" => Ok(chrono::Month::April),
                "may" => Ok(chrono::Month::May),
                "jun" => Ok(chrono::Month::June),
                "jul" => Ok(chrono::Month::July),
                "aug" => Ok(chrono::Month::August),
                "sep" => Ok(chrono::Month::September),
                "oct" => Ok(chrono::Month::October),
                "nov" => Ok(chrono::Month::November),
                "dec" => Ok(chrono::Month::December),
                _ => Err(()),
            } {
                months.push(month);
                buffer.clear();
            } else if buffer.len() > 5 {
                return None;
            }
        }
    }

    if months.is_empty() {
        None
    } else {
        Some(months)
    }
}

fn weekday_name(weekday: Weekday) -> &'static str {
    match weekday {
        Weekday::Mon => "Mon",
        Weekday::Tue => "Tue",
        Weekday::Wed => "Wed",
        Weekday::Thu => "Thu",
        Weekday::Fri => "Fri",
        Weekday::Sat => "Sat",
        Weekday::Sun => "Sun",
    }
}

fn month_name(month: u32) -> &'static str {
    match month {
        1 => "Jan",
        2 => "Feb",
        3 => "Mar",
        4 => "Apr",
        5 => "May",
        6 => "Jun",
        7 => "Jul",
        8 => "Aug",
        9 => "Sep",
        10 => "Oct",
        11 => "Nov",
        12 => "Dec",
        _ => "Month",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::Task;
    use chrono::{TimeZone, Utc};

    fn sample_task() -> Task {
        let start = Utc.with_ymd_and_hms(2024, 1, 1, 9, 0, 0).unwrap();
        Task::new("Sample".to_string(), start)
    }

    #[test]
    fn motion_builder_weekly_uses_weekday() {
        let task = sample_task();
        let draft = build_draft_from_motion(&task, RecurrenceMotion::Weekly);
        assert_eq!(draft.series.definition.weekdays.len(), 1);
        assert_eq!(draft.series.definition.weekdays[0], Weekday::Mon);
        assert!(draft.series.definition.month_days.is_empty());
    }

    #[test]
    fn parse_weekday_command() {
        let task = sample_task();
        let result = parse_command("r/mtwrf", &task).unwrap();
        match result {
            ParsedCommand::Draft(draft) => {
                assert_eq!(draft.series.definition.weekdays.len(), 5);
                assert!(draft.series.definition.month_days.is_empty());
                assert_eq!(draft.series.spawn_mode, RecurrenceSpawnMode::OnCompletion);
            }
            _ => panic!("Expected Draft"),
        }
    }

    #[test]
    fn parse_month_day_and_weekday_command() {
        let task = sample_task();
        let result = parse_command("r/1,15/mtwrf", &task).unwrap();
        match result {
            ParsedCommand::Draft(draft) => {
                assert_eq!(draft.series.definition.month_days, vec![1, 15]);
                assert_eq!(draft.series.definition.weekdays.len(), 5);
            }
            _ => panic!("Expected Draft"),
        }
    }

    #[test]
    fn parse_all_at_once_with_count() {
        let task = sample_task();
        let result = parse_command("r/su/30/a", &task).unwrap();
        match result {
            ParsedCommand::Draft(draft) => {
                assert_eq!(draft.series.spawn_mode, RecurrenceSpawnMode::AllAtOnce);
                assert_eq!(draft.series.total_occurrences, Some(30));
                assert_eq!(draft.series.definition.weekdays.len(), 2);
                assert_eq!(draft.occurrences.len(), 29);
            }
            _ => panic!("Expected Draft"),
        }
    }

    #[test]
    fn preview_generates_daily_occurrences() {
        let task = sample_task();
        let draft = build_draft_from_motion(&task, RecurrenceMotion::Daily);
        assert_eq!(draft.occurrences.len(), DEFAULT_PREVIEW_LIMIT);
        let first = &draft.occurrences[0];
        assert!(first.start.date_naive() > task.start.date_naive());
    }
}
