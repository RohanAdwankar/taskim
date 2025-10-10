mod commands;
mod config;
mod data;
mod month_view;
mod recurrence;
mod task;
mod task_edit;
mod undo;
mod utils;

use crate::data::{load_data, save_data};
use crate::month_view::{render_month_view, MonthView, SelectionType};
use crate::recurrence::{
    build_draft_from_motion, parse_command as parse_recurrence_command, ParsedCommand,
    RecurrenceDraft, RecurrenceMotion, RecurrenceSpawnMode,
};
use crate::task::Task;
use crate::task::TaskData;
use crate::task_edit::{render_task_edit_popup, TaskEditState};
use crate::undo::{Operation, UndoStack};
use crate::utils::days_in_month;
use commands::get_command_registry;

use chrono::{Datelike, Local, Timelike};
use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
    DefaultTerminal, Frame,
};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone)]
enum AppMode {
    Normal,
    TaskEdit(TaskEditState),
    Command(CommandState),
    RecurrencePreview(RecurrencePreviewState),
}

#[derive(Debug, Clone, PartialEq)]
struct CommandState {
    input: String,
    cursor_position: usize,
    show_help: bool,
    last_error: Option<String>,
}

#[derive(Debug, Clone)]
struct RecurrencePreviewState {
    task_id: String,
    draft: RecurrenceDraft,
}

impl CommandState {
    fn new() -> Self {
        Self {
            input: String::new(),
            cursor_position: 0,
            show_help: false,
            last_error: None,
        }
    }

    fn add_char(&mut self, ch: char) {
        self.input.insert(self.cursor_position, ch);
        self.cursor_position += 1;
    }

    fn remove_char(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
            self.input.remove(self.cursor_position);
        }
    }

    fn move_cursor_left(&mut self) {
        self.cursor_position = self.cursor_position.saturating_sub(1);
    }

    fn move_cursor_right(&mut self) {
        if self.cursor_position < self.input.len() {
            self.cursor_position += 1;
        }
    }
}

struct App {
    mode: AppMode,
    data: TaskData,
    month_view: MonthView,
    should_exit: bool,
    undo_stack: UndoStack,
    yanked_task: Option<crate::task::Task>, // Store yanked task for paste operation
    pending_key: Option<char>,              // For handling multi-key sequences like 'gg'
    pending_insert_order: Option<u32>,      // For tracking task insertion order
    scramble_mode: bool,                    // Toggle for scrambling task names with numbers
    config: crate::config::Config,          // <-- add config field
    show_keybinds: bool,                    // runtime toggle for keybind help
    status_message: Option<String>,         // surface transient status information
}

impl App {
    fn new() -> Self {
        let data = load_data();
        let current_date = Local::now().date_naive();
        let month_view = MonthView::new(current_date);
        let config = crate::config::Config::from_file_or_default("config.yml");
        let show_keybinds = config.show_keybinds;
        Self {
            mode: AppMode::Normal,
            data,
            month_view,
            should_exit: false,
            undo_stack: UndoStack::new(50), // Allow up to 50 undo operations
            yanked_task: None,
            pending_key: None,
            pending_insert_order: None,
            scramble_mode: false,
            config,
            show_keybinds,
            status_message: None,
        }
    }

    fn save(&self) -> Result<()> {
        save_data(&self.data).map_err(|e| color_eyre::eyre::eyre!(e))?;
        Ok(())
    }

    fn set_status_message<S: Into<String>>(&mut self, message: S) {
        self.status_message = Some(message.into());
    }

    fn handle_key_event(&mut self, key: crossterm::event::KeyEvent) -> Result<()> {
        match &self.mode {
            AppMode::Normal => self.handle_normal_mode_key(key)?,
            AppMode::Command(state) => {
                let mut new_state = state.clone();
                if self.handle_command_mode_key(key, &mut new_state)? {
                    // Command completed or cancelled. If the mode wasn't changed during command execution,
                    // fall back to normal mode. This allows commands to transition into other modes (e.g.,
                    // recurrence preview) without being overwritten here.
                    if matches!(self.mode, AppMode::Command(_)) {
                        self.mode = AppMode::Normal;
                    }
                } else {
                    self.mode = AppMode::Command(new_state);
                }
            }
            AppMode::TaskEdit(state) => {
                let mut new_state = state.clone();
                if self.handle_task_edit_key(key, &mut new_state)? {
                    // Task edit completed
                    let mut task = new_state.to_task();
                    if new_state.is_new_task {
                        // Use pending insert order if set (for 'o' and 'O' commands)
                        if let Some(insert_order) = self.pending_insert_order.take() {
                            self.data.insert_task_at_order(task.clone(), insert_order);

                            // Select the new task by its order
                            let task_date = task.start.date_naive();
                            self.month_view.select_task_by_order(
                                task_date,
                                insert_order,
                                &self.data.events,
                            );
                        } else {
                            // Regular insertion (for 'i' command) - add to end
                            let task_date = task.start.date_naive();
                            task.order = self.data.max_order_for_date(task_date) + 1;
                            self.data.events.push(task.clone());
                        }

                        // Track task creation
                        self.undo_stack
                            .push(Operation::CreateTask { task: task.clone() });
                    } else {
                        // Track task edit
                        if let Some(existing) = self
                            .data
                            .events
                            .iter_mut()
                            .find(|t| Some(&t.id) == new_state.task_id.as_ref())
                        {
                            let old_task = existing.clone();
                            *existing = task.clone();

                            self.undo_stack.push(Operation::EditTask {
                                task_id: task.id.clone(),
                                old_task,
                                new_task: task,
                            });
                        }
                    }
                    self.mode = AppMode::Normal;
                    self.save()?;
                } else {
                    self.mode = AppMode::TaskEdit(new_state);
                }
            }
            AppMode::RecurrencePreview(state) => {
                let mut preview_state = state.clone();
                if self.handle_recurrence_preview_key(key, &mut preview_state)? {
                    self.mode = AppMode::Normal;
                } else {
                    self.mode = AppMode::RecurrencePreview(preview_state);
                }
            }
        }
        Ok(())
    }

