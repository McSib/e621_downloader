use std::time::Duration;

use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};

pub struct ProgressStyleBuilder {
    pub progress_style: ProgressStyle,
}

impl ProgressStyleBuilder {
    pub fn template(mut self, msg_template: &str) -> Self {
        self.progress_style = self.progress_style.template(msg_template).unwrap();
        self
    }

    pub fn progress_chars(mut self, chars: &str) -> Self {
        self.progress_style = self.progress_style.progress_chars(chars);
        self
    }
}

impl Default for ProgressStyleBuilder {
    fn default() -> Self {
        Self {
            progress_style: ProgressStyle::default_bar(),
        }
    }
}

pub struct ProgressBarBuilder {
    pub progress_bar: ProgressBar,
}

impl ProgressBarBuilder {
    pub fn new(len: u64) -> Self {
        Self {
            progress_bar: ProgressBar::new(len),
        }
    }

    pub fn style(self, progress_style: ProgressStyle) -> Self {
        self.progress_bar.set_style(progress_style);
        self
    }

    pub fn draw_target(self, target: ProgressDrawTarget) -> Self {
        self.progress_bar.set_draw_target(target);
        self
    }

    pub fn reset(self) -> Self {
        self.progress_bar.reset();
        self
    }

    pub fn steady_tick(self, duration: Duration) -> Self {
        self.progress_bar.enable_steady_tick(duration);
        self
    }

    pub fn build(self) -> ProgressBar {
        self.progress_bar
    }
}
