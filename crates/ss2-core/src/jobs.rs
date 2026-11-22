//! Job system and thread policy (T03-018).
//!
//! The engine has a few long-lived threads with a fixed role (main, render, audio, network, IO)
//! and a pool of worker threads for short CPU jobs (decoding assets, skinning, culling). This
//! module decides how many workers to start ([`ThreadPolicy`]: a pure function of the machine
//! and the roles that already have a thread) and runs the jobs ([`JobPool`]): fixed, named
//! threads, a shared queue, a result handle per job, and a panic inside a job comes back as an
//! error instead of taking the pool down.
//!
//! **Determinism:** jobs run in any order on any thread. The simulation (`ss2-sim`,
//! `ss2-physics`, `ss2-net`) must not depend on job timing; use the pool for presentation and
//! loading work, or give each job a disjoint input and join in a fixed order.
//!
//! **Core hints:** [`ThreadRole::core_hint`] says which kind of core a role wants on a
//! big.LITTLE machine. It is only a hint (data): pinning needs OS calls that this crate does
//! not make (`unsafe_code` is denied), so the platform layer applies it where the OS allows.
//!
//! **Panics:** a job that panics is reported through its [`JobHandle`] as
//! [`JobError::Panicked`]; the worker carries on. The process-wide panic hook (crash reports,
//! `crate::crash`) still runs for it, so a recovered job panic also leaves a report.

use std::any::Any;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::JoinHandle;

use crate::crash::payload_message;
use crate::{Error, Result};

/// The environment variable that overrides the worker count (`SS2_WORKERS=4`).
pub const WORKER_ENV: &str = "SS2_WORKERS";

/// The most workers ever started, whatever the machine offers.
pub const MAX_WORKERS: usize = 32;

/// A thread with a fixed job in the engine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThreadRole {
    /// The thread that owns the window and runs the frame loop.
    Main,
    /// Command recording and submission.
    Render,
    /// The audio mixer (a missed deadline is audible).
    Audio,
    /// Socket IO (mostly asleep).
    Network,
    /// File and archive reads (mostly asleep).
    Io,
    /// A [`JobPool`] worker.
    Worker,
}

/// Which kind of core a thread would like on a machine with fast and slow cores.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoreHint {
    /// A fast core: latency matters.
    Performance,
    /// A slow, frugal core is fine.
    Efficiency,
    /// No preference.
    Any,
}

impl ThreadRole {
    /// The roles that keep a core busy most of the time besides the workers.
    const BUSY: [Self; 3] = [Self::Main, Self::Render, Self::Audio];

    /// The prefix of the OS thread name (shown by debuggers and profilers). Workers add `-<n>`.
    pub fn thread_name(self) -> &'static str {
        match self {
            Self::Main => "ss2-main",
            Self::Render => "ss2-render",
            Self::Audio => "ss2-audio",
            Self::Network => "ss2-net",
            Self::Io => "ss2-io",
            Self::Worker => "ss2-worker",
        }
    }

    /// The core kind this role wants (a hint, see the module documentation).
    pub fn core_hint(self) -> CoreHint {
        match self {
            Self::Main | Self::Render | Self::Audio => CoreHint::Performance,
            Self::Network | Self::Io => CoreHint::Efficiency,
            Self::Worker => CoreHint::Any,
        }
    }

    /// How many threads of the other roles compete with the workers for cores.
    pub fn reserved_count() -> usize {
        Self::BUSY.len()
    }
}

/// How many workers to start.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThreadPolicy {
    /// Logical cores of the machine (`0` when unknown).
    pub available_cores: usize,
    /// Cores taken by the threads of the other roles.
    pub reserved: usize,
    /// A count asked for by the user, which wins over the computed one.
    pub requested: Option<usize>,
    /// The hard upper limit.
    pub max_workers: usize,
}

impl ThreadPolicy {
    /// The policy for this machine: its logical cores, `reserved` cores kept for other threads
    /// and the [`WORKER_ENV`] override.
    pub fn detect(reserved: usize) -> Self {
        Self {
            available_cores: std::thread::available_parallelism().map_or(0, usize::from),
            reserved,
            requested: parse_worker_override(std::env::var(WORKER_ENV).ok().as_deref()),
            max_workers: MAX_WORKERS,
        }
    }