    fn handle_normal_mode_key(&mut self, key: crossterm::event::KeyEvent) -> Result<()> {
        // Handle keybindings
        if self.config.force_quit.matches(key.code, key.modifiers) {
            self.should_exit = true;
            return Ok(());
        }

        // Handle multi-key sequences first
        if let Some(pending) = self.pending_key {
            if pending == 'g'
                && key.code == KeyCode::Char('g')
                && key.modifiers == KeyModifiers::NONE
            {
                // Handle 'gg' - go to previous year
                self.month_view.prev_year();
                self.pending_key = None;
                return Ok(());
            } else if pending == 'd'
                && key.code == KeyCode::Char('d')
                && key.modifiers == KeyModifiers::NONE
            {
                // Handle 'dd' - cut the selected task (vim-style)
                if let Some(task_id) = self.month_view.get_selected_task_id() {
                    if let Some(task) = self.data.remove_task_and_reorder(&task_id) {
                        let task_date = task.start.date_naive();

                        // Store the cut task for pasting
                        self.yanked_task = Some(task.clone());

                        self.handle_task_removed_side_effects(&task);

                        // Track deletion for undo functionality
                        self.undo_stack.push(Operation::DeleteTask { task });

                        // Check if there are any remaining tasks on the same date
                        let remaining_tasks = self.data.get_tasks_for_date(task_date);

                        if remaining_tasks.is_empty() {
                            // No more tasks on this day, select the day itself
                            self.month_view.selection = month_view::Selection {
                                selection_type: month_view::SelectionType::Day(task_date),
                            };
                        } else {
                            // Select the first remaining task
                            self.month_view.selection = month_view::Selection {
                                selection_type: month_view::SelectionType::Task(
                                    remaining_tasks[0].id.clone(),
                                ),
                            };
                        }

                        self.save()?;
                    }
                }
                self.pending_key = None;
                return Ok(());
            } else if pending == 'r' && key.modifiers == KeyModifiers::NONE {
                if let KeyCode::Char(ch) = key.code {
                    if let Some(motion) = RecurrenceMotion::from_char(ch) {
                        match self.start_recurrence_preview_from_motion(motion) {
                            Ok(_) => {}
                            Err(err) => self.set_status_message(err),
                        }
                        self.pending_key = None;
                        return Ok(());
                    }
                }
                self.pending_key = None;
                return Ok(());
            }
            // If we have a pending key but don't match, clear it and continue with normal processing
            self.pending_key = None;
        }

        if self.config.quit.matches(key.code, key.modifiers)
            || self.config.quit_alt.matches(key.code, key.modifiers)
        {
            self.should_exit = true;
        } else if self.config.move_left.matches(key.code, key.modifiers) {
            self.month_view.move_left(&self.data.events);
        } else if self.config.move_down.matches(key.code, key.modifiers) {
            self.month_view.move_down(&self.data.events);
        } else if self.config.move_up.matches(key.code, key.modifiers) {
            self.month_view.move_up(&self.data.events);
        } else if self.config.move_right.matches(key.code, key.modifiers) {
            self.month_view.move_right(&self.data.events);
        } else if self.config.insert_edit.matches(key.code, key.modifiers) {
            match &self.month_view.selection.selection_type {
                SelectionType::Day(date) => {
                    // Create new task
                    let edit_state = TaskEditState::new_task(*date);
                    self.mode = AppMode::TaskEdit(edit_state);
                }
                SelectionType::Task(task_id) => {
                    // Edit existing task
                    if let Some(task) = self.data.events.iter().find(|t| &t.id == task_id) {
                        let edit_state = TaskEditState::edit_task(task);
                        self.mode = AppMode::TaskEdit(edit_state);
                    }
                }
            }
        } else if self.config.save_task.matches(key.code, key.modifiers) {
            match &self.month_view.selection.selection_type {
                SelectionType::Task(task_id) => {
                    // Edit existing task (same as insert_edit for task)
                    if let Some(task) = self.data.events.iter().find(|t| &t.id == task_id) {
                        let edit_state = TaskEditState::edit_task(task);
                        self.mode = AppMode::TaskEdit(edit_state);
                    }
                }
                _ => {}
            }
        } else if self.config.insert_below.matches(key.code, key.modifiers) {
            // Insert task below current position (vim-style: o)
            let selected_date = self.month_view.get_selected_date(&self.data.events);
            let edit_state = TaskEditState::new_task(selected_date);

            // Store the insertion order for when the task is created
            let insert_order = if let Some(current_order) =
                self.month_view.get_current_task_order(&self.data.events)
            {
                current_order + 1
            } else {
                self.data.max_order_for_date(selected_date) + 1
            };

            // We'll need to track this order for when the task gets created
            // For now, set up the task edit state
            self.pending_insert_order = Some(insert_order);
            self.mode = AppMode::TaskEdit(edit_state);
        } else if self.config.insert_above.matches(key.code, key.modifiers) {
            // Insert task above current position (vim-style: O)
            let selected_date = self.month_view.get_selected_date(&self.data.events);
            let edit_state = TaskEditState::new_task(selected_date);

            // Store the insertion order for when the task is created
            let insert_order = if let Some(current_order) =
                self.month_view.get_current_task_order(&self.data.events)
            {
                current_order
            } else {
                0
            };

            // We'll need to track this order for when the task gets created
            self.pending_insert_order = Some(insert_order);
            self.mode = AppMode::TaskEdit(edit_state);
        } else if self.config.delete_line.matches(key.code, key.modifiers) {
            // Handle first 'd' for 'dd' sequence
            self.pending_key = Some('d');
        } else if self.config.delete.matches(key.code, key.modifiers) {
            // Delete/cut the selected task (vim-style 'x') - same as 'dd'
            if let Some(task_id) = self.month_view.get_selected_task_id() {
                if let Some(task) = self.data.remove_task_and_reorder(&task_id) {
                    let task_date = task.start.date_naive();

                    // Store the cut task for pasting (copy functionality)
                    self.yanked_task = Some(task.clone());

                    self.handle_task_removed_side_effects(&task);

                    // Track deletion for undo functionality
                    self.undo_stack.push(Operation::DeleteTask { task });

                    // Check if there are any remaining tasks on the same date
                    let remaining_tasks = self.data.get_tasks_for_date(task_date);

                    if remaining_tasks.is_empty() {
                        // No more tasks on this day, select the day itself
                        self.month_view.selection = month_view::Selection {
                            selection_type: month_view::SelectionType::Day(task_date),
                        };
                    } else {
                        // Select the first remaining task (ordered)
                        self.month_view.selection = month_view::Selection {
                            selection_type: month_view::SelectionType::Task(
                                remaining_tasks[0].id.clone(),
                            ),
                        };
                    }

                    self.save()?;
                }
            }
        } else if self.config.undo.matches(key.code, key.modifiers) {
            // Undo last operation
            if let Some(operation) = self.undo_stack.undo() {
                match operation {
                    Operation::DeleteTask { task } => {
                        // Restore deleted task
                        self.data.events.push(task.clone());
                        self.handle_task_restored_side_effects(&task);

                        // Select the restored task
                        self.month_view.selection = month_view::Selection {
                            selection_type: month_view::SelectionType::Task(task.id),
                        };
                    }
                    Operation::EditTask {
                        task_id,
                        old_task,
                        new_task: _,
                    } => {
                        // Revert task edit
                        if let Some(existing) =
                            self.data.events.iter_mut().find(|t| t.id == task_id)
                        {
                            *existing = old_task;
                        }
                    }
                    Operation::CreateTask { task } => {
                        // Remove created task
                        self.data.events.retain(|t| t.id != task.id);
                        self.handle_task_removed_side_effects(&task);

                        // Select the day where the task was
                        let task_date = task.start.date_naive();
                        self.month_view.selection = month_view::Selection {
                            selection_type: month_view::SelectionType::Day(task_date),
                        };
                    }
                }
                self.save()?;
            }
        } else if self.config.redo.matches(key.code, key.modifiers) {
            // Redo last undone operation
            if let Some(operation) = self.undo_stack.redo() {
                match operation {
                    Operation::DeleteTask { task } => {
                        // Re-delete the task
                        self.data.events.retain(|t| t.id != task.id);
                        self.handle_task_removed_side_effects(&task);

                        // Select the day where the task was
                        let task_date = task.start.date_naive();
                        self.month_view.selection = month_view::Selection {
                            selection_type: month_view::SelectionType::Day(task_date),
                        };
                    }
                    Operation::EditTask {
                        task_id,
                        old_task: _,
                        new_task,
                    } => {
                        // Re-apply task edit
                        if let Some(existing) =
                            self.data.events.iter_mut().find(|t| t.id == task_id)
                        {
                            *existing = new_task;
                        }
                    }
                    Operation::CreateTask { task } => {
                        // Re-create task
                        self.data.events.push(task.clone());
                        self.handle_task_restored_side_effects(&task);

                        // Select the restored task
                        self.month_view.selection = month_view::Selection {
                            selection_type: month_view::SelectionType::Task(task.id),
                            // task_index_in_day: Some(0),
                        };
                    }
                }
                self.save()?;
            }
        } else if key.code == KeyCode::Char('r') && key.modifiers == KeyModifiers::NONE {
            self.pending_key = Some('r');
            self.set_status_message(
                "Awaiting recurrence modifier (d=day, w=week, m=month, y=year).",
            );
            return Ok(());
        } else if self.config.toggle_complete.matches(key.code, key.modifiers) {
            // Toggle task completion
            if let Some(task_id) = self.month_view.get_selected_task_id() {
                if let Some(index) = self.data.events.iter().position(|t| t.id == task_id) {
                    let was_completed = self.data.events[index].completed;
                    self.data.events[index].completed = !was_completed;
                    let task_snapshot = self.data.events[index].clone();

                    if !was_completed {
                        let previous_message = self.status_message.clone();
                        if let Err(err) = self.handle_task_marked_complete(&task_snapshot) {
                            self.set_status_message(err);
                        } else if self.status_message.as_ref() == previous_message.as_ref() {
                            self.set_status_message("Task marked complete.");
                        }
                    } else {
                        self.set_status_message("Task marked incomplete.");
                    }

                    self.save()?;
                }
            }
        } else if self.config.yank.matches(key.code, key.modifiers) {
            // Yank (copy) task
            if let Some(task_id) = self.month_view.get_selected_task_id() {
                if let Some(task) = self.data.events.iter().find(|t| t.id == task_id) {
                    self.yanked_task = Some(task.clone());
                }
            }
        } else if self.config.paste.matches(key.code, key.modifiers) {
            // Paste task below current position
            if let Some(yanked_task) = &self.yanked_task {
                let selected_date = self.month_view.get_selected_date(&self.data.events);
                let mut new_task = yanked_task.clone();

                // Generate new ID for the pasted task
                new_task.id = format!(
                    "task_{}",
                    chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
                );

                // Set new start/end times for the selected date
                let duration = new_task.end - new_task.start;
                let new_start = selected_date
                    .and_hms_opt(
                        new_task.start.time().hour(),
                        new_task.start.time().minute(),
                        new_task.start.time().second(),
                    )
                    .unwrap()
                    .and_utc();
                new_task.start = new_start;
                new_task.end = new_start + duration;

                // Insert task with proper ordering
                let insert_order = if let Some(current_order) =
                    self.month_view.get_current_task_order(&self.data.events)
                {
                    current_order + 1
                } else {
                    self.data.max_order_for_date(selected_date) + 1
                };

                self.data
                    .insert_task_at_order(new_task.clone(), insert_order);

                // Track the paste operation for undo
                self.undo_stack.push(Operation::CreateTask {
                    task: new_task.clone(),
                });

                // Select the new task
                self.month_view.select_task_by_order(
                    selected_date,
                    insert_order,
                    &self.data.events,
                );
                self.save()?;
            }
        } else if self.config.paste_above.matches(key.code, key.modifiers) {
            // Paste task above current position
            if let Some(yanked_task) = &self.yanked_task {
                let selected_date = self.month_view.get_selected_date(&self.data.events);
                let mut new_task = yanked_task.clone();

                // Generate new ID for the pasted task
                new_task.id = format!(
                    "task_{}",
                    chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
                );

                // Set new start/end times for the selected date
                let duration = new_task.end - new_task.start;
                let new_start = selected_date
                    .and_hms_opt(
                        new_task.start.time().hour(),
                        new_task.start.time().minute(),
                        new_task.start.time().second(),
                    )
                    .unwrap()
                    .and_utc();
                new_task.start = new_start;
                new_task.end = new_start + duration;

                // Insert task with proper ordering (above current)
                let insert_order = if let Some(current_order) =
                    self.month_view.get_current_task_order(&self.data.events)
                {
                    current_order
                } else {
                    0
                };

                self.data
                    .insert_task_at_order(new_task.clone(), insert_order);

                // Track the paste operation for undo
                self.undo_stack.push(Operation::CreateTask {
                    task: new_task.clone(),
                });

                // Select the new task
                self.month_view.select_task_by_order(
                    selected_date,
                    insert_order,
                    &self.data.events,
                );
                self.save()?;
            }
        } else if self.config.next_month.matches(key.code, key.modifiers) {
            // Next month (vim-style: L) - preserve day
            self.month_view.next_month_preserve_day();
        } else if self.config.prev_month.matches(key.code, key.modifiers) {
            // Previous month (vim-style: H) - preserve day
            self.month_view.prev_month_preserve_day();
        } else if self.config.next_year.matches(key.code, key.modifiers) {
            // Next year (vim-style: G)
            self.month_view.next_year();
        } else if self.config.prev_year.matches(key.code, key.modifiers) {
            // Handle first 'g' for 'gg' sequence
            self.pending_key = Some('g');
        } else if self.config.go_to_today.matches(key.code, key.modifiers) {
            // Go to today (vim-style: t)
            self.month_view.go_to_today();
        } else if self.config.next_week.matches(key.code, key.modifiers) {
            // Next week (vim-style: w)
            self.month_view.next_week(&self.data.events);
        } else if self.config.prev_week.matches(key.code, key.modifiers) {
            // Previous week (vim-style: b)
            self.month_view.prev_week(&self.data.events);
        } else if self
            .config
            .first_day_of_month
            .matches(key.code, key.modifiers)
        {
            // First day of month (vim-style: 0)
            self.month_view.first_day_of_month();
        } else if self
            .config
            .last_day_of_month
            .matches(key.code, key.modifiers)
            || (key.code == KeyCode::Char('$') && key.modifiers == KeyModifiers::NONE)
        {
            // Last day of month (vim-style: $) - handle both shift+4 and direct $
            self.month_view.last_day_of_month();
        } else if key.code == KeyCode::Char(':') && key.modifiers == KeyModifiers::NONE {
            // Enter command mode (vim-style: :)
            self.status_message = None;
            self.mode = AppMode::Command(CommandState::new());
        } else if key.code == KeyCode::Char('s') && key.modifiers == KeyModifiers::NONE {
            // Toggle scramble mode
            self.scramble_mode = !self.scramble_mode;
        }
        Ok(())
    }

