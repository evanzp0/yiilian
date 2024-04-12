use yiilian_core::common::working_dir::WorkingDir;

pub struct AppState {
    pub working_dir: WorkingDir,
}

impl AppState {
    pub fn new(working_dir: WorkingDir) -> Self {
        AppState { working_dir }
    }
}