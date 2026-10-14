//! Running recursive parsers on a thread with a large stack.
//!
//! Some parsers (YAML, SVG) recurse once per nesting level. They have their own depth limits,
//! but a few hundred levels can still overflow a small stack, such as the 512 KB of the
//! threads Swift concurrency runs work on. A stack overflow aborts the whole process, so
//! such parsing runs on a dedicated thread whose stack is big enough for those limits.

use std::thread;

/// Enough for the parsers' own nesting limits, even in debug builds. Only touched pages are
/// committed, so this costs nothing for shallow input.
const STACK_SIZE: usize = 64 * 1024 * 1024;

/// Runs `work` on a thread with a [`STACK_SIZE`] stack and returns its result. If the thread
/// can't be started, `work` runs on the current thread instead.
pub(crate) fn with_large_stack<T: Send>(work: impl FnOnce() -> T + Send) -> T {
    let work = std::sync::Mutex::new(Some(work));
    let run = || {
        work.lock()
            .ok()
            .and_then(|mut work| work.take())
            .map(|work| work())
    };
    let spawned = thread::scope(|scope| {
        thread::Builder::new()
            .stack_size(STACK_SIZE)
            .spawn_scoped(scope, run)
            .ok()
            .map(|handle| handle.join())
    });
    match spawned {
        Some(Ok(Some(result))) => result,
        Some(Err(panic)) => std::panic::resume_unwind(panic),
        _ => {
            let work = work
                .into_inner()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .expect("work runs only once, and only here if the thread didn't take it");
            work()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_the_result() {
        assert_eq!(with_large_stack(|| 40 + 2), 42);
    }

    #[test]
    fn deep_recursion_fits() {
        fn depth(n: u64) -> u64 {
            let padding = [n; 64];
            if n == 0 {
                0
            } else {
                depth(n - 1) + padding[0] / n
            }
        }
        assert_eq!(with_large_stack(|| depth(20_000)), 20_000);
    }
}
