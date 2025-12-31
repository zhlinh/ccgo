//! Terminal output utilities

#![allow(dead_code)]

use console::{style, Term};
use indicatif::{ProgressBar, ProgressStyle};

/// Print an error message to stderr
pub fn print_error(message: &str) {
    eprintln!("{}: {}", style("error").red().bold(), message);
}

/// Print a warning message to stderr
pub fn print_warning(message: &str) {
    eprintln!("{}: {}", style("warning").yellow().bold(), message);
}

/// Print a success message
pub fn print_success(message: &str) {
    println!("{}: {}", style("success").green().bold(), message);
}

/// Print an info message
pub fn print_info(message: &str) {
    println!("{}: {}", style("info").blue().bold(), message);
}

/// Create a spinner progress bar
pub fn create_spinner(message: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
            .template("{spinner:.blue} {msg}")
            .unwrap(),
    );
    pb.set_message(message.to_string());
    pb.enable_steady_tick(std::time::Duration::from_millis(100));
    pb
}

/// Create a progress bar with a known length
pub fn create_progress_bar(len: u64, message: &str) -> ProgressBar {
    let pb = ProgressBar::new(len);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{msg} [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
            .unwrap()
            .progress_chars("=>-"),
    );
    pb.set_message(message.to_string());
    pb
}

/// Get the terminal for interactive output
pub fn get_term() -> Term {
    Term::stderr()
}

/// Clear the current line
pub fn clear_line() {
    let term = get_term();
    let _ = term.clear_line();
}