    fn handle_task_edit_key(
        &mut self,
        key: crossterm::event::KeyEvent,
        state: &mut TaskEditState,
    ) -> Result<bool> {
        if key.kind == KeyEventKind::Press {
            if self.config.cancel_edit.matches(key.code, key.modifiers) {
                // Cancel edit
                return Ok(true);
            } else if self.config.save_task.matches(key.code, key.modifiers) {
                // Save task
                if !state.title.trim().is_empty() {
                    return Ok(true);
                }
            } else if self.config.switch_field.matches(key.code, key.modifiers) {
                state.switch_field();
            } else if self.config.backspace.matches(key.code, key.modifiers) {
                state.remove_char();
            } else if let KeyCode::Char(ch) = key.code {
                state.add_char(ch);
            }
        }
        Ok(false)
    }

    fn handle_command_mode_key(
        &mut self,
        key: crossterm::event::KeyEvent,
        state: &mut CommandState,
    ) -> Result<bool> {
        match key.code {
            KeyCode::Esc => {
                // Cancel command mode
                return Ok(true);
            }
            KeyCode::Enter => {
                // Execute command
                let command = state.input.trim();

                if command == "help" {
                    // Toggle help display
                    state.show_help = !state.show_help;
                    state.input.clear();
                    state.cursor_position = 0;
                    return Ok(false); // Stay in command mode to show help
                } else if !command.is_empty() {
                    match self.execute_command(&state.input) {
                        Ok(_) => {
                            state.last_error = None;
                        }
                        Err(e) => {
                            state.last_error = Some(e);
                        }
                    }
                    state.input.clear();
                    state.cursor_position = 0;
                    return Ok(state.last_error.is_none());
                } else {
                    // Empty command, just exit
                    return Ok(true);
                }
            }
            KeyCode::Backspace => {
                state.remove_char();
                // Hide help when user starts typing
                state.show_help = false;
            }
            KeyCode::Left => {
                state.move_cursor_left();
            }
            KeyCode::Right => {
                state.move_cursor_right();
            }
            KeyCode::Char(ch) => {
                state.add_char(ch);
                // Hide help when user starts typing
                state.show_help = false;
            }
            _ => {}
        }
        Ok(false)
    }

