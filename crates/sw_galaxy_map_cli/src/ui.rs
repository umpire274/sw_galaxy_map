use owo_colors::OwoColorize;

#[derive(Clone, Copy, Debug)]
pub enum Level {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Clone, Copy, Debug)]
pub struct Style {
    pub emoji: bool,
    pub color: bool,
}

impl Default for Style {
    fn default() -> Self {
        // Colors only when stdout is a TTY; emojis always on by default.
        let color = atty::is(atty::Stream::Stdout);
        Self { emoji: true, color }
    }
}

pub fn info(msg: impl AsRef<str>) {
    print_line(Level::Info, msg.as_ref(), Style::default());
}

pub fn success(msg: impl AsRef<str>) {
    print_line(Level::Success, msg.as_ref(), Style::default());
}

pub fn warning(msg: impl AsRef<str>) {
    print_line(Level::Warning, msg.as_ref(), Style::default());
}

pub fn error(msg: impl AsRef<str>) {
    print_line(Level::Error, msg.as_ref(), Style::default());
}

/// Lower-level API if you need custom style (e.g., disable emoji/colors).
pub fn print_line(level: Level, msg: &str, style: Style) {
    let emoji = match level {
        Level::Info => "ℹ️ ",
        Level::Success => "✅ ",
        Level::Warning => "⚠️ ",
        Level::Error => "❌ ",
    };

    let prefix = if style.emoji { emoji } else { "" };
    let line = format!("{}{}", prefix, msg);

    if style.color {
        match level {
            Level::Info => println!("{}", line),
            Level::Success => println!("{}", line.green()),
            Level::Warning => println!("{}", line.yellow()),
            Level::Error => println!("{}", line.red()),
        }
    } else {
        println!("{}", line);
    }
}
