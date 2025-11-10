use crate::task::Task;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

#[derive(Debug, Clone, PartialEq)]
pub enum ListSelection {
    Task(String),    // task id
    NoTasks,         // when there are no tasks
}

#[derive(Debug, Clone, PartialEq)]
pub enum FocusPanel {
    List,
    Detail,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DetailEditField {
    None,
    Title,
    Description,
}

#[derive(Debug, Clone)]
pub struct ListView {
    pub selection: ListSelection,
    pub show_detail: bool,       // whether to show task details on the right
    pub focus: FocusPanel,        // which panel has focus (list or detail)
    pub editing_field: DetailEditField, // which field is being edited in detail panel
    pub edit_buffer: String,      // buffer for editing
}

impl ListView {
    pub fn new() -> Self {
        Self {
            selection: ListSelection::NoTasks,
            show_detail: false,
            focus: FocusPanel::List,
            editing_field: DetailEditField::None,
            edit_buffer: String::new(),
        }
    }

    /// Get the currently selected task ID, if any
    pub fn get_selected_task_id(&self) -> Option<String> {
        match &self.selection {
            ListSelection::Task(id) => Some(id.clone()),
            ListSelection::NoTasks => None,
        }
    }

    /// Start editing a field in the detail panel
    pub fn start_editing_title(&mut self, current_title: &str) {
        self.editing_field = DetailEditField::Title;
        self.edit_buffer = current_title.to_string();
    }

    pub fn start_editing_description(&mut self, current_desc: &str) {
        self.editing_field = DetailEditField::Description;
        self.edit_buffer = current_desc.to_string();
    }

    /// Stop editing and clear buffer
    pub fn stop_editing(&mut self) {
        self.editing_field = DetailEditField::None;
        self.edit_buffer.clear();
    }

    /// Check if currently editing
    pub fn is_editing(&self) -> bool {
        self.editing_field != DetailEditField::None
    }

    /// Add character to edit buffer
    pub fn add_char(&mut self, ch: char) {
        self.edit_buffer.push(ch);
    }

    /// Remove character from edit buffer
    pub fn remove_char(&mut self) {
        self.edit_buffer.pop();
    }

    /// Move selection down to the next task
    pub fn move_down(&mut self, tasks: &[Task]) {
        if tasks.is_empty() {
            self.selection = ListSelection::NoTasks;
            return;
        }

        match &self.selection {
            ListSelection::Task(current_id) => {
                if let Some(current_idx) = tasks.iter().position(|t| &t.id == current_id) {
                    let next_idx = (current_idx + 1) % tasks.len();
                    self.selection = ListSelection::Task(tasks[next_idx].id.clone());
                }
            }
            ListSelection::NoTasks => {
                if !tasks.is_empty() {
                    self.selection = ListSelection::Task(tasks[0].id.clone());
                }
            }
        }
    }

    /// Move selection down by n tasks
    pub fn move_down_by(&mut self, tasks: &[Task], n: usize) {
        if tasks.is_empty() {
            self.selection = ListSelection::NoTasks;
            return;
        }

        match &self.selection {
            ListSelection::Task(current_id) => {
                if let Some(current_idx) = tasks.iter().position(|t| &t.id == current_id) {
                    let next_idx = (current_idx + n).min(tasks.len() - 1);
                    self.selection = ListSelection::Task(tasks[next_idx].id.clone());
                }
            }
            ListSelection::NoTasks => {
                if !tasks.is_empty() {
                    let idx = n.min(tasks.len() - 1);
                    self.selection = ListSelection::Task(tasks[idx].id.clone());
                }
            }
        }
    }

    /// Move selection up to the previous task
    pub fn move_up(&mut self, tasks: &[Task]) {
        if tasks.is_empty() {
            self.selection = ListSelection::NoTasks;
            return;
        }

        match &self.selection {
            ListSelection::Task(current_id) => {
                if let Some(current_idx) = tasks.iter().position(|t| &t.id == current_id) {
                    let prev_idx = if current_idx == 0 {
                        tasks.len() - 1
                    } else {
                        current_idx - 1
                    };
                    self.selection = ListSelection::Task(tasks[prev_idx].id.clone());
                }
            }
            ListSelection::NoTasks => {
                if !tasks.is_empty() {
                    self.selection = ListSelection::Task(tasks[0].id.clone());
                }
            }
        }
    }