    fn execute_command(&mut self, command: &str) -> Result<(), String> {
        let trimmed = command.trim();
        let registry = get_command_registry();
        // Special handling for help command
        if trimmed.starts_with("help") {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() == 1 {
                let mut help_text = String::from("Available commands:\n");
                for (cmd, info) in &registry {
                    help_text.push_str(&format!(":{:<15} - {}\n", cmd, info.description));
                }
                return Err(help_text);
            } else if parts.len() == 2 {
                let query = parts[1].trim_start_matches(':');
                if let Some(info) = registry.get(query) {
                    return Err(format!(":{} - {}", query, info.description));
                } else {
                    return Err(format!("No help found for :{}", query));
                }
            }
        }
        if trimmed.is_empty() {
            return Ok(());
        }
        if let Some(result) = self.try_execute_recurrence_command(trimmed) {
            return result;
        }
        // Try registry
        if let Some(cmd) = registry.get(trimmed) {
            (cmd.exec)(self, trimmed)?;
            return Ok(());
        }
        // Try to parse as a date in various formats
        if let Some(date) = self.parse_date_command(trimmed) {
            if date.month() != self.month_view.current_date.month()
                || date.year() != self.month_view.current_date.year()
            {
                self.month_view.current_date = date.with_day(1).unwrap();
                self.month_view.weeks =
                    MonthView::build_weeks_for_date(self.month_view.current_date);
            }
            self.month_view.selection = month_view::Selection {
                selection_type: month_view::SelectionType::Day(date),
            };
            return Ok(());
        }
        Err(format!(
            "Unknown command: {}. Type ':help' for available commands.",
            trimmed
        ))
    }

