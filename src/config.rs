// Taskim Configuration
// Edit this file to customize your keybindings

use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::style::Color;
use serde::Deserialize;
use serde_yaml::Value;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

// --- YAML config file struct ---
#[derive(Debug, Clone, Deserialize)]
pub struct ConfigFile {
    pub show_keybinds: Option<bool>,
    pub show_preview: Option<bool>,
    pub open_in_editor: Option<bool>,
    pub editor_path: Option<String>,
    #[serde(rename = "file-mode")]
    pub file_mode: Option<FileModeConfigFile>,
    pub colors: Option<HashMap<String, String>>,
    pub task_edit_colors: Option<HashMap<String, String>>,
    pub keybindings: Option<HashMap<String, serde_yaml::Value>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FileModeConfigFile {
    pub enabled: Option<bool>,
    pub path: Option<String>,
    pub years: Option<i32>,
}

// --- Runtime keybinding struct ---
#[derive(Debug, Clone, PartialEq)]
pub struct KeyBinding {
    pub key: KeyCode,
    pub modifiers: KeyModifiers,
    pub description: String,
    pub color: Color,
}

impl KeyBinding {
    pub fn matches(&self, key: KeyCode, modifiers: KeyModifiers) -> bool {
        self.key == key && self.modifiers == modifiers
    }
}

// --- Runtime config struct ---
#[derive(Debug, Clone)]
pub struct UiColors {
    pub default_fg: Color,
    pub default_bg: Color,
    pub default_task_fg: Color,
    pub day_number_fg: Color,
    pub selected_task_fg: Color,
    pub selected_task_bg: Color,
    pub completed_task_fg: Color,
    pub selected_completed_task_bg: Color,
    pub selected_completed_task_fg: Color,
    pub selected_task_bold: bool,
    // Add more fields as needed
}

#[derive(Debug, Clone)]
pub struct TaskEditColors {
    pub popup_bg: Color,
    pub popup_fg: Color,
    pub border_fg: Color,
    pub border_selected_fg: Color,
    pub title_fg: Color,
    pub title_selected_fg: Color,
    pub content_fg: Color,
    pub content_selected_fg: Color,
    pub instructions_fg: Color,
    pub instructions_key_fg: Color,
}

#[derive(Debug, Clone)]
pub struct FileModeConfig {
    pub enabled: bool,
    pub path: String,
    pub years: i32,
}

#[derive(Debug, Clone)]
pub struct Config {
    // Navigation
    pub move_left: KeyBinding,
    pub move_down: KeyBinding,
    pub move_up: KeyBinding,
    pub move_right: KeyBinding,
    // Task operations
    pub insert_edit: KeyBinding,
    pub insert_above: KeyBinding,
    pub insert_below: KeyBinding,
    pub delete: KeyBinding,
    pub delete_line: KeyBinding,
    pub toggle_complete: KeyBinding,
    pub yank: KeyBinding,
    pub paste: KeyBinding,
    pub paste_above: KeyBinding,
    // Undo/Redo
    pub undo: KeyBinding,
    pub redo: KeyBinding,
    // Month/Year navigation
    pub next_month: KeyBinding,
    pub prev_month: KeyBinding,
    pub next_year: KeyBinding,
    pub prev_year: KeyBinding,
    // Week navigation
    pub next_week: KeyBinding,
    pub prev_week: KeyBinding,
    // Day navigation
    pub first_day_of_month: KeyBinding,
    pub last_day_of_month: KeyBinding,
    // Go to today
    pub go_to_today: KeyBinding,
    // Task editing
    pub save_task: KeyBinding,
    pub cancel_edit: KeyBinding,
    pub switch_field: KeyBinding,
    pub backspace: KeyBinding,
    // App control
    pub quit: KeyBinding,
    pub quit_alt: KeyBinding,
    pub force_quit: KeyBinding,
    // New config fields
    pub show_keybinds: bool,
    pub show_preview: bool,
    pub open_in_editor: bool,
    pub editor_path: Option<String>,
    pub file_mode: FileModeConfig,
    pub ui_colors: UiColors,
    pub task_edit_colors: TaskEditColors,
}

impl Config {
    pub fn from_file_or_default<P: AsRef<Path>>(path: P) -> Self {
        let file = ConfigFile::load_from_yaml(&path);
        Self::from_config_file(file)
    }
    pub fn from_config_file(file: Option<ConfigFile>) -> Self {
        let show_keybinds = file.as_ref().and_then(|f| f.show_keybinds).unwrap_or(true);
        let show_preview = file.as_ref().and_then(|f| f.show_preview).unwrap_or(false);
        let open_in_editor = file
            .as_ref()
            .and_then(|f| f.open_in_editor)
            .unwrap_or(false);
        let editor_path = file.as_ref().and_then(|f| f.editor_path.clone());
        let file_mode_config = file.as_ref().and_then(|f| f.file_mode.clone());
        let file_mode = FileModeConfig {
            enabled: file_mode_config
                .as_ref()
                .and_then(|f| f.enabled)
                .unwrap_or(false),
            path: file_mode_config
                .as_ref()
                .and_then(|f| f.path.clone())
                .unwrap_or_else(|| "tasks".to_string()),
            years: file_mode_config
                .as_ref()
                .and_then(|f| f.years)
                .unwrap_or(1)
                .max(0),
        };
        let colors = file.as_ref().and_then(|f| f.colors.as_ref()).cloned();
        let task_edit_colors_map = file
            .as_ref()
            .and_then(|f| f.task_edit_colors.as_ref())
            .cloned();
        let keybindings_yaml = file.as_ref().and_then(|f| f.keybindings.as_ref());
        // Build default keybindings as a HashMap
        let default_keybindings = default_keybindings();
        let mut keybindings_map = std::collections::HashMap::new();
        for (name, default) in default_keybindings.iter() {
            let yaml_val = keybindings_yaml.and_then(|m| m.get(*name));
            keybindings_map.insert(*name, parse_keybinding(yaml_val, default));
        }
        let ui_colors = UiColors {
            default_fg: parse_color(&colors, "default_fg", Color::White),
            default_bg: parse_color(&colors, "default_bg", Color::Black),
            default_task_fg: parse_color(&colors, "default_task_fg", Color::White),
            day_number_fg: parse_color(&colors, "day_number_fg", Color::White),
            selected_task_fg: parse_color(&colors, "selected_task_fg", Color::Black),
            selected_task_bg: parse_color(&colors, "selected_task_bg", Color::Gray),
            completed_task_fg: parse_color(&colors, "completed_task_fg", Color::Green),
            selected_completed_task_bg: parse_color(
                &colors,
                "selected_completed_task_bg",
                Color::DarkGray,
            ),
            selected_completed_task_fg: parse_color(
                &colors,
                "selected_completed_task_fg",
                Color::Green,
            ),
            selected_task_bold: parse_bool(&(&colors), "selected_task_bold", true),
        };
        let task_edit_colors = TaskEditColors {
            popup_bg: parse_color(&task_edit_colors_map, "popup_bg", Color::Black),
            popup_fg: parse_color(&task_edit_colors_map, "popup_fg", Color::White),
            border_fg: parse_color(&task_edit_colors_map, "border_fg", Color::White),
            border_selected_fg: parse_color(
                &task_edit_colors_map,
                "border_selected_fg",
                Color::Blue,
            ),
            title_fg: parse_color(&task_edit_colors_map, "title_fg", Color::White),
            title_selected_fg: parse_color(&task_edit_colors_map, "title_selected_fg", Color::Blue),
            content_fg: parse_color(&task_edit_colors_map, "content_fg", Color::White),
            content_selected_fg: parse_color(
                &task_edit_colors_map,
                "content_selected_fg",
                Color::Blue,
            ),
            instructions_fg: parse_color(&task_edit_colors_map, "instructions_fg", Color::Gray),
            instructions_key_fg: parse_color(
                &task_edit_colors_map,
                "instructions_key_fg",
                Color::Blue,
            ),
        };
        Config {
            move_left: keybindings_map["move_left"].clone(),
            move_down: keybindings_map["move_down"].clone(),
            move_up: keybindings_map["move_up"].clone(),
            move_right: keybindings_map["move_right"].clone(),
            insert_edit: keybindings_map["insert_edit"].clone(),
            insert_above: keybindings_map["insert_above"].clone(),
            insert_below: keybindings_map["insert_below"].clone(),
            delete: keybindings_map["delete"].clone(),
            delete_line: keybindings_map["delete_line"].clone(),
            toggle_complete: keybindings_map["toggle_complete"].clone(),
            yank: keybindings_map["yank"].clone(),
            paste: keybindings_map["paste"].clone(),
            paste_above: keybindings_map["paste_above"].clone(),
            undo: keybindings_map["undo"].clone(),
            redo: keybindings_map["redo"].clone(),
            next_month: keybindings_map["next_month"].clone(),
            prev_month: keybindings_map["prev_month"].clone(),
            next_year: keybindings_map["next_year"].clone(),
            prev_year: keybindings_map["prev_year"].clone(),
            next_week: keybindings_map["next_week"].clone(),
            prev_week: keybindings_map["prev_week"].clone(),
            first_day_of_month: keybindings_map["first_day_of_month"].clone(),
            last_day_of_month: keybindings_map["last_day_of_month"].clone(),
            go_to_today: keybindings_map["go_to_today"].clone(),
            save_task: keybindings_map["save_task"].clone(),
            cancel_edit: keybindings_map["cancel_edit"].clone(),
            switch_field: keybindings_map["switch_field"].clone(),
            backspace: keybindings_map["backspace"].clone(),
            quit: keybindings_map["quit"].clone(),
            quit_alt: keybindings_map["quit_alt"].clone(),
            force_quit: keybindings_map["force_quit"].clone(),
            show_keybinds,
            show_preview,
            open_in_editor,
            editor_path,
            file_mode,
            ui_colors,
            task_edit_colors,
        }
    }
}

// Helper functions for UI
impl Config {
    pub fn get_normal_mode_help_spans(
        &self,
        can_undo: bool,
        can_redo: bool,
    ) -> Vec<ratatui::text::Span<'static>> {
        use ratatui::{style::Style, text::Span};

