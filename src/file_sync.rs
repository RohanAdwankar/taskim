use crate::config::FileModeConfig;
use crate::task::{Task, TaskData};
use chrono::{Datelike, Local};
use color_eyre::Result;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

pub fn sync_from_files(data: &mut TaskData, config: &FileModeConfig) -> Result<bool> {
    if !config.enabled {
        return Ok(false);
    }

    let root = expand_home(&config.path);
    if !root.exists() {
        return Ok(false);
    }

    let mut changed = false;
    for path in markdown_files(&root)? {
        let content = fs::read_to_string(&path)?;
        let Some(task_id) = extract_task_id(&content) else {
            continue;
        };

        if let Some(task) = data.events.iter_mut().find(|task| task.id == task_id) {
            if apply_markdown_to_task(task, &content) {
                changed = true;
            }
        }
    }

    Ok(changed)
}

pub fn export_files(data: &TaskData, config: &FileModeConfig) -> Result<()> {
    if !config.enabled {
        return Ok(());
    }

    let root = expand_home(&config.path);
    fs::create_dir_all(&root)?;

    let current_year = Local::now().year();
    let oldest_year = current_year - config.years;
    let mut tasks: Vec<&Task> = data
        .events
        .iter()
        .filter(|task| {
            let year = task.start.date_naive().year();
            year >= oldest_year && year <= current_year
        })
        .collect();
    tasks.sort_by_key(|task| (task.start, task.order, task.id.clone()));

    let paths_by_id = expected_paths(&root, tasks, current_year);
    for task in &data.events {
        if let Some(path) = paths_by_id.get(&task.id) {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }

            let markdown = task_to_markdown(task);
            if fs::read_to_string(path).ok().as_deref() != Some(markdown.as_str()) {
                fs::write(path, markdown)?;
            }
        }
    }

    remove_stale_taskim_files(&root, &paths_by_id)?;
    Ok(())
}

fn expected_paths<'a>(
    root: &Path,
    tasks: Vec<&'a Task>,
    current_year: i32,
) -> HashMap<String, PathBuf> {
    let mut used_paths = HashSet::new();
    let mut paths_by_id = HashMap::new();

    for task in tasks {
        let date = task.start.date_naive();
        let week = format!("week-{:02}", date.iso_week().week());
        let mut dir = root.to_path_buf();
        if date.year() != current_year {
            dir = dir.join(date.year().to_string());
        }
        dir = dir.join(week);

        let base_name = sanitize_file_name(&task.title);
        let mut candidate = dir.join(format!("{}.md", base_name));
        let mut suffix = 2;
        while used_paths.contains(&candidate) {
            candidate = dir.join(format!("{}-{}.md", base_name, suffix));
            suffix += 1;
        }
        used_paths.insert(candidate.clone());
        paths_by_id.insert(task.id.clone(), candidate);
    }

    paths_by_id
}

fn remove_stale_taskim_files(root: &Path, expected_paths: &HashMap<String, PathBuf>) -> Result<()> {
    let expected: HashSet<PathBuf> = expected_paths.values().cloned().collect();
    for path in markdown_files(root)? {
        if expected.contains(&path) {
            continue;
        }

        let content = fs::read_to_string(&path)?;
        if extract_task_id(&content).is_some() {
            fs::remove_file(path)?;
        }
    }
    Ok(())
}

fn markdown_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    collect_markdown_files(root, &mut files)?;
    Ok(files)
}

fn collect_markdown_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    if !dir.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_markdown_files(&path, files)?;
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("md") {
            files.push(path);
        }
    }
    Ok(())
}

fn task_to_markdown(task: &Task) -> String {
    let content = task
        .comments
        .first()
        .map(|comment| comment.text.as_str())
        .unwrap_or("");
    format!(
        "<!-- taskim-id: {} -->\n# {}\n\n{}",
        task.id, task.title, content
    )
}

fn apply_markdown_to_task(task: &mut Task, markdown: &str) -> bool {
    let (title, content) = parse_markdown(markdown);
    if title.trim().is_empty() {
        return false;
    }

    let mut changed = false;
    if task.title != title {
        task.title = title;
        changed = true;
    }

    let existing_content = task
        .comments
        .first()
        .map(|comment| comment.text.as_str())
        .unwrap_or("");
    if existing_content != content {
        task.comments.clear();
        if !content.is_empty() {
            task.add_comment(content);
        }
        changed = true;
    }

    changed
}

fn parse_markdown(markdown: &str) -> (String, String) {
    let mut lines = markdown.lines();
    let mut title = String::new();
    let mut content_lines = Vec::new();

    for line in lines.by_ref() {
        if line.trim_start().starts_with("<!-- taskim-id:") {
            continue;
        }

        if let Some(rest) = line.strip_prefix("# ") {
            title = rest.trim().to_string();
            break;
        }

        if !line.trim().is_empty() {
            title = line.trim().to_string();
            break;
        }
    }

    content_lines.extend(lines.map(ToString::to_string));
    while content_lines
        .first()
        .map(|line| line.trim().is_empty())
        .unwrap_or(false)
    {
        content_lines.remove(0);
    }

    (title, content_lines.join("\n").trim_end().to_string())
}

fn extract_task_id(markdown: &str) -> Option<String> {
    for line in markdown.lines().take(5) {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("<!-- taskim-id:") {
            return rest
                .strip_suffix("-->")
                .map(|id| id.trim().to_string())
                .filter(|id| !id.is_empty());
        }
    }
    None
}

fn sanitize_file_name(title: &str) -> String {
    let mut sanitized = String::new();
    for ch in title.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == ' ' {
            sanitized.push(ch);
        } else {
            sanitized.push('-');
        }
    }

    let sanitized = sanitized.trim().trim_matches('-').trim();
    if sanitized.is_empty() {
        "untitled".to_string()
    } else {
        sanitized.to_string()
    }
}

fn expand_home(path: &str) -> PathBuf {
    if path == "~" {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home);
        }
    } else if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home).join(rest);
        }
    }
    PathBuf::from(path)
}