    fn parse_date_command(&self, input: &str) -> Option<chrono::NaiveDate> {
        use chrono::NaiveDate;

        // Try parsing as YYYY (year only)
        if let Ok(year) = input.parse::<i32>() {
            if year >= 1900 && year <= 2050 {
                let current_month = self.month_view.current_date.month();
                let current_day = self.month_view.get_selected_date(&self.data.events).day();

                // Calculate days in the target month for the specified year
                let days_in_month = days_in_month(year, current_month);

                let safe_day = std::cmp::min(current_day, days_in_month);
                return NaiveDate::from_ymd_opt(year, current_month, safe_day);
            }
        }

        // Try parsing as MM/DD/YYYY (simple manual parsing)
        let parts: Vec<&str> = input.split('/').collect();
        if parts.len() == 3 {
            if let (Ok(month), Ok(day), Ok(year)) = (
                parts[0].parse::<u32>(),
                parts[1].parse::<u32>(),
                parts[2].parse::<i32>(),
            ) {
                return NaiveDate::from_ymd_opt(year, month, day);
            }
        }

        // Try parsing as DD (day only)
        if let Ok(day) = input.parse::<u32>() {
            if day >= 1 && day <= 31 {
                let current_year = self.month_view.current_date.year();
                let current_month = self.month_view.current_date.month();

                // Check if the day is valid for the current month
                let days_in_month = days_in_month(current_year, current_month);

                if day <= days_in_month {
                    return NaiveDate::from_ymd_opt(current_year, current_month, day);
                }
            }
        }

        None
    }

    fn build_display_tasks(&self) -> Vec<Task> {
        let mut tasks = self.data.events.clone();
        if let AppMode::RecurrencePreview(state) = &self.mode {
            tasks.extend(self.build_preview_tasks(state));
        }
        tasks
    }

