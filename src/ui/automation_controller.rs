use std::cell::RefCell;
use std::rc::{Rc, Weak};

use crate::ui::automation_state::{
    UiAutomationAction, UiAutomationActionResult, UiAutomationDescription, UiAutomationSnapshot,
};
use crate::ui::window::KnotWindow;

thread_local! {
    static AUTOMATION_REGISTRATION: RefCell<Option<AutomationRegistration>> = const { RefCell::new(None) };
}

#[derive(Clone, Copy)]
pub struct UiAutomationApi {
    pub describe: fn() -> Option<UiAutomationDescription>,
    pub snapshot: fn() -> Option<UiAutomationSnapshot>,
    pub dispatch: fn(UiAutomationAction) -> Option<UiAutomationActionResult>,
}

#[derive(Clone)]
enum AutomationRegistration {
    Window(Weak<KnotWindow>),
    Api(UiAutomationApi),
}

pub fn register_window(window: &Rc<KnotWindow>) {
    AUTOMATION_REGISTRATION.with(|slot| {
        *slot.borrow_mut() = Some(AutomationRegistration::Window(Rc::downgrade(window)));
    });
}

pub fn register_protocol_api(api: UiAutomationApi) {
    AUTOMATION_REGISTRATION.with(|slot| {
        *slot.borrow_mut() = Some(AutomationRegistration::Api(api));
    });
}

pub fn clear_registration() {
    AUTOMATION_REGISTRATION.with(|slot| {
        *slot.borrow_mut() = None;
    });
}

fn with_window<T>(f: impl FnOnce(&KnotWindow) -> T) -> Option<T> {
    AUTOMATION_REGISTRATION.with(|slot| {
        let registration = slot.borrow();
        let AutomationRegistration::Window(window) = registration.as_ref()? else {
            return None;
        };
        let window = window.upgrade()?;
        Some(f(window.as_ref()))
    })
}

pub fn describe_ui_automation() -> Option<UiAutomationDescription> {
    let registration = AUTOMATION_REGISTRATION.with(|slot| slot.borrow().clone());
    match registration? {
        AutomationRegistration::Window(_) => with_window(KnotWindow::describe_ui_automation),
        AutomationRegistration::Api(api) => (api.describe)(),
    }
}

pub fn get_ui_snapshot() -> Option<UiAutomationSnapshot> {
    let registration = AUTOMATION_REGISTRATION.with(|slot| slot.borrow().clone());
    match registration? {
        AutomationRegistration::Window(_) => with_window(KnotWindow::ui_automation_snapshot),
        AutomationRegistration::Api(api) => (api.snapshot)(),
    }
}

pub fn dispatch_ui_action(action: UiAutomationAction) -> Option<UiAutomationActionResult> {
    let registration = AUTOMATION_REGISTRATION.with(|slot| slot.borrow().clone());
    match registration? {
        AutomationRegistration::Window(_) => {
            with_window(|window| window.dispatch_ui_automation_action(action))
        }
        AutomationRegistration::Api(api) => (api.dispatch)(action),
    }
}

pub fn protocol_api() -> UiAutomationApi {
    UiAutomationApi {
        describe: describe_ui_automation,
        snapshot: get_ui_snapshot,
        dispatch: dispatch_ui_action,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::automation_state::UiAutomationAction;

    #[test]
    fn controller_returns_none_without_registered_window() {
        clear_registration();

        assert!(describe_ui_automation().is_none());
        assert!(get_ui_snapshot().is_none());
        assert!(dispatch_ui_action(UiAutomationAction::FocusSearch).is_none());
    }
}
