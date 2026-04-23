//! Handlers for each CLI subcommand.
//!
//! Each function returns `anyhow::Result<i32>` where the integer is the exit
//! code. 0 = success; 1 = logical failure; other codes reserved.

use crate::cli::context::Context;

pub fn add(_ctx: &mut Context, _title: &str, _description: Option<&str>) -> anyhow::Result<i32> {
    todo!("Task 8")
}

pub fn list(
    _ctx: &mut Context,
    _all: bool,
    _archived: bool,
    _completed: bool,
) -> anyhow::Result<i32> {
    todo!("Task 9")
}

pub fn start(_ctx: &mut Context, _target: &str) -> anyhow::Result<i32> {
    todo!("Task 10")
}

pub fn stop(_ctx: &mut Context) -> anyhow::Result<i32> {
    todo!("Task 11")
}

pub fn status(_ctx: &mut Context) -> anyhow::Result<i32> {
    todo!("Task 11")
}

pub fn done(_ctx: &mut Context, _id: i64) -> anyhow::Result<i32> {
    todo!("Task 12")
}

pub fn archive(_ctx: &mut Context, _id: i64) -> anyhow::Result<i32> {
    todo!("Task 12")
}

pub fn delete(_ctx: &mut Context, _id: i64) -> anyhow::Result<i32> {
    todo!("Task 12")
}
