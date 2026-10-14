//! **The real terminal**: the `Console` the program asks its questions on.
//!
//! Name: provisional (lane maintainer/split-parents-to-zero, 2026-10-13 UTC).

use super::*;

/// Standard input and output.
pub struct Terminal;

impl Console for Terminal {
    fn say(&mut self, line: &str) {
        println!("{line}");
    }

    fn ask(&mut self, prompt: &str) -> Result<String, String> {
        print!("{prompt}");
        io::stdout().flush().map_err(|e| e.to_string())?;
        let mut line = String::new();
        let n = io::stdin()
            .lock()
            .read_line(&mut line)
            .map_err(|e| e.to_string())?;
        if n == 0 {
            return Err("no answer (end of input); nothing was written".to_owned());
        }
        Ok(line.trim().to_owned())
    }
}