    /// Move selection up by n tasks
    pub fn move_up_by(&mut self, tasks: &[Task], n: usize) {
        if tasks.is_empty() {
            self.selection = ListSelection::NoTasks;
            return;
        }

        match &self.selection {
            ListSelection::Task(current_id) => {
                if let Some(current_idx) = tasks.iter().position(|t| &t.id == current_id) {
                    let prev_idx = current_idx.saturating_sub(n);
                    self.selection = ListSelection::Task(tasks[prev_idx].id.clone());
                }
            }
            ListSelection::NoTasks => {
                if !tasks.is_empty() {
                    self.selection = ListSelection::Task(tasks[0].id.clone());
                }
            }
        }
    }

    /// Go to first task (gg in vim)
    pub fn go_to_first(&mut self, tasks: &[Task]) {
        if !tasks.is_empty() {
            self.selection = ListSelection::Task(tasks[0].id.clone());
        }
    }

    /// Go to last task (G in vim)
    pub fn go_to_last(&mut self, tasks: &[Task]) {
        if !tasks.is_empty() {
            self.selection = ListSelection::Task(tasks[tasks.len() - 1].id.clone());
        }
    }

    /// Initialize or update the selection based on available tasks
    pub fn update_selection(&mut self, tasks: &[Task]) {
        if tasks.is_empty() {
            self.selection = ListSelection::NoTasks;
            return;
        }

        // Check if current selection is still valid
        match &self.selection {
            ListSelection::Task(id) => {
                if !tasks.iter().any(|t| &t.id == id) {
                    // Current selection is invalid, select first task
                    self.selection = ListSelection::Task(tasks[0].id.clone());
                }
            }
            ListSelection::NoTasks => {
                // Select first task if available
                self.selection = ListSelection::Task(tasks[0].id.clone());
            }
        }
    }

