use slint::Weak;

use crate::AppWindow;

pub trait Notifier: Send + Sync {
    fn notify(&self);
}

pub struct HoldAppWeak {
    app_weak: Weak<AppWindow>,
}

impl HoldAppWeak {
    pub fn new(app_weak: Weak<AppWindow>) -> Self {
        Self { app_weak }
    }
}

impl Notifier for HoldAppWeak {
    fn notify(&self) {
        if let Some(app) = self.app_weak.upgrade() {
            app.invoke_openGraphWindow();
        }
    }
}