        let mut spans = Vec::new();

        // Movement keys (show as combined)
        spans.push(Span::styled("hjkl", Style::default().fg(Color::Green)));
        spans.push(Span::raw(": Move | "));

        // Task operations
        spans.push(Span::styled(
            "i",
            Style::default().fg(self.insert_edit.color),
        ));
        spans.push(Span::raw(": Insert/Edit | "));
        spans.push(Span::styled("x", Style::default().fg(self.delete.color)));
        spans.push(Span::raw(": Delete | "));
        spans.push(Span::styled(
            "c",
            Style::default().fg(self.toggle_complete.color),
        ));
        spans.push(Span::raw(": Toggle Complete | "));

        // Yank/Paste
        spans.push(Span::styled("y", Style::default().fg(self.yank.color)));
        spans.push(Span::raw(": Yank | "));
        spans.push(Span::styled("p", Style::default().fg(self.paste.color)));
        spans.push(Span::raw(": Paste | "));
        spans.push(Span::styled(
            "P",
            Style::default().fg(self.paste_above.color),
        ));
        spans.push(Span::raw(": Paste Above | "));

        // Undo/Redo (only show if available)
        if can_undo {
            spans.push(Span::styled("u", Style::default().fg(self.undo.color)));
            spans.push(Span::raw(": Undo | "));
        }
        if can_redo {
            spans.push(Span::styled("Ctrl+r", Style::default().fg(self.redo.color)));
            spans.push(Span::raw(": Redo | "));
        }

