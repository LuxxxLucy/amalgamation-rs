use indicatif::{ProgressBar, ProgressStyle};

pub struct ProgressTracker {
    bar: ProgressBar,
}

impl ProgressTracker {
    pub fn new() -> Self {
        let bar = ProgressBar::new_spinner();
        bar.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} [{elapsed_precise}] {msg}")
                .unwrap(),
        );
        Self { bar }
    }

    pub fn set_stage(&self, stage: &str) {
        self.bar.set_message(stage.to_string());
    }

    pub fn finish(&self) {
        self.bar.finish_with_message("Complete!");
    }
}

impl Drop for ProgressTracker {
    fn drop(&mut self) {
        if !self.bar.is_finished() {
            self.bar.finish_and_clear();
        }
    }
}
