use log::Level;
use tauri::command;

const MAX_FRONTEND_MSG: usize = 4096;

fn sanitize_frontend_message(message: &str) -> String {
    let truncated = if message.len() > MAX_FRONTEND_MSG {
        let mut end = MAX_FRONTEND_MSG;
        while !message.is_char_boundary(end) {
            end -= 1;
        }
        &message[..end]
    } else {
        message
    };
    truncated.replace('\\', "\\\\").replace('\n', "\\n").replace('\r', "\\r")
}

#[command]
pub fn log_frontend_error(level: String, message: String) {
    let level = match level.as_str() {
        "error" => Level::Error,
        "warn" => Level::Warn,
        "info" => Level::Info,
        "debug" => Level::Debug,
        _ => Level::Info,
    };
    log::log!(level, "[frontend] {}", sanitize_frontend_message(&message));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_escapes_newlines() {
        assert_eq!(
            sanitize_frontend_message("line1\nline2\rline3"),
            "line1\\nline2\\rline3"
        );
    }

    #[test]
    fn sanitize_escapes_backslashes_first() {
        assert_eq!(sanitize_frontend_message("a\\nb"), "a\\\\nb");
    }

    #[test]
    fn sanitize_truncates_long_messages_at_char_boundary() {
        let s = "a".repeat(MAX_FRONTEND_MSG + 100);
        let out = sanitize_frontend_message(&s);
        assert_eq!(out.len(), MAX_FRONTEND_MSG);
    }

    #[test]
    fn sanitize_truncate_respects_multibyte_chars() {
        let mut s = "a".repeat(MAX_FRONTEND_MSG - 1);
        s.push('é');
        let out = sanitize_frontend_message(&s);
        assert!(out.is_char_boundary(out.len()));
        assert!(out.len() <= MAX_FRONTEND_MSG);
    }

    #[test]
    fn sanitize_short_message_unchanged() {
        assert_eq!(sanitize_frontend_message("hello"), "hello");
    }
}