    fn build_preview_tasks(&self, state: &RecurrencePreviewState) -> Vec<Task> {
        let mut previews = Vec::new();
        let base_task = match self.data.events.iter().find(|t| t.id == state.task_id) {
            Some(task) => task.clone(),
            None => return previews,
        };

        let mut order_map: HashMap<chrono::NaiveDate, u32> = HashMap::new();
        for task in &self.data.events {
            let date = task.start.date_naive();
            let entry = order_map.entry(date).or_insert(task.order);
            if task.order > *entry {
                *entry = task.order;
            }
        }

        for (idx, occurrence) in state.draft.occurrences.iter().enumerate() {
            let mut preview = base_task.clone();
            preview.id = format!("preview:{}:{}", state.draft.series.id, idx);
            preview.start = occurrence.start;
            preview.end = occurrence.end;
            preview.completed = false;
            preview.is_preview = true;
            preview.recurrence_series_id = Some(state.draft.series.id.clone());
            preview.recurrence_occurrence =
                Some(state.draft.series.generated_occurrences + idx as u32);

            let date = preview.start.date_naive();
            let entry = order_map
                .entry(date)
                .or_insert_with(|| self.data.max_order_for_date(date));
            *entry += 1;
            preview.order = *entry;

            previews.push(preview);
        }

        previews
    }

    fn start_recurrence_preview_from_motion(
        &mut self,
        motion: RecurrenceMotion,
    ) -> Result<(), String> {
        let task_id = self
            .month_view
            .get_selected_task_id()
            .ok_or_else(|| "Select a task before setting recurrence.".to_string())?;

        let task = self
            .data
            .events
            .iter()
            .find(|t| t.id == task_id)
            .cloned()
            .ok_or_else(|| "Selected task could not be found.".to_string())?;

        if task.recurrence_series_id.is_some() {
            return Err("Task already has a recurrence. Use :r/clear first.".to_string());
        }

        let draft = build_draft_from_motion(&task, motion);
        self.activate_recurrence_preview(task_id, draft)
    }

    fn activate_recurrence_preview(
        &mut self,
        task_id: String,
        draft: RecurrenceDraft,
    ) -> Result<(), String> {
        self.mode = AppMode::RecurrencePreview(RecurrencePreviewState { task_id, draft });
        self.status_message = None;
        Ok(())
    }