        // Month/Year navigation (vim-style)
        spans.push(Span::styled("H/L", Style::default().fg(Color::Cyan)));
        spans.push(Span::raw(": Month | "));
        spans.push(Span::styled("gg/G", Style::default().fg(Color::Cyan)));
        spans.push(Span::raw(": Year | "));

        // Week navigation
        spans.push(Span::styled(
            "w/b",
            Style::default().fg(self.next_week.color),
        ));
        spans.push(Span::raw(": Week | "));

        // Day navigation
        spans.push(Span::styled(
            "0/$",
            Style::default().fg(self.first_day_of_month.color),
        ));
        spans.push(Span::raw(": Day | "));

        // Search helpers
        spans.push(Span::styled("/", Style::default().fg(Color::Yellow)));
        spans.push(Span::raw(": Search | "));
        spans.push(Span::styled("n/N", Style::default().fg(Color::Yellow)));
        spans.push(Span::raw(": Next/Prev match | "));

        // Quit
        spans.push(Span::styled("q", Style::default().fg(self.quit.color)));
        spans.push(Span::raw(": Quit"));

        spans
    }

    pub fn get_edit_mode_help_spans(&self) -> Vec<ratatui::text::Span<'static>> {
        use ratatui::{style::Style, text::Span};

        vec![
            Span::styled("Tab", Style::default().fg(self.switch_field.color)),
            Span::raw(": Switch field | "),
            Span::styled("Enter", Style::default().fg(self.save_task.color)),
            Span::raw(": Save | "),
            Span::styled("Esc", Style::default().fg(self.cancel_edit.color)),
            Span::raw(": Cancel"),
        ]
    }
}

