mod ansi;
#[cfg(not(target_family = "wasm"))]
mod appearance;
mod block;
mod colors;
#[cfg(not(target_family = "wasm"))]
mod controls;
#[cfg(not(target_family = "wasm"))]
mod frame;
mod history;
#[cfg(not(target_family = "wasm"))]
mod input;
#[cfg(not(target_family = "wasm"))]
mod keys;
mod links;
mod logs;
#[cfg(not(target_family = "wasm"))]
mod paint;
mod processes;
#[cfg(not(target_family = "wasm"))]
mod pty;
#[cfg(not(target_family = "wasm"))]
mod remote;
#[cfg(not(target_family = "wasm"))]
mod search;
mod shells;
#[cfg(all(test, feature = "test-support"))]
mod tests;
#[cfg(not(target_family = "wasm"))]
mod view;

pub use ansi::AnsiText;
#[cfg(not(target_family = "wasm"))]
pub use appearance::Appearance;
pub use block::{CommandBlock, CommandState};
pub use history::{CommandHistory, PastCommand};
#[cfg(not(target_family = "wasm"))]
pub(crate) use input::bind_keys;
pub use links::{Target, links};
pub use logs::{LogLevel, LogLine, LogViewer};
pub use processes::{Process, ProcessList};
#[cfg(not(target_family = "wasm"))]
pub use pty::Launch;
#[cfg(not(target_family = "wasm"))]
pub use remote::RemoteTransport;
pub use shells::{ShellChoice, ShellSelector, TerminalToolbar};
#[cfg(not(target_family = "wasm"))]
pub use view::{Terminal, TerminalEvent};