    fn handle_recurrence_preview_key(
        &mut self,
        key: crossterm::event::KeyEvent,
        _state: &mut RecurrencePreviewState,
    ) -> Result<bool> {
        if key.kind != KeyEventKind::Press {
            return Ok(false);
        }

        match key.code {
            KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => {
                if let AppMode::RecurrencePreview(state) = &self.mode {
                    let preview_state = state.clone();
                    match self.commit_recurrence(preview_state) {
                        Ok(_) => {
                            self.set_status_message("Recurrence applied.");
                            return Ok(true);
                        }
                        Err(err) => {
                            self.set_status_message(err);
                            return Ok(false);
                        }
                    }
                }
                Ok(false)
            }
            KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => {
                self.set_status_message("Recurrence cancelled.");
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn commit_recurrence(&mut self, preview_state: RecurrencePreviewState) -> Result<(), String> {
        let task_index = self
            .data
            .events
            .iter()
            .position(|t| t.id == preview_state.task_id)
            .ok_or_else(|| "Selected task could not be found.".to_string())?;

        if self.data.events[task_index].recurrence_series_id.is_some() {
            return Err("Task already has a recurrence. Use :r/clear first.".to_string());
        }

        let old_task = self.data.events[task_index].clone();
        self.data.events[task_index].recurrence_series_id =
            Some(preview_state.draft.series.id.clone());
        self.data.events[task_index].recurrence_occurrence = Some(0);
        self.data.events[task_index].is_preview = false;
        let updated_task = self.data.events[task_index].clone();

        self.undo_stack.push(Operation::EditTask {
            task_id: updated_task.id.clone(),
            old_task,
            new_task: updated_task.clone(),
        });

        let mut series = preview_state.draft.series.clone();

        if series.spawn_mode == RecurrenceSpawnMode::AllAtOnce {
            for (idx, occurrence) in preview_state.draft.occurrences.iter().enumerate() {
                let mut new_task = updated_task.clone();
                new_task.id = Uuid::new_v4().to_string();
                new_task.start = occurrence.start;
                new_task.end = occurrence.end;
                new_task.completed = false;
                new_task.is_preview = false;
                new_task.recurrence_series_id = Some(series.id.clone());
                new_task.recurrence_occurrence = Some((idx as u32) + 1);

                let date = new_task.start.date_naive();
                let order = self.data.max_order_for_date(date) + 1;
                self.data.insert_task_at_order(new_task.clone(), order);

                self.undo_stack.push(Operation::CreateTask {
                    task: new_task.clone(),
                });
            }

            series.generated_occurrences = 1 + preview_state.draft.occurrences.len() as u32;
            series.active = false;
        } else {
            series.generated_occurrences = 1;
            series.active = series.has_remaining();
        }

        let series_id = series.id.clone();
        self.data.recurrences.insert(series_id, series);

        self.save().map_err(|e| e.to_string())?;
        Ok(())
    }

    fn handle_task_marked_complete(&mut self, completed_task: &Task) -> Result<(), String> {
        let Some(series_id) = &completed_task.recurrence_series_id else {
            return Ok(());
        };

        let mut next_task: Option<Task> = None;
        let mut completion_message: Option<String> = None;

        if let Some(series) = self.data.recurrences.get_mut(series_id) {
            if series.spawn_mode != RecurrenceSpawnMode::OnCompletion || !series.active {
                return Ok(());
            }

            if let Some(total) = series.total_occurrences {
                if series.generated_occurrences >= total {
                    series.active = false;
                    return Ok(());
                }
            }

            if let Some(next) = series.next_occurrence_after(completed_task.start) {
                let mut new_task = completed_task.clone();
                new_task.id = Uuid::new_v4().to_string();
                new_task.start = next.start;
                new_task.end = next.end;
                new_task.completed = false;
                new_task.is_preview = false;
                new_task.recurrence_series_id = Some(series_id.clone());
                new_task.recurrence_occurrence = Some(series.generated_occurrences);

                series.generated_occurrences += 1;
                if let Some(total) = series.total_occurrences {
                    if series.generated_occurrences >= total {
                        series.active = false;
                    }
                }
                next_task = Some(new_task);
            } else {
                series.active = false;
                completion_message = Some("Recurrence completed.".to_string());
            }
        } else {
            return Err("Recurrence metadata missing for completed task.".to_string());
        }

        if let Some(mut new_task) = next_task {
            let date = new_task.start.date_naive();
            let order = self.data.max_order_for_date(date) + 1;
            new_task.order = order;
            self.data.insert_task_at_order(new_task.clone(), order);
            self.undo_stack.push(Operation::CreateTask {
                task: new_task.clone(),
            });
            self.set_status_message(format!(
                "Spawned next occurrence on {}.",
                new_task.start.format("%Y-%m-%d")
            ));
        } else if let Some(message) = completion_message {
            self.set_status_message(message);
        }

        Ok(())
    }

    fn handle_task_removed_side_effects(&mut self, task: &Task) {
        if let Some(series_id) = &task.recurrence_series_id {
            if let Some(series) = self.data.recurrences.get_mut(series_id) {
                series.active = false;
            }
        }
    }

    fn handle_task_restored_side_effects(&mut self, task: &Task) {
        if let Some(series_id) = &task.recurrence_series_id {
            if let Some(series) = self.data.recurrences.get_mut(series_id) {
                if series.spawn_mode == RecurrenceSpawnMode::OnCompletion && series.has_remaining()
                {
                    series.active = true;
                }
            }
        }
    }

    fn clear_recurrence_for_task(&mut self, task_id: &str) -> Result<(), String> {
        let index = self
            .data
            .events
            .iter()
            .position(|t| t.id == task_id)
            .ok_or_else(|| "Selected task not found.".to_string())?;

        if self.data.events[index].recurrence_series_id.is_none() {
            return Err("Selected task does not have a recurrence.".to_string());
        }

        let series_id = self.data.events[index]
            .recurrence_series_id
            .clone()
            .unwrap();

        let mut edits = Vec::new();
        for task in self.data.events.iter_mut() {
            if task
                .recurrence_series_id
                .as_deref()
                .map(|id| id == series_id.as_str())
                .unwrap_or(false)
            {
                let old_task = task.clone();
                task.recurrence_series_id = None;
                task.recurrence_occurrence = None;
                task.is_preview = false;
                edits.push((task.id.clone(), old_task, task.clone()));
            }
        }

        for (task_id, old_task, new_task) in edits {
            self.undo_stack.push(Operation::EditTask {
                task_id,
                old_task,
                new_task,
            });
        }

        self.data.recurrences.remove(&series_id);
        self.save().map_err(|e| e.to_string())?;
        self.set_status_message("Recurrence cleared.");
        Ok(())
    }

    fn try_execute_recurrence_command(&mut self, command: &str) -> Option<Result<(), String>> {
        if !command.starts_with('r') {
            return None;
        }

        let task_id = match self.month_view.get_selected_task_id() {
            Some(id) => id,
            None => return Some(Err("Select a task before setting recurrence.".to_string())),
        };

        let task = match self.data.events.iter().find(|t| t.id == task_id).cloned() {
            Some(task) => task,
            None => return Some(Err("Selected task not found.".to_string())),
        };

        match parse_recurrence_command(command, &task) {
            Ok(ParsedCommand::Clear) => Some(self.clear_recurrence_for_task(&task_id)),
            Ok(ParsedCommand::Draft(draft)) => {
                if task.recurrence_series_id.is_some() {
                    return Some(Err(
                        "Task already has a recurrence. Use :r/clear first.".to_string()
                    ));
                }
                match self.activate_recurrence_preview(task_id, draft) {
                    Ok(_) => Some(Ok(())),
                    Err(err) => Some(Err(err)),
                }
            }
            Err(err) => Some(Err(err.message)),
        }
    }

    fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        loop {
            terminal.draw(|frame| self.render(frame))?;

            if self.should_exit {
                break;
            }

            if let Ok(event) = event::read() {
                if let Event::Key(key_event) = event {
                    self.handle_key_event(key_event)?;
                }
            }
        }
        Ok(())
    }

    fn render(&self, frame: &mut Frame) {
        let area = frame.area();

        // Create main layout - adjust footer size based on command mode
        let footer_height = match &self.mode {
            AppMode::Command(state) if state.show_help => 7, // More space for help (added wrap commands)
            _ => 2,                                          // Normal footer size
        };

        let layout = Layout::vertical([
            Constraint::Min(0),                // Main content
            Constraint::Length(footer_height), // Footer
        ])
        .split(area);

        // Render main content
        let display_tasks = self.build_display_tasks();
        render_month_view(
            frame,
            layout[0],
            &self.month_view,
            &display_tasks,
            self.scramble_mode,
            &self.config,
        );

        // Render footer
        self.render_footer(frame, layout[1]);

        // Render mode-specific overlays
        match &self.mode {
            AppMode::TaskEdit(state) => {
                render_task_edit_popup(frame, area, state, &self.config);
            }
            AppMode::Command(_) => {
                // Command mode is handled in the footer
            }
            AppMode::Normal => {}
            AppMode::RecurrencePreview(_) => {}
        }
    }

    fn render_footer(&self, frame: &mut Frame, area: Rect) {
        match &self.mode {
            AppMode::Command(state) => {
                let mut lines = vec![];
                if let Some(message) = &self.status_message {
                    lines.push(Line::from(vec![Span::raw(message.clone())]));
                }
                let has_error_or_help = state.last_error.is_some() || state.show_help;
                if let Some(err) = &state.last_error {
                    lines.push(Line::from(vec![Span::styled(
                        err,
                        Style::default().fg(self.config.ui_colors.selected_completed_task_bg),
                    )]));
                }
                if state.show_help {
                    let help_lines = vec![
                        Line::from(vec![Span::styled(
                            "Date Navigation Commands:",
                            Style::default().fg(self.config.ui_colors.selected_task_fg),
                        )]),
                        Line::from(vec![
                            Span::styled(
                                "YYYY",
                                Style::default().fg(self.config.ui_colors.selected_task_bg),
                            ),
                            Span::raw(" - Go to year (e.g., 2024) | "),
                            Span::styled(
                                "DD",
                                Style::default().fg(self.config.ui_colors.selected_task_bg),
                            ),
                            Span::raw(" - Go to day in current month (e.g., 15)"),
                        ]),
                        Line::from(vec![
                            Span::styled(
                                "MM/DD/YYYY",
                                Style::default().fg(self.config.ui_colors.selected_task_bg),
                            ),
                            Span::raw(" - Go to specific date (e.g., 06/15/2024)"),
                        ]),
                        Line::from(vec![
                            Span::styled(
                                "Quit Commands:",
                                Style::default().fg(self.config.ui_colors.completed_task_fg),
                            ),
                            Span::raw(" "),
                            Span::styled(
                                ":q",
                                Style::default().fg(self.config.ui_colors.selected_task_bg),
                            ),
                            Span::raw(" - Quit | "),
                        ]),
                        Line::from(vec![
                            Span::styled(
                                "Display Commands:",
                                Style::default().fg(self.config.ui_colors.completed_task_fg),
                            ),
                            Span::raw(" "),
                            Span::styled(
                                ":set wrap",
                                Style::default().fg(self.config.ui_colors.selected_task_bg),
                            ),
                            Span::raw(" - Enable text wrapping | "),
                            Span::styled(
                                ":set nowrap",
                                Style::default().fg(self.config.ui_colors.selected_task_bg),
                            ),
                            Span::raw(" - Disable text wrapping"),
                        ]),
                        Line::from(vec![
                            Span::styled(
                                ":help",
                                Style::default()
                                    .fg(self.config.ui_colors.selected_completed_task_fg),
                            ),
                            Span::raw(" - Toggle this help | "),
                            Span::styled(
                                "Esc",
                                Style::default()
                                    .fg(self.config.ui_colors.selected_completed_task_bg),
                            ),
                            Span::raw(" - Exit command mode"),
                        ]),
                    ];
                    lines.extend(help_lines)
                }
                if !has_error_or_help {
                    let command_line = format!(":{}", state.input);
                    lines.push(Line::from(vec![Span::raw(command_line)]));
                }
                let help_paragraph = Paragraph::new(lines).style(
                    Style::default()
                        .fg(self.config.ui_colors.default_fg)
                        .bg(self.config.ui_colors.default_bg),
                );
                frame.render_widget(help_paragraph, area);
            }
            AppMode::Normal => {
                let mut lines = Vec::new();
                if let Some(message) = &self.status_message {
                    lines.push(Line::from(vec![Span::raw(message.clone())]));
                }
                if self.show_keybinds {
                    let spans = self.config.get_normal_mode_help_spans(
                        self.undo_stack.can_undo(),
                        self.undo_stack.can_redo(),
                    );
                    lines.push(Line::from(spans));
                }
                if lines.is_empty() {
                    lines.push(Line::from(Span::raw("")));
                }
                let footer = Paragraph::new(lines)
                    .style(Style::default().fg(self.config.ui_colors.default_fg));
                frame.render_widget(footer, area);
            }
            AppMode::RecurrencePreview(state) => {
                let summary = if state.draft.occurrences.is_empty() {
                    format!(
                        "Recurrence: {} · No additional occurrences generated. Enter=Confirm, Esc=Cancel.",
                        state.draft.description
                    )
                } else {
                    format!(
                        "Recurrence: {} · Previewing {} future occurrences. Enter=Confirm, Esc=Cancel.",
                        state.draft.description,
                        state.draft.occurrences.len()
                    )
                };

                let combined = if let Some(message) = &self.status_message {
                    if message.is_empty() {
                        summary.clone()
                    } else {
                        format!("{} — {}", summary, message)
                    }
                } else {
                    summary.clone()
                };

                let footer = Paragraph::new(vec![Line::from(vec![Span::styled(
                    combined,
                    Style::default().fg(self.config.ui_colors.default_fg),
                )])])
                .style(
                    Style::default()
                        .fg(self.config.ui_colors.default_fg)
                        .bg(self.config.ui_colors.default_bg),
                );
                frame.render_widget(footer, area);
            }
            AppMode::TaskEdit(_) => {
                let spans = self.config.get_edit_mode_help_spans();
                let help_text = vec![Line::from(spans)];
                let footer = Paragraph::new(help_text)
                    .style(Style::default().fg(self.config.ui_colors.default_fg));
                frame.render_widget(footer, area);
            }
        }
    }
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let terminal = ratatui::init();
    let app = App::new();
    let result = app.run(terminal);
    ratatui::restore();
    result
}
