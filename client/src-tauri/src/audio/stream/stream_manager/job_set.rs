use std::time::Duration;
use tokio::task::JoinHandle;

/// The tasks a stream owns, stopped by waiting for them rather than by waiting a fixed time.
pub struct JobSet {
    jobs: Vec<JoinHandle<()>>,
}

impl JobSet {
    pub fn empty() -> Self {
        Self { jobs: Vec::new() }
    }

    pub fn is_empty(&self) -> bool {
        self.jobs.is_empty()
    }

    /// Waits for every job to end, aborting any that outlast `grace`. Returns whether they ended
    /// on their own.
    ///
    /// The set is emptied before the wait, so a stream reports itself stopped from the moment the
    /// stop commits rather than when it completes.
    pub async fn settle(&mut self, grace: Duration) -> bool {
        let jobs = std::mem::take(&mut self.jobs);
        if jobs.is_empty() {
            return true;
        }

        let aborts: Vec<_> = jobs.iter().map(|job| job.abort_handle()).collect();

        let joined = async {
            for job in jobs {
                let _ = job.await;
            }
        };

        if tokio::time::timeout(grace, joined).await.is_err() {
            for handle in &aborts {
                handle.abort();
            }
            return false;
        }

        true
    }
}

impl From<Vec<JoinHandle<()>>> for JobSet {
    fn from(jobs: Vec<JoinHandle<()>>) -> Self {
        Self { jobs }
    }
}

impl Default for JobSet {
    fn default() -> Self {
        Self::empty()
    }
}
