use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Controls whether typewriter animation is enabled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypewriterMode {
    Off,
    On,
}

/// Runtime configuration for the typewriter effect.
#[derive(Debug, Clone, Copy)]
pub struct TypewriterConfig {
    pub mode: TypewriterMode,
    pub delay_ms: u64,
    pub chars_per_tick: usize,
    pub max_animated_lines: usize,
}

impl Default for TypewriterConfig {
    fn default() -> Self {
        Self {
            mode: TypewriterMode::On,
            delay_ms: 15,
            chars_per_tick: 2,
            max_animated_lines: 30,
        }
    }
}

/// Internal state for the typewriter effect.
#[derive(Debug, Clone)]
pub struct TypewriterState {
    queue: VecDeque<String>,
    current_line: String,
    visible_chars: usize,
    last_tick: Instant,
    active: bool,
}

impl Default for TypewriterState {
    fn default() -> Self {
        Self {
            queue: VecDeque::new(),
            current_line: String::new(),
            visible_chars: 0,
            last_tick: Instant::now(),
            active: false,
        }
    }
}

impl TypewriterState {
    /// Returns true if an animation is currently active.
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Queue lines for animated rendering.
    pub fn enqueue_lines(&mut self, lines: Vec<String>) {
        if lines.is_empty() {
            return;
        }

        self.queue.extend(lines);
        self.last_tick = Instant::now();

        if !self.active {
            self.advance_to_next_line();
            self.active = !self.current_line.is_empty();
        }
    }

    /// Immediately stops animation and returns all pending lines, including the current one.
    pub fn flush_all(&mut self) -> Vec<String> {
        let mut out = Vec::new();

        if self.active && !self.current_line.is_empty() {
            out.push(self.current_line.clone());
        }

        out.extend(self.queue.drain(..));

        self.current_line.clear();
        self.visible_chars = 0;
        self.active = false;

        out
    }

    /// Advance animation state. Returns any newly completed log lines.
    pub fn update(&mut self, config: TypewriterConfig) -> Vec<String> {
        let mut completed = Vec::new();

        if config.mode == TypewriterMode::Off || !self.active {
            return completed;
        }

        if self.last_tick.elapsed() < Duration::from_millis(config.delay_ms) {
            return completed;
        }

        self.last_tick = Instant::now();

        let total_chars = self.current_line.chars().count();

        if self.visible_chars < total_chars {
            self.visible_chars = (self.visible_chars + config.chars_per_tick).min(total_chars);
            return completed;
        }

        completed.push(self.current_line.clone());

        self.active = self.advance_to_next_line();

        completed
    }

    /// Returns the currently visible partial line, if any.
    pub fn visible_partial_line(&self) -> Option<String> {
        if !self.active || self.current_line.is_empty() {
            return None;
        }

        Some(self.current_line.chars().take(self.visible_chars).collect())
    }

    fn advance_to_next_line(&mut self) -> bool {
        if let Some(next) = self.queue.pop_front() {
            self.current_line = next;
            self.visible_chars = 0;
            true
        } else {
            self.current_line.clear();
            self.visible_chars = 0;
            false
        }
    }
}
