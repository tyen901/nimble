use std::path::PathBuf;
use crate::gui::widgets::{StatusDisplay, PathPicker};

pub struct GenSrfPanelState {
    pub input_path: PathPicker,
    pub output_path: PathPicker,
    pub status: StatusDisplay,
    pub output_dir: Option<PathBuf>,
}

impl Default for GenSrfPanelState {
    fn default() -> Self {
        Self {
            input_path: PathPicker::new("Input Path:", "Select Input Directory"),
            output_path: PathPicker::new("Output Path (optional):", "Select Output Directory"),
            status: StatusDisplay::default(),
            output_dir: None,
        }
    }
}
