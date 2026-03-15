//! Helpers for running daemon work off the GTK thread and delivering results back safely.

use glib::ControlFlow;
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

type Job = Box<dyn FnOnce() + Send + 'static>;

fn spawn_background_job(job: Job) {
    if let Some(runtime) = crate::BACKGROUND_RUNTIME.get() {
        std::mem::drop(runtime.spawn_blocking(job));
    } else {
        std::thread::spawn(job);
    }
}

pub struct BackgroundResult<T, E> {
    receiver: Arc<Mutex<mpsc::Receiver<Result<T, E>>>>,
}

impl<T, E> BackgroundResult<T, E>
where
    T: Send + 'static,
    E: Send + 'static + From<String>,
{
    fn new(receiver: mpsc::Receiver<Result<T, E>>) -> Self {
        Self {
            receiver: Arc::new(Mutex::new(receiver)),
        }
    }

    pub fn attach_local<F>(self, ui: F)
    where
        F: FnOnce(Result<T, E>) + 'static,
    {
        let receiver = Arc::clone(&self.receiver);
        let mut ui = Some(ui);

        glib::timeout_add_local(Duration::from_millis(10), move || {
            let next = receiver
                .lock()
                .expect("background receiver lock should not be poisoned")
                .try_recv();

            match next {
                Ok(result) => {
                    if let Some(ui) = ui.take() {
                        ui(result);
                    }
                    ControlFlow::Break
                }
                Err(mpsc::TryRecvError::Empty) => ControlFlow::Continue,
                Err(mpsc::TryRecvError::Disconnected) => {
                    if let Some(ui) = ui.take() {
                        ui(Err(E::from(
                            "background task ended before delivering a result".to_string(),
                        )));
                    }
                    ControlFlow::Break
                }
            }
        });
    }
}

pub fn run_background<T, E, Work>(work: Work) -> BackgroundResult<T, E>
where
    T: Send + 'static,
    E: Send + 'static + From<String>,
    Work: FnOnce() -> Result<T, E> + Send + 'static,
{
    let (sender, receiver) = mpsc::channel();

    spawn_background_job(Box::new(move || {
        let _ = sender.send(work());
    }));

    BackgroundResult::new(receiver)
}

#[cfg(test)]
mod tests {
    use super::run_background;
    use std::sync::{mpsc, Mutex, OnceLock};
    use std::time::{Duration, Instant};

    fn default_main_context_test_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn background_result_is_forwarded_back_to_ui_handler() {
        let _lock = default_main_context_test_lock()
            .lock()
            .expect("default main context test lock should not be poisoned");
        let context = glib::MainContext::default();
        let _guard = context
            .acquire()
            .expect("main context should be acquirable");
        let main_thread = std::thread::current().id();
        let (tx, rx) = mpsc::channel();

        run_background(move || Ok::<_, String>(41_u32)).attach_local(move |result| {
            tx.send((std::thread::current().id(), result.map(|value| value + 1)))
                .expect("ui callback should send");
        });

        let deadline = Instant::now() + Duration::from_secs(1);
        loop {
            if let Ok((callback_thread, received)) = rx.try_recv() {
                assert_eq!(callback_thread, main_thread);
                assert_eq!(received.expect("result should be ok"), 42);
                break;
            }

            assert!(
                Instant::now() < deadline,
                "timed out waiting for UI callback"
            );
            context.iteration(true);
        }
    }

    #[test]
    fn background_disconnect_invokes_ui_with_error() {
        let _lock = default_main_context_test_lock()
            .lock()
            .expect("default main context test lock should not be poisoned");
        let context = glib::MainContext::default();
        let _guard = context
            .acquire()
            .expect("main context should be acquirable");
        let (tx, rx) = mpsc::channel();

        let (sender, receiver) = mpsc::channel::<Result<u32, String>>();
        drop(sender);

        super::BackgroundResult::new(receiver).attach_local(move |result| {
            tx.send(result).expect("ui callback should send");
        });

        let deadline = Instant::now() + Duration::from_secs(1);
        loop {
            if let Ok(result) = rx.try_recv() {
                assert_eq!(
                    result.expect_err("disconnect should surface an error"),
                    "background task ended before delivering a result".to_string()
                );
                break;
            }

            assert!(
                Instant::now() < deadline,
                "timed out waiting for disconnect callback"
            );
            context.iteration(true);
        }
    }
}
