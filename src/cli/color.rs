use owo_colors::OwoColorize;

use crate::ui::Style;

/// Color helper with a single policy shared across commands.
///
/// Notes:
/// - All functions return `String` to avoid borrow-to-temporary issues (E0716).
/// - Colors are applied only if `style.color == true`.
pub struct Colors {
    pub enabled: bool,
}

impl Colors {
    pub fn new(style: &Style) -> Self {
        Self {
            enabled: style.color,
        }
    }

    #[inline]
    pub fn ok(&self, s: impl AsRef<str>) -> String {
        let s = s.as_ref();
        if self.enabled {
            s.green().to_string()
        } else {
            s.to_string()
        }
    }

    #[inline]
    pub fn err(&self, s: impl AsRef<str>) -> String {
        let s = s.as_ref();
        if self.enabled {
            s.red().to_string()
        } else {
            s.to_string()
        }
    }

    #[inline]
    pub fn warn(&self, s: impl AsRef<str>) -> String {
        let s = s.as_ref();
        if self.enabled {
            s.yellow().to_string()
        } else {
            s.to_string()
        }
    }

    #[inline]
    pub fn info(&self, s: impl AsRef<str>) -> String {
        let s = s.as_ref();
        if self.enabled {
            s.cyan().to_string()
        } else {
            s.to_string()
        }
    }

    #[inline]
    pub fn dim(&self, s: impl AsRef<str>) -> String {
        let s = s.as_ref();
        if self.enabled {
            s.bright_black().to_string()
        } else {
            s.to_string()
        }
    }

    // Domain-specific helpers (policy)
    #[inline]
    pub fn from_name(&self, s: impl AsRef<str>) -> String {
        // from = red
        self.err(s)
    }

    #[inline]
    pub fn to_name(&self, s: impl AsRef<str>) -> String {
        // to = green
        self.ok(s)
    }

    #[inline]
    pub fn label_start(&self, s: impl AsRef<str>) -> String {
        // start label = red
        self.err(s)
    }

    #[inline]
    pub fn label_end(&self, s: impl AsRef<str>) -> String {
        // end label = green
        self.ok(s)
    }

    #[inline]
    pub fn label_detour(&self, s: impl AsRef<str>) -> String {
        // detour label = yellow
        self.warn(s)
    }

    #[inline]
    pub fn obstacle(&self, s: impl AsRef<str>) -> String {
        // obstacle = red
        self.err(s)
    }

    #[inline]
    pub fn waypoint(&self, s: impl AsRef<str>) -> String {
        // inserted waypoint = yellow
        self.warn(s)
    }

    #[inline]
    pub fn violated(&self, s: impl AsRef<str>) -> String {
        // inserted waypoint = yellow
        self.err(s)
    }

    #[inline]
    pub fn tries(&self, exhausted: bool, s: impl AsRef<str>) -> String {
        // exhausted tries = red, otherwise green
        let s = s.as_ref();
        if !self.enabled {
            return s.to_string();
        }
        if exhausted {
            s.red().to_string()
        } else {
            s.green().to_string()
        }
    }

    /// Color a "total score" given a penalty ratio (penalties/base).
    /// Policy:
    /// - <= 1%: green
    /// - <= 5%: yellow
    /// - else: red
    pub fn score_total_by_ratio(&self, penalty_ratio: f64, total_txt: impl AsRef<str>) -> String {
        let s = total_txt.as_ref();
        if !self.enabled {
            return s.to_string();
        }

        if penalty_ratio <= 0.01 {
            s.green().to_string()
        } else if penalty_ratio <= 0.05 {
            s.yellow().to_string()
        } else {
            s.red().to_string()
        }
    }

    /// Color a scalar magnitude (dominant penalty value).
    /// Policy:
    /// - <= 0.05: green
    /// - <= 1.0: yellow
    /// - else: red
    pub fn magnitude(&self, v: f64, txt: impl AsRef<str>) -> String {
        let s = txt.as_ref();
        if !self.enabled {
            return s.to_string();
        }

        if v <= 0.05 {
            s.green().to_string()
        } else if v <= 1.0 {
            s.yellow().to_string()
        } else {
            s.red().to_string()
        }
    }

    pub fn driver_line(&self, line: impl AsRef<str>) -> String {
        let line = line.as_ref();
        if !self.enabled {
            return line.to_string();
        }
        if line.starts_with("constraint:") || line.starts_with("limit:") {
            line.red().to_string()
        } else if line.starts_with("cost:") {
            line.yellow().to_string()
        } else {
            line.to_string()
        }
    }

    /// Color a value based on absolute thresholds.
    /// Policy:
    /// - value <= good  -> green
    /// - value >= bad   -> red
    /// - otherwise      -> yellow
    #[inline]
    pub fn by_thresholds(
        &self,
        value: f64,
        good: Option<f64>,
        bad: Option<f64>,
        txt: impl AsRef<str>,
    ) -> String {
        let s = txt.as_ref();
        if !self.enabled {
            return s.to_string();
        }

        match (good, bad) {
            (Some(g), Some(b)) => {
                if value <= g {
                    s.green().to_string()
                } else if value >= b {
                    s.red().to_string()
                } else {
                    s.yellow().to_string()
                }
            }
            _ => s.to_string(),
        }
    }

    #[inline]
    pub fn dom_penalty(&self, v: f64, txt: impl AsRef<str>) -> String {
        let dom_val_plain = format!("{:.3}", v);
        let dom_val_out: String;
        if v <= 0.05 {
            dom_val_out = self.ok(dom_val_plain);
        } else if v <= 1.0 {
            dom_val_out = self.warn(dom_val_plain);
        } else {
            dom_val_out = self.err(dom_val_plain);
        }

        format!("{} ({})", self.warn(txt), dom_val_out)
    }
}
