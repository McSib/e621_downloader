use std::io::stdout;
use std::time::Duration;
use crossterm::*;
use lazy_static::lazy_static;

const MAX_FLOAT_PRINTING_PRECISION: usize = 10;

lazy_static! {
    pub static ref INTERVAL: Duration = Duration::from_millis(100);
}

pub fn term_height() -> usize {
    terminal::size().unwrap().1 as usize
}

pub fn unprint(item_count: usize) {
    execute!(
        stdout(),
        cursor::MoveUp(item_count as u16),
        terminal::Clear(terminal::ClearType::FromCursorDown)
    ).unwrap()
}

pub fn float_printing_precision(n: f64) -> usize {
    let s = n.to_string();
    match s.split('.').nth(1) {
        Some(s) => s.len().min(MAX_FLOAT_PRINTING_PRECISION),
        None => 0
    }
}

pub fn value_valid(value: f64, step: Option<f64>, min: Option<f64>, max: Option<f64>) -> bool {
    if value.is_nan() {
        return false;
    }
    if let Some(min) = min {
        if value < min {
            return false;
        }
    }
    if let Some(max) = max {
        if value > max {
            return false;
        }
    }
    if let Some(step) = step {
        if (value - min.unwrap_or(max.unwrap_or(0.0)).abs()) % step != 0.0 {
            return false;
        }
    }
    true
}

pub fn number_range_indicator(step: Option<f64>, min: Option<f64>, max: Option<f64>) -> String {
    let prefix = String::new();
    if let Some(step) = step {
        if let Some(min) = min {
            print!("[{:.*}, {:.*}, ..",
                   float_printing_precision(min), min,
                   float_printing_precision(min + step), min + step,
            );
            if let Some(max) = max {
                print!(", {:.*}] ", float_printing_precision(max), max);
            } else {
                print!("] ");
            }
        } else if let Some(max) = max {
            print!("[.., {:.*}, {:.*}] ",
                   float_printing_precision(max - step), max - step,
                   float_printing_precision(max), max
            );
        } else {
            print!("[.., {:.*}, 0, {:.*}, ..] ",
                   float_printing_precision(-step), -step,
                   float_printing_precision(step), step
            );
        }
    } else if let Some(min) = min {
        if let Some(max) = max {
            print!("[{:.*}..{:.*}] ",
                   float_printing_precision(min), min,
                   float_printing_precision(max), max
            );
        } else {
            print!("[> {:.*}] ", float_printing_precision(min), min);
        }
    } else if let Some(max) = max {
        print!("[< {:.*}] ", float_printing_precision(max), max);
    } else {
        print!(": ");
    }
    prefix
}