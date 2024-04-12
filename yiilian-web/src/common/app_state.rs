use once_cell::sync::OnceCell;
use tera::Tera;
use yiilian_core::common::working_dir::WorkingDir;

pub static mut APP_STATE: OnceCell<AppState> = OnceCell::new();

#[derive(Debug)]
pub struct AppState {
    pub working_dir: WorkingDir,
    pub tera: Tera,
}

impl AppState {
    pub fn new(working_dir: WorkingDir, tera: Tera) -> Self {
        AppState { working_dir, tera }
    }

    pub fn working_dir(&self) -> &WorkingDir {
        &self.working_dir
    }

    pub fn tera(&self) -> &Tera {
        &self.tera
    }
}

pub fn app_state() -> &'static AppState {
    let app_state = unsafe { APP_STATE.get().unwrap() };

    app_state
}

pub fn app_state_mut() -> &'static mut  AppState {
    let app_state = unsafe { APP_STATE.get_mut().unwrap() };

    app_state
}

pub fn init_app_state(app_state: AppState) {
    unsafe {
        APP_STATE.set(app_state).unwrap();
    }
}