impl ConfigFile {
    pub fn load_from_yaml<P: AsRef<Path>>(path: P) -> Option<Self> {
        let content = fs::read_to_string(path).ok()?;
        serde_yaml::from_str(&content).ok()
    }
}

pub fn save_bool_preference<P: AsRef<Path>>(path: P, key: &str, value: bool) -> Result<(), String> {
    let path = path.as_ref();
    let mut lines = match fs::read_to_string(path) {
        Ok(content) => content.lines().map(str::to_string).collect::<Vec<_>>(),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(err) => return Err(format!("Failed to read config: {}", err)),
    };

    let new_line = format!("{}: {}", key, value);
    let prefix = format!("{}:", key);

    if let Some(index) = lines
        .iter()
        .position(|line| line.trim_start().starts_with(&prefix))
    {
        lines[index] = new_line;
    } else if let Some(index) = lines
        .iter()
        .position(|line| line.trim_start().starts_with("show_keybinds:"))
    {
        lines.insert(index + 1, new_line);
    } else {
        if !lines.is_empty() && !lines.last().is_some_and(|line| line.is_empty()) {
            lines.push(String::new());
        }
        lines.push(new_line);
    }

    let output = if lines.is_empty() {
        String::new()
    } else {
        format!("{}\n", lines.join("\n"))
    };

    fs::write(path, output).map_err(|err| format!("Failed to write config: {}", err))
}

fn parse_color(map: &Option<HashMap<String, String>>, key: &str, default: Color) -> Color {
    map.as_ref()
        .and_then(|m| m.get(key))
        .map(|s| parse_color_name(s))
        .unwrap_or(default)
}

fn parse_color_name(name: &str) -> Color {
    // Try to parse as integer for indexed color
    if let Ok(idx) = name.parse::<u8>() {
        return Color::Indexed(idx);
    }
    match name.to_lowercase().as_str() {
        "black" => Color::Black,
        "red" => Color::Red,
        "green" => Color::Green,
        "yellow" => Color::Yellow,
        "blue" => Color::Blue,
        "magenta" => Color::Magenta,
        "cyan" => Color::Cyan,
        "gray" => Color::Gray,
        "darkgray" => Color::DarkGray,
        "white" => Color::White,
        _ => Color::White,
    }
}

fn parse_bool(map: &&Option<HashMap<String, String>>, key: &str, default: bool) -> bool {
    map.as_ref()
        .and_then(|m| m.get(key))
        .and_then(|s| s.parse::<bool>().ok())
        .unwrap_or(default)
}

fn parse_keybinding(yaml: Option<&Value>, default: &KeyBinding) -> KeyBinding {
    match yaml {
        Some(Value::String(s)) => {
            // Only key is overridden
            KeyBinding {
                key: parse_key_code(s),
                ..default.clone()
            }
        }
        Some(Value::Sequence(seq)) => {
            let key = seq
                .get(0)
                .and_then(|v| v.as_str())
                .map(parse_key_code)
                .unwrap_or(default.key);
            let modifiers = seq
                .get(1)
                .and_then(|v| v.as_str())
                .map(parse_modifiers)
                .unwrap_or(default.modifiers);
            let description = seq
                .get(2)
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| default.description.clone());
            let color = seq
                .get(3)
                .and_then(|v| v.as_str())
                .map(|c| parse_color_name(c))
                .unwrap_or(default.color);
            KeyBinding {
                key,
                modifiers,
                description,
                color,
            }
        }
        _ => default.clone(),
    }
}

