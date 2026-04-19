use std::collections::HashMap;

pub struct CommandInfo {
    pub description: &'static str,
    pub exec: fn(&mut crate::App, &str) -> Result<(), String>,
}

pub fn get_command_registry() -> HashMap<&'static str, CommandInfo> {
    let mut map = HashMap::new();
    // Quit commands
    map.insert(
        "q",
        CommandInfo {
            description: "Quit the application.",
            exec: |app, _| {
                app.should_exit = true;
                Ok(())
            },
        },
    );
    map.insert(
        "quit",
        CommandInfo {
            description: "Quit the application.",
            exec: |app, _| {
                app.should_exit = true;
                Ok(())
            },
        },
    );
    map.insert(
        "wq",
        CommandInfo {
            description: "Quit the application.",
            exec: |app, _| {
                app.should_exit = true;
                Ok(())
            },
        },
    );
    map.insert(
        "x",
        CommandInfo {
            description: "Quit the application.",
            exec: |app, _| {
                app.should_exit = true;
                Ok(())
            },
        },
    );
    // Help command is handled specially in main.rs
    map.insert(
        "seekeys",
        CommandInfo {
            description: "Show keybindings bar.",
            exec: |app, _| {
                app.show_keybinds = true;
                Ok(())
            },
        },
    );
    map.insert(
        "set seekeys",
        CommandInfo {
            description: "Show keybindings bar.",
            exec: |app, _| {
                app.show_keybinds = true;
                Ok(())
            },
        },
    );
    map.insert(
        "nokeys",
        CommandInfo {
            description: "Hide keybindings bar.",
            exec: |app, _| {
                app.show_keybinds = false;
                Ok(())
            },
        },
    );
    map.insert(
        "set nokeys",
        CommandInfo {
            description: "Hide keybindings bar.",
            exec: |app, _| {
                app.show_keybinds = false;
                Ok(())
            },
        },
    );
    map.insert(
        "showpreview",
        CommandInfo {
            description: "Show the preview sidebar and save it to config.",
            exec: |app, _| app.set_preview_preference(true),
        },
    );
    map.insert(
        "set showpreview",
        CommandInfo {
            description: "Show the preview sidebar and save it to config.",
            exec: |app, _| app.set_preview_preference(true),
        },
    );
    map.insert(
        "hidepreview",
        CommandInfo {
            description: "Hide the preview sidebar and save it to config.",
            exec: |app, _| app.set_preview_preference(false),
        },
    );
    map.insert(
        "set hidepreview",
        CommandInfo {
            description: "Hide the preview sidebar and save it to config.",
            exec: |app, _| app.set_preview_preference(false),
        },
    );
    map.insert(
        "wrap",
        CommandInfo {
            description: "Enable UI text wrapping.",
            exec: |app, _| {
                app.month_view.set_wrap(true);
                Ok(())
            },
        },
    );
    map.insert(
        "set wrap",
        CommandInfo {
            description: "Enable UI text wrapping.",
            exec: |app, _| {
                app.month_view.set_wrap(true);
                Ok(())
            },
        },
    );
    map.insert(
        "nowrap",
        CommandInfo {
            description: "Disable UI text wrapping.",
            exec: |app, _| {
                app.month_view.set_wrap(false);
                Ok(())
            },
        },
    );
    map.insert(
        "set nowrap",
        CommandInfo {
            description: "Disable UI text wrapping.",
            exec: |app, _| {
                app.month_view.set_wrap(false);
                Ok(())
            },
        },
    );
    // Date navigation commands are handled in main.rs
    map.insert(
        "YYYY",
        CommandInfo {
            description: "Jump to a specific year (e.g., :2025).",
            exec: |_, _| Ok(()), // Handled in main.rs parse_date_command
        },
    );
    map.insert(
        "MM/DD/YYYY",
        CommandInfo {
            description: "Jump to a specific date (e.g., :06/15/2025).",
            exec: |_, _| Ok(()), // Handled in main.rs parse_date_command
        },
    );
    map.insert(
        "YYYY-MM-DD",
        CommandInfo {
            description: "Jump to a specific date (e.g., :2025-06-15).",
            exec: |_, _| Ok(()), // Handled in main.rs parse_date_command
        },
    );
    map.insert(
        "DD",
        CommandInfo {
            description: "Jump to a specific day in the current month (e.g., :15).",
            exec: |_, _| Ok(()), // Handled in main.rs parse_date_command
        },
    );
    map
}
