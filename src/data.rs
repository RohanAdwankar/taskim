use crate::task::TaskData;
use std::fs;
use std::path::Path;

const DATA_FILE: &str = "task_manager_data.json";

pub fn load_data() -> TaskData {
    if Path::new(DATA_FILE).exists() {
        match fs::read_to_string(DATA_FILE) {
            Ok(content) => match serde_json::from_str(&content) {
                Ok(data) => data,
                Err(e) => {
                    eprintln!("Error parsing data file: {}", e);
                    TaskData::default()
                }
            },
            Err(e) => {
                eprintln!("Error reading data file: {}", e);
                TaskData::default()
            }
        }
    } else {
        TaskData::default()
    }
}

pub fn save_data(data: &TaskData) -> Result<(), color_eyre::eyre::Error> {
    let content = serde_json::to_string_pretty(data)?;
    fs::write(DATA_FILE, content)?;
    Ok(())
}