fn parse_key_code(s: &str) -> KeyCode {
    match s.to_lowercase().as_str() {
        "enter" => KeyCode::Enter,
        "esc" | "escape" => KeyCode::Esc,
        "tab" => KeyCode::Tab,
        "backspace" => KeyCode::Backspace,
        "$" => KeyCode::Char('$'),
        _ if s.len() == 1 => KeyCode::Char(s.chars().next().unwrap()),
        _ => KeyCode::Null,
    }
}

fn parse_modifiers(s: &str) -> KeyModifiers {
    match s.to_uppercase().as_str() {
        "SHIFT" => KeyModifiers::SHIFT,
        "CTRL" | "CONTROL" => KeyModifiers::CONTROL,
        "ALT" => KeyModifiers::ALT,
        _ => KeyModifiers::NONE,
    }
}

fn default_keybindings() -> std::collections::HashMap<&'static str, KeyBinding> {
    let mut map = std::collections::HashMap::new();
    map.insert(
        "move_left",
        KeyBinding {
            key: KeyCode::Char('h'),
            modifiers: KeyModifiers::NONE,
            description: String::from("Move"),
            color: Color::Green,
        },
    );
    map.insert(
        "move_down",
        KeyBinding {
            key: KeyCode::Char('j'),
            modifiers: KeyModifiers::NONE,
            description: String::from("Move"),
            color: Color::Green,
        },
    );
    map.insert(
        "move_up",
        KeyBinding {
            key: KeyCode::Char('k'),
            modifiers: KeyModifiers::NONE,
            description: String::from("Move"),
            color: Color::Green,
        },
    );
    map.insert(
        "move_right",
        KeyBinding {
            key: KeyCode::Char('l'),
            modifiers: KeyModifiers::NONE,
            description: String::from("Move"),
            color: Color::Green,
        },
    );
    map.insert(
        "insert_edit",
        KeyBinding {
            key: KeyCode::Char('i'),
            modifiers: KeyModifiers::NONE,
            description: String::from("Insert/Edit"),
            color: Color::Green,
        },
    );
    map.insert(
        "insert_above",
        KeyBinding {
            key: KeyCode::Char('O'),
            modifiers: KeyModifiers::SHIFT,
            description: String::from("Insert Above"),
            color: Color::Green,
        },
    );
    map.insert(
        "insert_below",
        KeyBinding {
            key: KeyCode::Char('o'),
            modifiers: KeyModifiers::NONE,
            description: String::from("Insert Below"),
            color: Color::Green,
        },
    );
    map.insert(
        "delete",
        KeyBinding {
            key: KeyCode::Char('x'),
            modifiers: KeyModifiers::NONE,
            description: String::from("Delete"),
            color: Color::Red,
        },
    );
    map.insert(
        "delete_line",
        KeyBinding {
            key: KeyCode::Char('d'),
            modifiers: KeyModifiers::NONE,
            description: String::from("Cut Task (dd)"),
            color: Color::Red,
        },
    );
    map.insert(
        "toggle_complete",
        KeyBinding {
            key: KeyCode::Char('c'),
            modifiers: KeyModifiers::NONE,
            description: String::from("Toggle Complete"),
            color: Color::Blue,
        },
    );
    map.insert(
        "yank",
        KeyBinding {
            key: KeyCode::Char('y'),
            modifiers: KeyModifiers::NONE,
            description: String::from("Yank (Copy)"),
            color: Color::Yellow,
        },
    );
    map.insert(
        "paste",
        KeyBinding {
            key: KeyCode::Char('p'),
            modifiers: KeyModifiers::NONE,
            description: String::from("Paste"),
            color: Color::Yellow,
        },
    );
    map.insert(
        "paste_above",
        KeyBinding {
            key: KeyCode::Char('P'),
            modifiers: KeyModifiers::SHIFT,
            description: String::from("Paste Above"),
            color: Color::Yellow,
        },
    );
    map.insert(
        "undo",
        KeyBinding {
            key: KeyCode::Char('u'),
            modifiers: KeyModifiers::NONE,
            description: String::from("Undo"),
            color: Color::Magenta,
        },
    );
    map.insert(
        "redo",
        KeyBinding {
            key: KeyCode::Char('r'),
            modifiers: KeyModifiers::CONTROL,
            description: String::from("Redo"),
            color: Color::Magenta,
        },
    );
    map.insert(
        "next_month",
        KeyBinding {
            key: KeyCode::Char('L'),
            modifiers: KeyModifiers::SHIFT,
            description: String::from("Next Month"),
            color: Color::Cyan,
        },
    );
    map.insert(
        "prev_month",
        KeyBinding {
            key: KeyCode::Char('H'),
            modifiers: KeyModifiers::SHIFT,
            description: String::from("Prev Month"),
            color: Color::Cyan,
        },
    );
    map.insert(
        "next_year",
        KeyBinding {
            key: KeyCode::Char('G'),
            modifiers: KeyModifiers::SHIFT,
            description: String::from("Last Year"),
            color: Color::Cyan,
        },
    );
    map.insert(
        "prev_year",
        KeyBinding {
            key: KeyCode::Char('g'),
            modifiers: KeyModifiers::NONE,
            description: String::from("First Year (gg)"),
            color: Color::Cyan,
        },
    );
    map.insert(
        "next_week",
        KeyBinding {
            key: KeyCode::Char('w'),
            modifiers: KeyModifiers::NONE,
            description: String::from("Next Week"),
            color: Color::Blue,
        },
    );
    map.insert(
        "prev_week",
        KeyBinding {
            key: KeyCode::Char('b'),
            modifiers: KeyModifiers::NONE,
            description: String::from("Previous Week"),
            color: Color::Blue,
        },
    );
    map.insert(
        "first_day_of_month",
        KeyBinding {
            key: KeyCode::Char('0'),
            modifiers: KeyModifiers::NONE,
            description: String::from("First Day"),
            color: Color::Blue,
        },
    );
    map.insert(
        "last_day_of_month",
        KeyBinding {
            key: KeyCode::Char('$'),
            modifiers: KeyModifiers::SHIFT,
            description: String::from("Last Day"),
            color: Color::Blue,
        },
    );
    map.insert(
        "go_to_today",
        KeyBinding {
            key: KeyCode::Char('t'),
            modifiers: KeyModifiers::NONE,
            description: String::from("Go to Today"),
            color: Color::Magenta,
        },
    );
    map.insert(
        "save_task",
        KeyBinding {
            key: KeyCode::Enter,
            modifiers: KeyModifiers::NONE,
            description: String::from("Save"),
            color: Color::Green,
        },
    );
    map.insert(
        "cancel_edit",
        KeyBinding {
            key: KeyCode::Esc,
            modifiers: KeyModifiers::NONE,
            description: String::from("Cancel"),
            color: Color::Red,
        },
    );
    map.insert(
        "switch_field",
        KeyBinding {
            key: KeyCode::Tab,
            modifiers: KeyModifiers::NONE,
            description: String::from("Switch Field"),
            color: Color::Green,
        },
    );
    map.insert(
        "backspace",
        KeyBinding {
            key: KeyCode::Backspace,
            modifiers: KeyModifiers::NONE,
            description: String::from("Delete Char"),
            color: Color::Gray,
        },
    );
    map.insert(
        "quit",
        KeyBinding {
            key: KeyCode::Char('q'),
            modifiers: KeyModifiers::NONE,
            description: String::from("Quit"),
            color: Color::Red,
        },
    );
    map.insert(
        "quit_alt",
        KeyBinding {
            key: KeyCode::Esc,
            modifiers: KeyModifiers::NONE,
            description: String::from("Quit"),
            color: Color::Red,
        },
    );
    map.insert(
        "force_quit",
        KeyBinding {
            key: KeyCode::Char('c'),
            modifiers: KeyModifiers::CONTROL,
            description: String::from("Force Quit"),
            color: Color::Red,
        },
    );
    map
}