    /// The number of workers: the requested count, else the cores minus the reserved ones;
    /// always between one and `max_workers`.
    pub fn worker_count(&self) -> usize {
        let wanted = self
            .requested
            .unwrap_or_else(|| self.available_cores.saturating_sub(self.reserved));
        wanted.clamp(1, self.max_workers.max(1))
    }
}

/// The worker count in the text of an environment variable: a whole number, spaces allowed.
/// Anything else is ignored (`None`).
pub fn parse_worker_override(text: Option<&str>) -> Option<usize> {
    text?.trim().parse().ok()
}

/// Why a job did not produce a value.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum JobError {
    /// The job panicked.
    #[error("job {job:?} panicked: {message}")]
    Panicked {
        /// The name the job was spawned with.
        job: &'static str,
        /// The panic message.
        message: String,
    },
    /// The pool was shut down before the job ran.
    #[error("job {0:?} never ran: the pool was shut down")]
    Cancelled(&'static str),
}

/// The result of a job, to be waited for with [`JobHandle::join`]. Dropping the handle does not
/// cancel the job.
pub struct JobHandle<T> {
    name: &'static str,
    receiver: Receiver<std::result::Result<T, JobError>>,
}

impl<T> JobHandle<T> {
    /// Wait for the job to finish and return its value.
    ///
    /// # Errors
    /// [`JobError::Panicked`] if the job panicked, [`JobError::Cancelled`] if it never ran.
    pub fn join(self) -> std::result::Result<T, JobError> {
        self.receiver
            .recv()
            .unwrap_or(Err(JobError::Cancelled(self.name)))
    }
}

type Job = Box<dyn FnOnce() + Send + 'static>;

/// A fixed set of named worker threads sharing one queue. Dropping the pool runs the jobs that
/// are already queued, then stops and joins the workers.
pub struct JobPool {
    sender: Mutex<Option<Sender<Job>>>,
    workers: Vec<JoinHandle<()>>,
}

impl JobPool {
    /// Start `workers` threads named `ss2-worker-0`, `ss2-worker-1`, ...
    ///
    /// # Errors
    /// [`Error::Invalid`] for zero workers or when the OS refuses to start a thread.
    pub fn new(workers: usize) -> Result<Self> {
        if workers == 0 {
            return Err(Error::invalid("job pool", "needs at least one worker"));
        }
        let (sender, receiver) = mpsc::channel::<Job>();
        let receiver = Arc::new(Mutex::new(receiver));
        let mut handles = Vec::with_capacity(workers);
        for index in 0..workers {
            let receiver = Arc::clone(&receiver);
            let spawned = std::thread::Builder::new()
                .name(format!("{}-{index}", ThreadRole::Worker.thread_name()))
                .spawn(move || worker_loop(&receiver));
            match spawned {
                Ok(handle) => handles.push(handle),
                Err(e) => {
                    // Stop the workers that did start before reporting the failure.
                    drop(sender);
                    for handle in handles {
                        let _ = handle.join();
                    }
                    return Err(Error::invalid(
                        "job pool",
                        format!("cannot start a thread: {e}"),
                    ));
                }
            }
        }
        Ok(Self {
            sender: Mutex::new(Some(sender)),
            workers: handles,
        })
    }

    /// A pool sized by [`ThreadPolicy::detect`] for the usual reserved threads.
    ///
    /// # Errors
    /// As [`JobPool::new`].
    pub fn detect() -> Result<Self> {
        Self::new(ThreadPolicy::detect(ThreadRole::reserved_count()).worker_count())
    }

    /// The number of worker threads.
    pub fn worker_count(&self) -> usize {
        self.workers.len()
    }

