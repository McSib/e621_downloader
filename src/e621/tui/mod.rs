/*
 * Copyright (c) 2022 McSib
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use std::time::Duration;

use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};

pub(crate) struct ProgressStyleBuilder {
    progress_style: ProgressStyle,
}

impl ProgressStyleBuilder {
    pub(crate) fn template(mut self, msg_template: &str) -> Self {
        self.progress_style = self.progress_style.template(msg_template).unwrap();
        self
    }

    pub(crate) fn progress_chars(mut self, chars: &str) -> Self {
        self.progress_style = self.progress_style.progress_chars(chars);
        self
    }

    pub(crate) fn build(self) -> ProgressStyle {
        self.progress_style
    }
}

impl Default for ProgressStyleBuilder {
    fn default() -> Self {
        Self {
            progress_style: ProgressStyle::default_bar(),
        }
    }
}

pub(crate) struct ProgressBarBuilder {
    pub(crate) progress_bar: ProgressBar,
}

impl ProgressBarBuilder {
    pub(crate) fn new(len: u64) -> Self {
        Self {
            progress_bar: ProgressBar::new(len),
        }
    }

    pub(crate) fn style(self, progress_style: ProgressStyle) -> Self {
        self.progress_bar.set_style(progress_style);
        self
    }

    pub(crate) fn draw_target(self, target: ProgressDrawTarget) -> Self {
        self.progress_bar.set_draw_target(target);
        self
    }

    pub(crate) fn reset(self) -> Self {
        self.progress_bar.reset();
        self
    }

    pub(crate) fn steady_tick(self, duration: Duration) -> Self {
        self.progress_bar.enable_steady_tick(duration);
        self
    }

    pub(crate) fn build(self) -> ProgressBar {
        self.progress_bar
    }
}
