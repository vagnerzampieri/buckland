//! Handlers for each CLI subcommand.
//!
//! Each function returns `anyhow::Result<i32>` where the integer is the exit
//! code. 0 = success; 1 = logical failure; other codes reserved.

mod add;
mod archive;
mod delete;
mod done;
mod helpers;
mod list;
mod report;
mod shortcut;
mod start;
mod status;
mod stop;

pub use add::add;
pub use archive::archive;
pub use delete::delete;
pub use done::done;
pub use list::list;
pub use report::{report, ReportArgs};
pub use shortcut::shortcut_refresh;
pub use start::start;
pub use status::status;
pub use stop::stop;