    /// Toggle detail view (show/hide right panel)
    pub fn toggle_detail_view(&mut self) {
        self.show_detail = !self.show_detail;
    }
}

/// Render the list view
pub fn render_list_view(
    frame: &mut Frame,
    area: Rect,
    list_view: &ListView,
    tasks: &[Task],
    scramble_mode: bool,
    config: &crate::config::Config,
) {
    if list_view.show_detail && list_view.get_selected_task_id().is_some() {
        // Split view: list on left, detail on right
        let chunks = Layout::horizontal([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(area);

        render_task_list(frame, chunks[0], list_view, tasks, scramble_mode, config);
        render_task_detail(frame, chunks[1], list_view, tasks, config);
    } else {
        // Full width task list
        render_task_list(frame, area, list_view, tasks, scramble_mode, config);
    }
}

fn render_task_list(
    frame: &mut Frame,
    area: Rect,
    list_view: &ListView,
    tasks: &[Task],
    scramble_mode: bool,
    config: &crate::config::Config,
) {
    use crate::list_view::FocusPanel;

    let selected_id = list_view.get_selected_task_id();

    let items: Vec<ListItem> = tasks
        .iter()
        .map(|task| {
            let is_selected = selected_id.as_ref() == Some(&task.id);
            let date_str = format!("{}", task.start.format("%Y-%m-%d"));
            let status_char = if task.completed { "✓" } else { " " };
            
            let title = if scramble_mode {
                let numeric_id: String = task.id.chars().filter(|c| c.is_numeric()).take(4).collect();
                format!("Task #{}", numeric_id)
            } else {
                task.title.clone()
            };

            let content = format!("[{}] {} - {}", status_char, date_str, title);

            let style = if is_selected {
                if task.completed {
                    Style::default()
                        .fg(config.ui_colors.selected_completed_task_fg)
                        .bg(config.ui_colors.selected_completed_task_bg)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                        .fg(config.ui_colors.selected_task_fg)
                        .bg(config.ui_colors.selected_task_bg)
                        .add_modifier(Modifier::BOLD)
                }
            } else {
                if task.completed {
                    Style::default().fg(config.ui_colors.completed_task_fg)
                } else {
                    Style::default().fg(config.ui_colors.default_fg)
                }
            };

            ListItem::new(Line::from(vec![Span::styled(content, style)]))
        })
        .collect();

    let title = if list_view.show_detail {
        "Tasks (h/l to switch focus)"
    } else {
        "All Tasks (Enter to view details)"
    };

    let border_style = if list_view.focus == FocusPanel::List {
        Style::default().fg(config.ui_colors.selected_task_bg)
    } else {
        Style::default().fg(config.ui_colors.default_fg)
    };

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(border_style),
        )
        .style(Style::default().fg(config.ui_colors.default_fg));

    frame.render_widget(list, area);
}

fn render_task_detail(
    frame: &mut Frame,
    area: Rect,
    list_view: &ListView,
    tasks: &[Task],
    config: &crate::config::Config,
) {
    use crate::list_view::{DetailEditField, FocusPanel};

    let selected_id = match &list_view.selection {
        ListSelection::Task(id) => id,
        ListSelection::NoTasks => {
            let paragraph = Paragraph::new("No task selected")
                .block(Block::default().borders(Borders::ALL).title("Task Details"));
            frame.render_widget(paragraph, area);
            return;
        }
    };

    let task = match tasks.iter().find(|t| &t.id == selected_id) {
        Some(t) => t,
        None => {
            let paragraph = Paragraph::new("Task not found")
                .block(Block::default().borders(Borders::ALL).title("Task Details"));
            frame.render_widget(paragraph, area);
            return;
        }
    };

    // Build the detail content
    let mut lines = vec![];
    
    // Title (editable)
    let title_style = if list_view.editing_field == DetailEditField::Title {
        Style::default().add_modifier(Modifier::BOLD | Modifier::UNDERLINED).fg(config.ui_colors.selected_task_bg)
    } else {
        Style::default()
    };
    
    lines.push(Line::from(vec![
        Span::styled("Title: ", Style::default().add_modifier(Modifier::BOLD)),
        if list_view.editing_field == DetailEditField::Title {
            Span::styled(&list_view.edit_buffer, title_style)
        } else {
            Span::raw(&task.title)
        },
    ]));
    lines.push(Line::from(""));

    // Date
    lines.push(Line::from(vec![
        Span::styled("Date: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(format!("{}", task.start.format("%Y-%m-%d %H:%M"))),
    ]));
    lines.push(Line::from(""));

    // Status
    lines.push(Line::from(vec![
        Span::styled("Status: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(if task.completed { "Completed" } else { "Pending" }),
    ]));
    lines.push(Line::from(""));

    // Comments/Description (editable)
    let desc_style = if list_view.editing_field == DetailEditField::Description {
        Style::default().add_modifier(Modifier::BOLD | Modifier::UNDERLINED).fg(config.ui_colors.selected_task_bg)
    } else {
        Style::default()
    };

    if list_view.editing_field == DetailEditField::Description {
        lines.push(Line::from(Span::styled(
            "Description:",
            Style::default().add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));
        for line in list_view.edit_buffer.lines() {
            lines.push(Line::from(Span::styled(line, desc_style)));
        }
    } else if !task.comments.is_empty() {
        lines.push(Line::from(Span::styled(
            "Description:",
            Style::default().add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));
        for comment in &task.comments {
            // Split comment into lines for better display
            for line in comment.text.lines() {
                lines.push(Line::from(Span::raw(line)));
            }
        }
    } else {
        lines.push(Line::from(Span::styled(
            "Description: (none)",
            Style::default().add_modifier(Modifier::ITALIC),
        )));
    }
    
    // Add spacing and help text
    lines.push(Line::from(""));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("─".repeat(40), Style::default().fg(config.ui_colors.default_fg)),
    ]));
    
    if list_view.is_editing() {
        lines.push(Line::from(vec![
            Span::styled("Editing: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled("Enter", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" to save, "),
            Span::styled("Tab", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" to switch field, "),
            Span::styled("Esc", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" to cancel"),
        ]));
    } else if list_view.focus == FocusPanel::Detail {
        lines.push(Line::from(vec![
            Span::styled("Press ", Style::default()),
            Span::styled("i", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(" to edit | ", Style::default()),
            Span::styled("h", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(" to return to list", Style::default()),
        ]));
    } else {
        lines.push(Line::from(vec![
            Span::styled("Press ", Style::default()),
            Span::styled("l", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(" to focus detail | ", Style::default()),
            Span::styled("Esc", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(" to hide details", Style::default()),
        ]));
    }

    let border_style = if list_view.focus == FocusPanel::Detail {
        Style::default().fg(config.ui_colors.selected_task_bg)
    } else {
        Style::default().fg(config.ui_colors.default_fg)
    };

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Task Details")
                .border_style(border_style),
        )
        .wrap(Wrap { trim: true })
        .style(Style::default().fg(config.ui_colors.default_fg));

    frame.render_widget(paragraph, area);
}
