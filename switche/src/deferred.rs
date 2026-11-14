
//! Deferral helpers .. deferred actions driven by a (pushable/resettable) deadline
//!
//! Both variants share the same semantics:
//! - a setup pushes the deadline to `now + delay` (so repeated setups postpone the action)
//! - when the deadline is due, only the last set action is executed
//! - resetting a pending deadline cancels the pending action
//!
//! They differ only in cost under load, so pick per usage:
//! - `DeferredExecutor`   .. spawns a short-lived sleeping thread per setup
//!   .. simple and perfectly fine for low-frequency setups (e.g. a config write-back on a user toggle)
//! - `CoalescingExecutor` .. keeps a single (lazily started) worker thread parked on a condvar,
//!   .. and setups are just a state update + notify .. use this where setups can be high-frequency
//!   .. (e.g. one per window-move event during a drag), where the former would spawn hundreds of
//!   .. threads that all just wake up and find they aren't the latest setup

use std::sync::{Arc, Condvar, Mutex};
use std::time::{self, SystemTime};


/// Action type to pass in to a `DeferredExecutor` or `CoalescingExecutor`
pub type Action = Arc < dyn Fn() + Send + Sync + 'static >;





// -------------------------------------------------------------------------------------------------
//  thread-per-setup variant
// -------------------------------------------------------------------------------------------------

#[derive (Debug, Clone)]
/// Deferred Executor sets up a deferred action until a (pushable/resettable) deadline has passed<br>
/// note: each setup spawns a thread that sleeps until its own deadline, so this is best suited for
/// .. low-frequency setups .. use `CoalescingExecutor` for high-frequency ones
pub struct DeferredExecutor {
    deadline : Arc <Mutex <SystemTime>>,
}


impl Default for DeferredExecutor {
    fn default () -> Self { Self::new() }
}


impl DeferredExecutor {

    pub fn new () -> Self {
        Self { deadline: Arc::new (Mutex::new (SystemTime::UNIX_EPOCH)) }
    }

    pub fn reset (&self) {
        *self.deadline.lock().unwrap() = SystemTime::UNIX_EPOCH;
    }

    pub fn set_deferral_dur (&self, dur:time::Duration) {
        if let Some(t) = SystemTime::now().checked_add(dur) {
            *self.deadline.lock().unwrap() = t
        }
    }

    pub fn add_deferral_dur (&self, dur:time::Duration) {
        let mut deadline = self.deadline.lock().unwrap();
        if let Some(t) = deadline.checked_add(dur) {
            *deadline = t
        }
    }

    pub fn is_reset (&self) -> bool {
        *self.deadline.lock().unwrap() == SystemTime::UNIX_EPOCH
    }

    pub fn is_due (&self) -> bool {
        let deadline = *self.deadline.lock().unwrap();
        deadline != SystemTime::UNIX_EPOCH && deadline < SystemTime::now()
    }

    fn check_defered_action (&self, action:Action) {
        // we'll check-and-reset atomically, (same logic as using reset/is_reset/is_due above, but under same lock)
        {
            let mut deadline = self.deadline.lock().unwrap();
            if *deadline == SystemTime::UNIX_EPOCH || *deadline >= SystemTime::now() {
                // ^^ deadline was reset, or pushed forward (so its not due yet)
                return
            }
            *deadline = SystemTime::UNIX_EPOCH;
        }
        action();
    }

    /// Sets up a deferred action which will only trigger after a deferrable deadline has passed.<br>
    /// Any subsequent calls to setup deferred action on this deadline will postpone this deadline (if pending).<br>
    /// When the deadline triggers, (only) the last set action will be executed.<br>
    /// Resetting a pending deadline effectively cancels any pending actions
    pub fn setup_deferred_action (&self, action:Action, delay:time::Duration) {
        // we simply set up a deadline and a thread to wake up and check
        self.set_deferral_dur(delay);
        let dfr_ex = self.clone();
        std::thread::spawn ( move || {
            std::thread::sleep (delay);
            dfr_ex.check_defered_action(action);
        } );
    }

}





// -------------------------------------------------------------------------------------------------
//  single-worker variant
// -------------------------------------------------------------------------------------------------

