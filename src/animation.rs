use colored::Colorize;
use crossterm::{
    cursor::MoveToColumn,
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{Clear, ClearType},
};
use std::io::Write;
use tokio::time::Duration;

pub async fn start<W: Write + Send + 'static>(
    message: String,
    no_animation: bool,
    writer: W,
) -> tokio::task::JoinHandle<()> {
    let mut writer = writer;
    tokio::spawn(async move {
        if no_animation {
            writeln!(writer, "{}", message.bright_black()).ok();
            return;
        }
        let emoji_support =
            terminal_supports_emoji::supports_emoji(terminal_supports_emoji::Stream::Stdout);
        let frames = if emoji_support {
            vec![
                "🕛", "🕐", "🕑", "🕒", "🕓", "🕔", "🕕", "🕖", "🕗", "🕘", "🕙", "🕚",
            ]
        } else {
            vec!["/", "-", "\\", "|"]
        };
        let mut current_frame = 0;
        loop {
            current_frame = (current_frame + 1) % frames.len();
            match execute!(
                writer,
                Clear(ClearType::CurrentLine),
                MoveToColumn(0),
                SetForegroundColor(Color::Yellow),
                Print(message.bright_black()),
                Print(frames[current_frame]),
                ResetColor
            ) {
                Ok(_) => {}
                Err(_) => {}
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
}

#[cfg(test)]
mod tests {

    use std::io::Result;
    use std::sync::{Arc, Mutex};

    use super::*;

    #[tokio::test]
    async fn test_animation() {
        let msg = String::from("Loading");
        let buffer = Arc::new(Mutex::new(Vec::new()));
        let writer = SharedBufferWriter {
            buffer: buffer.clone(),
        };
        let animation = start(msg, false, writer).await;
        tokio::time::sleep(Duration::from_millis(120)).await;
        animation.abort();

        // Lock the Mutex, read the buffer's contents, and then unlock the Mutex
        let locked_buffer = buffer.lock().unwrap();
        let output = String::from_utf8(locked_buffer.clone()).unwrap();

        //https://learn.microsoft.com/en-us/windows/console/console-virtual-terminal-sequences
        let expected = "\u{1b}[2K\u{1b}[1G\u{1b}[38;5;11m\u{1b}[90mLoading\u{1b}[0m🕐\u{1b}[0m\u{1b}[2K\u{1b}[1G\u{1b}[38;5;11m\u{1b}[90mLoading\u{1b}[0m🕑\u{1b}[0m";

        assert_eq!(output, expected);
    }

    struct SharedBufferWriter {
        buffer: Arc<Mutex<Vec<u8>>>,
    }

    impl Write for SharedBufferWriter {
        fn write(&mut self, buf: &[u8]) -> Result<usize> {
            let mut buffer = self.buffer.lock().unwrap();
            buffer.extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> Result<()> {
            Ok(())
        }
    }
}
