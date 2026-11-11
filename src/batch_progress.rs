//! Batch progress is measured in processed packages, not download bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub completed: u64,
    pub total: u64,
}

#[derive(Default)]
pub struct BatchProgress {
    total: u64,
    completed: u64,
}

impl BatchProgress {
    pub fn start(&mut self, total: usize) {
        self.total = total as u64;
        self.completed = 0;
    }
    pub fn finish_item(&mut self) {
        self.completed = self.completed.saturating_add(1).min(self.total);
    }
    pub fn clear(&mut self) {
        self.total = 0;
        self.completed = 0;
    }
    pub fn snapshot(&self) -> Option<Snapshot> {
        (self.total > 0).then_some(Snapshot {
            completed: self.completed,
            total: self.total,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn batch_starts_at_zero_and_advances_per_processed_item() {
        let mut progress = BatchProgress::default();
        assert_eq!(progress.snapshot(), None);
        progress.start(3);
        assert_eq!(
            progress.snapshot(),
            Some(Snapshot {
                completed: 0,
                total: 3
            })
        );
        progress.finish_item();
        assert_eq!(
            progress.snapshot(),
            Some(Snapshot {
                completed: 1,
                total: 3
            })
        );
        progress.finish_item(); // A failed package also finishes processing.
        progress.finish_item();
        progress.finish_item();
        assert_eq!(
            progress.snapshot(),
            Some(Snapshot {
                completed: 3,
                total: 3
            })
        );
    }
    #[test]
    fn completion_cancellation_and_disconnect_clear_progress() {
        let mut progress = BatchProgress::default();
        progress.start(4);
        progress.finish_item();
        progress.clear();
        assert_eq!(progress.snapshot(), None);
        progress.start(2);
        assert_eq!(
            progress.snapshot(),
            Some(Snapshot {
                completed: 0,
                total: 2
            })
        );
    }
    #[test]
    fn empty_batch_has_no_progress() {
        let mut progress = BatchProgress::default();
        progress.start(0);
        progress.finish_item();
        assert_eq!(progress.snapshot(), None);
    }
}
