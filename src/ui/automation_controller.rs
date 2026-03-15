use std::cell::RefCell;
use std::rc::{Rc, Weak};

use crate::ui::automation_state::{
    UiAutomationAction, UiAutomationActionResult, UiAutomationDescription, UiAutomationSnapshot,
};
use crate::ui::window::KnotWindow;

thread_local! {
    static AUTOMATION_WINDOW: RefCell<Option<Weak<KnotWindow>>> = const { RefCell::new(None) };
}

#[derive(Clone, Copy)]
pub struct UiAutomationApi {
    pub describe: fn() -> Option<UiAutomationDescription>,
    pub snapshot: fn() -> Option<UiAutomationSnapshot>,
    pub dispatch: fn(UiAutomationAction) -> Option<UiAutomationActionResult>,
}

pub fn register_window(window: &Rc<KnotWindow>) {
    AUTOMATION_WINDOW.with(|slot| {
        *slot.borrow_mut() = Some(Rc::downgrade(window));
    });
}

#[cfg(test)]
pub fn clear_window() {
    AUTOMATION_WINDOW.with(|slot| {
        *slot.borrow_mut() = None;
    });
}

fn with_window<T>(f: impl FnOnce(&KnotWindow) -> T) -> Option<T> {
    AUTOMATION_WINDOW.with(|slot| {
        let window = slot.borrow().as_ref()?.upgrade()?;
        Some(f(window.as_ref()))
    })
}

pub fn describe_ui_automation() -> Option<UiAutomationDescription> {
    with_window(KnotWindow::describe_ui_automation)
}

pub fn get_ui_snapshot() -> Option<UiAutomationSnapshot> {
    with_window(KnotWindow::ui_automation_snapshot)
}

pub fn dispatch_ui_action(action: UiAutomationAction) -> Option<UiAutomationActionResult> {
    with_window(|window| window.dispatch_ui_automation_action(action))
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
        clear_window();

        assert!(describe_ui_automation().is_none());
        assert!(get_ui_snapshot().is_none());
        assert!(dispatch_ui_action(UiAutomationAction::FocusSearch).is_none());
    }
}