    /// Queue `job` (named for traces and errors) and return the handle for its result.
    pub fn spawn<T, F>(&self, name: &'static str, job: F) -> JobHandle<T>
    where
        T: Send + 'static,
        F: FnOnce() -> T + Send + 'static,
    {
        let (result_tx, result_rx) = mpsc::channel();
        let run: Job = Box::new(move || {
            let span = tracing::trace_span!("job", name);
            let _entered = span.enter();
            let outcome =
                catch_unwind(AssertUnwindSafe(job)).map_err(|payload| JobError::Panicked {
                    job: name,
                    message: panic_text(payload.as_ref()),
                });
            // The handle may be gone; the result is then simply dropped.
            let _ = result_tx.send(outcome);
        });
        let sender = self.sender.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(sender) = sender.as_ref() {
            // A send only fails when every worker is gone; the handle then reports `Cancelled`.
            let _ = sender.send(run);
        }
        JobHandle {
            name,
            receiver: result_rx,
        }
    }
}

fn panic_text(payload: &(dyn Any + Send)) -> String {
    payload_message(payload)
}

fn worker_loop(receiver: &Mutex<Receiver<Job>>) {
    loop {
        // Hold the lock only while waiting for the next job, never while running it.
        let next = receiver
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .recv();
        match next {
            Ok(job) => job(),
            Err(_) => return, // the pool was dropped and the queue is empty
        }
    }
}

impl Drop for JobPool {
    fn drop(&mut self) {
        // Closing the channel lets the workers drain the queue and then exit.
        self.sender
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::mpsc;
    use std::time::Duration;

    #[test]
    fn workers_are_the_cores_minus_the_reserved_threads_but_at_least_one() {
        let p = |cores, reserved| ThreadPolicy {
            available_cores: cores,
            reserved,
            requested: None,
            max_workers: 64,
        };
        assert_eq!(p(8, 3).worker_count(), 5);
        assert_eq!(p(4, 3).worker_count(), 1);
        assert_eq!(p(2, 3).worker_count(), 1, "never zero");
        assert_eq!(
            p(0, 0).worker_count(),
            1,
            "an unknown core count still works"
        );
    }

    #[test]
    fn the_worker_count_is_capped() {
        let policy = ThreadPolicy {
            available_cores: 128,
            reserved: 3,
            requested: None,
            max_workers: 16,
        };
        assert_eq!(policy.worker_count(), 16);
    }

    #[test]
    fn a_requested_count_wins_within_one_and_the_cap() {
        let mk = |requested| ThreadPolicy {
            available_cores: 8,
            reserved: 3,
            requested,
            max_workers: 16,
        };
        assert_eq!(mk(Some(2)).worker_count(), 2);
        assert_eq!(mk(Some(0)).worker_count(), 1);
        assert_eq!(mk(Some(1000)).worker_count(), 16);
    }

    #[test]
    fn the_override_text_is_parsed_strictly() {
        assert_eq!(parse_worker_override(Some("4")), Some(4));
        assert_eq!(parse_worker_override(Some(" 6 ")), Some(6));
        assert_eq!(parse_worker_override(Some("0")), Some(0));
        assert_eq!(parse_worker_override(Some("four")), None);
        assert_eq!(parse_worker_override(Some("-1")), None);
        assert_eq!(parse_worker_override(Some("")), None);
        assert_eq!(parse_worker_override(None), None);
    }

    #[test]
    fn roles_have_stable_thread_names_and_hints() {
        assert_eq!(ThreadRole::Render.thread_name(), "ss2-render");
        assert_eq!(ThreadRole::Audio.thread_name(), "ss2-audio");
        assert_eq!(ThreadRole::Io.thread_name(), "ss2-io");
        assert_eq!(ThreadRole::Worker.thread_name(), "ss2-worker");
        assert_eq!(ThreadRole::Render.core_hint(), CoreHint::Performance);
        assert_eq!(ThreadRole::Audio.core_hint(), CoreHint::Performance);
        assert_eq!(ThreadRole::Io.core_hint(), CoreHint::Efficiency);
        assert_eq!(ThreadRole::Worker.core_hint(), CoreHint::Any);
    }

    #[test]
    fn the_detected_policy_is_usable() {
        let policy = ThreadPolicy::detect(ThreadRole::reserved_count());
        assert!((1..=policy.max_workers).contains(&policy.worker_count()));
    }

    #[test]
    fn a_job_returns_its_value_through_the_handle() {
        let pool = JobPool::new(2).unwrap();
        let handle = pool.spawn("add", || 20 + 22);
        assert_eq!(handle.join().unwrap(), 42);
    }

    #[test]
    fn many_jobs_all_run_and_keep_their_own_results() {
        let pool = JobPool::new(4).unwrap();
        let handles: Vec<_> = (0..200_u64)
            .map(|i| pool.spawn("square", move || i * i))
            .collect();
        let results: Vec<u64> = handles.into_iter().map(|h| h.join().unwrap()).collect();
        assert_eq!(results, (0..200_u64).map(|i| i * i).collect::<Vec<_>>());
    }

    #[test]
    fn workers_have_numbered_names() {
        let pool = JobPool::new(1).unwrap();
        let name = pool
            .spawn("name", || std::thread::current().name().map(str::to_owned))
            .join()
            .unwrap();
        assert_eq!(name.as_deref(), Some("ss2-worker-0"));
        assert_eq!(pool.worker_count(), 1);
    }

    #[test]
    fn jobs_really_run_at_the_same_time() {
        // Each job announces itself and then waits (bounded) for the other one's announcement.
        // Run one after the other, the first would never see the second and gives up.
        let pool = JobPool::new(2).unwrap();
        let (announce_a, seen_by_b) = mpsc::channel::<()>();
        let (announce_b, seen_by_a) = mpsc::channel::<()>();
        let a = pool.spawn("a", move || {
            announce_a.send(()).unwrap();
            seen_by_a.recv_timeout(Duration::from_secs(5)).is_ok()
        });
        let b = pool.spawn("b", move || {
            announce_b.send(()).unwrap();
            seen_by_b.recv_timeout(Duration::from_secs(5)).is_ok()
        });
        assert!(
            a.join().unwrap() && b.join().unwrap(),
            "the jobs did not overlap"
        );
    }

    #[test]
    fn a_panicking_job_is_an_error_and_the_pool_keeps_working() {
        let pool = JobPool::new(1).unwrap();
        let bad = pool.spawn("explode", || -> u32 { panic!("boom {}", 7) });
        match bad.join() {
            Err(JobError::Panicked { job, message }) => {
                assert_eq!(job, "explode");
                assert!(message.contains("boom 7"), "{message}");
            }
            other => panic!("expected a panic error, got {other:?}"),
        }
        assert_eq!(
            pool.spawn("after", || 5).join().unwrap(),
            5,
            "the worker survived"
        );
    }

    #[test]
    fn dropping_the_pool_finishes_the_queued_jobs_first() {
        let (tx, rx) = mpsc::channel();
        {
            let pool = JobPool::new(1).unwrap();
            for i in 0..10 {
                let tx = tx.clone();
                drop(pool.spawn("queued", move || tx.send(i).unwrap()));
            }
        } // the drop joins the worker
        drop(tx);
        assert_eq!(rx.iter().collect::<Vec<_>>(), (0..10).collect::<Vec<_>>());
    }

    #[test]
    fn a_pool_needs_at_least_one_worker() {
        assert!(JobPool::new(0).is_err());
    }

    #[test]
    fn an_unused_handle_does_not_block_or_cancel_the_job() {
        let (tx, rx) = mpsc::channel();
        let pool = JobPool::new(1).unwrap();
        drop(pool.spawn("fire and forget", move || tx.send("ran").unwrap()));
        assert_eq!(rx.recv_timeout(Duration::from_secs(10)), Ok("ran"));
    }

    #[test]
    fn a_pool_can_be_shared_between_threads() {
        let pool = Arc::new(JobPool::new(2).unwrap());
        let threads: Vec<_> = (0..4_u32)
            .map(|i| {
                let pool = Arc::clone(&pool);
                std::thread::spawn(move || pool.spawn("shared", move || i + 100).join().unwrap())
            })
            .collect();
        let mut got: Vec<u32> = threads.into_iter().map(|t| t.join().unwrap()).collect();
        got.sort_unstable();
        assert_eq!(got, [100, 101, 102, 103]);
    }
}