#[derive (Clone)]
/// Coalescing Executor has the same deadline-pushback / last-action-wins semantics as
/// `DeferredExecutor`, but instead of spawning a thread per setup it keeps a single (lazily started)
/// worker thread parked on a condvar .. setups only update the pending state and notify<br>
/// use this where setups can be high-frequency (e.g. window-move events during a drag)
pub struct CoalescingExecutor {
    inner : Arc <CoalescingExecutorInner>,
}
struct CoalescingExecutorInner {
    state : Mutex <CoalescingExecutorState>,
    cv    : Condvar,
}
struct CoalescingExecutorState {
    deadline : SystemTime,        // (UNIX_EPOCH means nothing is pending)
    action   : Option <Action>,   // (only the last set action runs when the deadline is due)
    worker   : bool,              // (whether the worker thread has been started)
}


impl std::fmt::Debug for CoalescingExecutor {
    fn fmt (&self, f:&mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let st = self.inner.state.lock().unwrap();
        f.debug_struct ("CoalescingExecutor")
            .field ("deadline", &st.deadline)
            .field ("has_action", &st.action.is_some())
            .finish()
    }
}

impl Default for CoalescingExecutor {
    fn default () -> Self { Self::new() }
}


impl CoalescingExecutor {

    pub fn new () -> Self {
        Self { inner: Arc::new (CoalescingExecutorInner {
            state : Mutex::new (CoalescingExecutorState {
                deadline : SystemTime::UNIX_EPOCH,
                action   : None,
                worker   : false,
            }),
            cv : Condvar::new(),
        })}
    }

    pub fn reset (&self) {
        let mut st = self.inner.state.lock().unwrap();
        st.deadline = SystemTime::UNIX_EPOCH;
        st.action = None;
        drop(st);
        self.inner.cv.notify_all();
    }

    pub fn set_deferral_dur (&self, dur:time::Duration) {
        if let Some(t) = SystemTime::now().checked_add(dur) {
            let mut st = self.inner.state.lock().unwrap();
            st.deadline = t;
            drop(st);
            self.inner.cv.notify_all();
        }
    }

    pub fn add_deferral_dur (&self, dur:time::Duration) {
        let mut st = self.inner.state.lock().unwrap();
        if let Some(t) = st.deadline.checked_add(dur) {
            st.deadline = t;
            drop(st);
            self.inner.cv.notify_all();
        }
    }

    pub fn is_reset (&self) -> bool {
        self.inner.state.lock().unwrap() .deadline == SystemTime::UNIX_EPOCH
    }

    pub fn is_due (&self) -> bool {
        let st = self.inner.state.lock().unwrap();
        st.deadline != SystemTime::UNIX_EPOCH && st.deadline < SystemTime::now()
    }

    /// Sets up a deferred action which will only trigger after a deferrable deadline has passed.<br>
    /// Any subsequent calls to setup deferred action on this deadline will postpone this deadline (if pending).<br>
    /// When the deadline triggers, (only) the last set action will be executed.<br>
    /// Resetting a pending deadline effectively cancels any pending actions
    pub fn setup_deferred_action (&self, action:Action, delay:time::Duration) {
        let Some (deadline) = SystemTime::now().checked_add(delay) else { return };
        let mut st = self.inner.state.lock().unwrap();
        st.deadline = deadline;
        st.action = Some (action);
        let start_worker = !st.worker;
        st.worker = true;
        drop(st);
        if start_worker {
            let inner = self.inner.clone();
            let _ = std::thread::Builder::new() .name ("coalescing-executor".into())
                .spawn ( move || Self::run_worker (inner) );
        }
        self.inner.cv.notify_all();
    }

    /// the worker loop .. waits (without needing a thread per setup) until a pending deadline is due,
    /// .. then atomically consumes it and runs the last set action
    fn run_worker (inner: Arc <CoalescingExecutorInner>) {
        let mut st = inner.state.lock().unwrap();
        loop {
            // nothing pending .. wait for the next setup
            if st.deadline == SystemTime::UNIX_EPOCH {
                st = inner.cv.wait(st).unwrap();
                continue
            }
            let now = SystemTime::now();
            if st.deadline >= now {
                // not due yet (or pushed forward since we last looked) .. wait for the remainder
                let remaining = st.deadline.duration_since(now).unwrap_or_default();
                st = inner.cv.wait_timeout(st, remaining).unwrap().0;
                continue
            }
            // check-and-reset atomically, and take the (last set) action to run outside the lock
            st.deadline = SystemTime::UNIX_EPOCH;
            let action = st.action.take();
            drop(st);
            if let Some(action) = action { action() }
            st = inner.state.lock().unwrap();
        }
    }

}

