/*!
 * Utility functions and helpers for MechaFlow.
 *
 * This module provides common utilities used throughout the MechaFlow ecosystem.
 */
use std::future::Future;
use std::pin::Pin;
use std::time::{Duration, Instant};

use tokio::time::timeout;
use tracing::{debug, warn};

use crate::error::{Error, Result};

/// Run a future with a timeout
///
/// # Arguments
///
/// * `duration` - The timeout duration
/// * `future` - The future to run
///
/// # Returns
///
/// The result of the future, or a timeout error if the timeout is reached
pub async fn with_timeout<F, T>(duration: Duration, future: F) -> Result<T>
where
    F: Future<Output = Result<T>>,
{
    match timeout(duration, future).await {
        Ok(result) => result,
        Err(_) => Err(Error::timeout("Operation timed out")),
    }
}

/// Run a future with a timeout and retry on failure
///
/// # Arguments
///
/// * `duration` - The timeout duration for each attempt
/// * `retries` - The number of retries
/// * `future_factory` - A function that creates a new future for each retry
///
/// # Returns
///
/// The result of the future, or the last error if all retries fail
pub async fn with_retry<F, Fut, T>(
    duration: Duration,
    retries: usize,
    mut future_factory: F,
) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T>>,
{
    let mut last_error = None;
    let start = Instant::now();

    for i in 0..=retries {
        if i > 0 {
            debug!("Retry {}/{}", i, retries);
        }

        match timeout(duration, future_factory()).await {
            Ok(Ok(result)) => {
                if i > 0 {
                    debug!("Succeeded after {} retries", i);
                }
                return Ok(result);
            }
            Ok(Err(e)) => {
                warn!("Attempt {} failed: {}", i + 1, e);
                last_error = Some(e);
            }
            Err(_) => {
                warn!("Attempt {} timed out", i + 1);
                last_error = Some(Error::timeout("Operation timed out"));
            }
        }
    }

    let elapsed = start.elapsed();
    warn!(
        "All {} retries failed after {:?}",
        retries,
        elapsed
    );

    Err(last_error.unwrap_or_else(|| Error::other("Unknown error in retry loop")))
}

/// Create a task that runs in the background
///
/// # Arguments
///
/// * `fut` - The future to run
///
/// # Returns
///
/// A handle to the spawned task
pub fn spawn_task<F>(fut: F) -> tokio::task::JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    tokio::spawn(fut)
}

/// Create a task that runs in the background and logs any errors
///
/// # Arguments
///
/// * `name` - A name for the task (for logging)
/// * `fut` - The future to run
pub fn spawn_and_log<F, T, E>(name: &str, fut: F) -> tokio::task::JoinHandle<()>
where
    F: Future<Output = std::result::Result<T, E>> + Send + 'static,
    T: Send + 'static,
    E: std::fmt::Display + Send + 'static,
{
    let task_name = name.to_string();
    tokio::spawn(async move {
        match fut.await {
            Ok(_) => {
                debug!("Task '{}' completed successfully", task_name);
            }
            Err(e) => {
                warn!("Task '{}' failed: {}", task_name, e);
            }
        }
    })
}

/// Convert a Duration to milliseconds
///
/// # Arguments
///
/// * `duration` - The duration to convert
///
/// # Returns
///
/// The duration in milliseconds
pub fn duration_to_millis(duration: Duration) -> u64 {
    duration.as_secs() * 1000 + u64::from(duration.subsec_millis())
}

/// Convert milliseconds to a Duration
///
/// # Arguments
///
/// * `millis` - The milliseconds to convert
///
/// # Returns
///
/// A Duration representing the milliseconds
pub fn millis_to_duration(millis: u64) -> Duration {
    Duration::from_millis(millis)
}

/// Create a Box<dyn Future> from a future
///
/// # Arguments
///
/// * `future` - The future to box
///
/// # Returns
///
/// A boxed future
pub fn box_future<F, T>(future: F) -> Pin<Box<dyn Future<Output = T> + Send>>
where
    F: Future<Output = T> + Send + 'static,
{
    Box::pin(future)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_with_timeout_success() {
        let result = with_timeout(Duration::from_secs(1), async { Ok::<_, Error>(42) }).await;
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_with_timeout_failure() {
        let result = with_timeout(
            Duration::from_millis(10),
            async {
                tokio::time::sleep(Duration::from_millis(100)).await;
                Ok::<_, Error>(42)
            },
        )
        .await;
        assert!(matches!(result, Err(Error::Timeout(_))));
    }

    #[tokio::test]
    async fn test_with_retry_success_first_try() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let result = with_retry(
            Duration::from_secs(1),
            3,
            move || {
                counter_clone.fetch_add(1, Ordering::SeqCst);
                async { Ok::<_, Error>(42) }
            },
        )
        .await;

        assert_eq!(result.unwrap(), 42);
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_with_retry_success_after_retries() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let result = with_retry(
            Duration::from_secs(1),
            3,
            move || {
                let current = counter_clone.fetch_add(1, Ordering::SeqCst);
                async move {
                    if current < 2 {
                        Err(Error::other("Intentional failure"))
                    } else {
                        Ok(42)
                    }
                }
            },
        )
        .await;

        assert_eq!(result.unwrap(), 42);
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_with_retry_all_failures() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let result = with_retry(
            Duration::from_secs(1),
            2,
            move || {
                counter_clone.fetch_add(1, Ordering::SeqCst);
                async { Err(Error::other("Intentional failure")) }
            },
        )
        .await;

        assert!(result.is_err());
        assert_eq!(counter.load(Ordering::SeqCst), 3); // Initial + 2 retries
    }

    #[tokio::test]
    async fn test_spawn_task() {
        let handle = spawn_task(async { 42 });
        let result = handle.await.unwrap();
        assert_eq!(result, 42);
    }

    #[tokio::test]
    async fn test_duration_conversions() {
        let duration = Duration::from_millis(1234);
        let millis = duration_to_millis(duration);
        assert_eq!(millis, 1234);

        let duration2 = millis_to_duration(millis);
        assert_eq!(duration, duration2);
    }
}
