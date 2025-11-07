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

#[derive(Debug, Clone)]
pub struct ListView {
    pub selection: ListSelection,
    pub show_detail: bool,  // whether to show task details on the right
}

impl ListView {
    pub fn new() -> Self {
        Self {
            selection: ListSelection::NoTasks,
            show_detail: false,
        }
    }

    /// Get the currently selected task ID, if any
    pub fn get_selected_task_id(&self) -> Option<String> {
        match &self.selection {
            ListSelection::Task(id) => Some(id.clone()),
            ListSelection::NoTasks => None,
        }
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
        "Tasks (Press Esc to hide details)"
    } else {
        "All Tasks (Press Enter to view details)"
    };

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(Style::default().fg(config.ui_colors.default_fg)),
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
    
    // Title
    lines.push(Line::from(vec![
        Span::styled("Title: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(&task.title),
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
        Span::raw(if task.completed { "Completed ✓" } else { "Pending" }),
    ]));
    lines.push(Line::from(""));

    // Comments/Description
    if !task.comments.is_empty() {
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
    lines.push(Line::from(vec![
        Span::styled("Press ", Style::default()),
        Span::styled("i", Style::default().add_modifier(Modifier::BOLD)),
        Span::styled(" to edit this task", Style::default()),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Press ", Style::default()),
        Span::styled("Esc", Style::default().add_modifier(Modifier::BOLD)),
        Span::styled(" to hide details", Style::default()),
    ]));

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Task Details")
                .border_style(Style::default().fg(config.ui_colors.default_fg)),
        )
        .wrap(Wrap { trim: true })
        .style(Style::default().fg(config.ui_colors.default_fg));

    frame.render_widget(paragraph, area);
}
