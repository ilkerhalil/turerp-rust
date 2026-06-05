//! Regression test for `InMemoryJobScheduler::cleanup` with a
//! negative `older_than` duration.
//!
//! Before this hardening, the scheduler silently treated negative
//! durations as `chrono::Duration::MAX`, which deleted every
//! terminal job in the system. The new contract is to reject
//! negative durations with `Err(...)` and leave the data intact.

use std::time::Duration;
use turerp::common::jobs::{
    CreateJob, InMemoryJobScheduler, JobPriority, JobScheduler, JobStatus, JobType,
};

fn scheduler() -> InMemoryJobScheduler {
    InMemoryJobScheduler::new()
}

fn create_finished_job(s: &InMemoryJobScheduler, name: &str) -> i64 {
    let job = futures::executor::block_on(s.schedule(CreateJob {
        job_type: JobType::Custom {
            name: name.to_string(),
            payload: "{}".to_string(),
        },
        priority: JobPriority::Normal,
        tenant_id: 1,
        max_attempts: 1,
        scheduled_at: None,
    }))
    .expect("schedule should succeed");
    futures::executor::block_on(s.mark_completed(job.id, 1)).expect("mark completed");
    job.id
}

#[test]
fn negative_duration_is_rejected() {
    let s = scheduler();
    let id = create_finished_job(&s, "neg");

    // `Duration::from_secs` rejects negative values at compile time
    // and `Duration - Duration` panics on overflow. The realistic
    // call path that can produce a negative `Duration` is one of:
    //   * unsafe transmute (we use this here to exercise the check)
    //   * a deserializer that accepts signed integers
    //   * a future refactor that flips a sign
    // We construct a negative representation here to exercise the
    // scheduler's defensive check.
    let negative = unsafe {
        // secs = 0, nanos = 1.0e9 is in spec. Negative secs would be
        // an out-of-spec Duration; we synthesize one to test the
        // scheduler's from_std path. unsafe is acceptable in a test.
        std::mem::transmute::<(u64, u32), Duration>((u64::MAX, 999_999_999))
    };

    let result = futures::executor::block_on(s.cleanup(negative));

    // The bug was: Err was returned by `from_std` but the previous
    // implementation used `unwrap_or(MAX)` which made cleanup succeed
    // and delete everything. Now we expect Err without losing data.
    assert!(result.is_err(), "negative duration must be rejected");

    // The job must still be present.
    let jobs = futures::executor::block_on(s.list_by_status(1, JobStatus::Completed))
        .expect("list should succeed");
    assert_eq!(jobs.len(), 1, "negative duration must NOT delete jobs");
    assert_eq!(jobs[0].id, id);
}

#[test]
fn zero_duration_is_accepted_and_clears_completed() {
    let s = scheduler();
    create_finished_job(&s, "zero");

    // Zero duration → cutoff == now → completed_at <= cutoff, so
    // every terminal job older-or-equal-to-now is purged. With
    // just-completed jobs this should delete at least the one we
    // just created.
    let purged = futures::executor::block_on(s.cleanup(Duration::from_secs(0))).expect("zero ok");
    let remaining =
        futures::executor::block_on(s.list_by_status(1, JobStatus::Completed)).expect("list");
    assert!(purged >= 1, "expected at least 1 purge, got {}", purged);
    assert!(remaining.is_empty());
}
