# Taskim

![TUI Demo](demo.gif)

Taskim is a terminal-based task manager built with Rust and [ratatui](https://github.com/ratatui-org/ratatui). It provides a Vim-inspired interface for managing tasks, navigating months, and customizing your workflow.

## Features

- **Monthly Calendar View:**  
  Visualize your tasks in a month grid, with navigation for days, weeks, months, and years.
- **Sequential List View:**  
  View all tasks in chronological order. Press `Tab` to switch between Calendar and List views.
  - In List view, press `Enter` to show task details in a side panel
  - Navigate with `j`/`k` or arrow keys
  - Press `i` to edit the selected task
  - All standard keybindings work in both views
- **Task Management:**  
  - Add, edit, and delete tasks for any date.
  - Tasks can have titles and optional content/comments.
  - Mark tasks as complete/incomplete.
  - Reorder tasks within a day.
- **Vim-style Keybindings:**  
  - Navigate with `h`, `j`, `k`, `l` or arrow keys.
  - Insert tasks above/below (`O`/`o`), delete (`dd`/`x`), yank/copy (`y`), paste (`p`/`P`), and undo/redo (`u/control-r`).
  - Command mode (`:`) for advanced actions (e.g., go to date, toggle wrap, show/hide keybinds).
  - Search tasks with `/` and move through matches with `n`/`N`. 
  - Switch views with `Tab` (Calendar ↔ List).
- **Recurring Tasks:**  
  Preview upcoming occurrences before committing, spawn follow-up tasks automatically on completion, or generate entire series upfront.
- **Scramble Mode:**  
  Toggle (`s`) to obscure task names for privacy.
- **Customizable UI:**  
  - Colors and keybindings are configurable via `config.yml`.
  - Toggle keybind help bar and UI wrap mode.

## Getting Started

1. **Build and Run:**
   For this you can either clone the repo and use:
   ```sh
   cargo run --release
   ```
   Or, you can run
   ```
   cargo install taskim
   taskim
   ```
3. **Configuration:**
   - Copy or edit config.yml in the project root to customize appearance and controls.
4. **Exit**
   - Quit with `q` or command mode `:wq`

## Motivation / Next Steps
The goal of this TUI was to replicate the features of the previous [task manager](https://github.com/RohanAdwankar/task-js) I have been using but be fully usable without a mouse using VIM motions.

At this point, the TUI is usable for me, but if there is some feature you would like to see, please let me know! (open an issue or PR)

### Command Mode (`:`) Reference

- `:q`, `:quit`, `:wq`, `:x` 
  Quit the application.

- `:help`, `:help <command>`
  Show help for command mode.

- `:seekeys`, `:set seekeys`  
  Show keybindings bar.

- `:nokeys`, `:set nokeys`  
  Hide keybindings bar.

- `:wrap`, `:set wrap`  
  Enable UI text wrapping.

- `:nowrap`, `:set nowrap`  
  Disable UI text wrapping.

- `:r/<pattern>`  
  Preview and apply recurrence patterns for the selected task (see “Recurring Tasks” for examples). Use `:r/clear` to remove recurrence metadata.

- `:MM/DD/YYYY`, `:YYYY-MM-DD`, `:DD`, `:YYYY`
  Jump to a specific date in the calendar.

## Recurring Tasks Reference

- **Quick motions:** With a task selected in normal mode, press `r` followed by `d`, `w`, `m`, or `y` to preview daily, weekly, monthly, or yearly recurrences. The calendar temporarily shows every upcoming instance and prompts you to `<Enter>` to confirm or `<Esc>` to cancel.
- **Advanced patterns:** Command mode supports `:r/<pattern>` where patterns can mix weekday letters (`mtwrfsu`), month-day lists (`1,15`), optional occurrence limits, and the `/a` suffix to create the full series immediately. Examples:
  - `:r/mtwfr` – recur on weekdays.
  - `:r/1,15/mtwrf` – recur on the 1st and 15th that fall on weekdays.
  - `:r/su/30` – recur on weekends for 30 total occurrences.
  - `:r/mtwrfsu/10/a` – create ten daily tasks upfront.
- **Lifecycle:** New occurrences appear automatically when you complete a recurring task (unless you chose `/a`). Deleting any task in the chain stops future spawning, and recurring metadata can be cleared with `:r/clear`.

## List View

The List View provides a sequential overview of all your tasks sorted chronologically. This view is ideal for seeing your entire task list at a glance and focusing on task details.

### How to Use List View

1. **Switch to List View:** Press `Tab` from the Calendar view (or press `Tab` again to return to Calendar)
2. **Navigate:** Use `j`/`k` or arrow keys to move up and down through tasks
3. **View Details:** Press `Enter` to toggle a detail panel on the right side showing:
   - Task title
   - Date and time
   - Completion status
   - Full description (multi-line supported)
4. **Edit Task:** Press `i` to edit the selected task (opens the task editor)
5. **Toggle Completion:** Press `Space` to mark task as complete/incomplete
6. **Delete Task:** Press `x` to delete the selected task
7. **Hide Details:** Press `Esc` to close the detail panel
8. **Other Features:** All standard keybindings (yank, paste, undo, search, etc.) work in List view

### List View Layout

When details are shown (after pressing `Enter`):
- **Left Panel (40%):** Scrollable list of all tasks with date and completion status
- **Right Panel (60%):** Detailed view of the selected task with edit instructions

### Config Reference
- For the color customization options outside of the named colors, I use the Ratatui indexed colors. You can see how the numbers correspond to the colors [here](https://github.com/ratatui/ratatui/blob/main/examples/README.md#color-explorer).
