//! Minimal request-state model for async UI flows.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequestState<T, E> {
    Idle,
    Loading,
    Success(T),
    Error(E),
}

impl<T, E> RequestState<T, E> {
    pub fn idle() -> Self {
        Self::Idle
    }

    pub fn loading() -> Self {
        Self::Loading
    }

    pub fn success(value: T) -> Self {
        Self::Success(value)
    }

    pub fn error(error: E) -> Self {
        Self::Error(error)
    }
}

#[cfg(test)]
mod tests {
    use super::RequestState;

    #[test]
    fn request_state_builds_loading_and_idle_variants() {
        assert_eq!(RequestState::<u32, String>::idle(), RequestState::Idle);
        assert_eq!(
            RequestState::<u32, String>::loading(),
            RequestState::Loading
        );
    }

    #[test]
    fn request_state_wraps_success_values() {
        assert_eq!(
            RequestState::<u32, String>::success(42),
            RequestState::Success(42)
        );
    }

    #[test]
    fn request_state_wraps_error_values() {
        assert_eq!(
            RequestState::<u32, String>::error("daemon failed".to_string()),
            RequestState::Error("daemon failed".to_string())
        );
    }
}
