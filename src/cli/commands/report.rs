use crate::cli::context::Context;

pub struct ReportArgs {
    pub today: bool,
    pub week: bool,
    pub month: bool,
    pub all: bool,
    pub range: Option<String>,
    pub by_task: bool,
    pub by_epic: bool,
    pub by_day: bool,
    pub json: bool,
}

pub fn report(_ctx: &mut Context, _args: ReportArgs) -> anyhow::Result<i32> {
    // Stub — fully implemented in Tasks 4–11.
    println!("report: not yet implemented");
    Ok(0)
}
