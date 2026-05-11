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
    pub copy: bool,
}

pub fn report(ctx: &mut Context, args: ReportArgs) -> anyhow::Result<i32> {
    use crate::domain::ReportBuilder;

    let now = chrono::Utc::now();
    let scope = match resolve_scope(&args, now) {
        Ok(s) => s,
        Err(msg) => {
            println!("{msg}");
            return Ok(1);
        }
    };
    let grouping = resolve_grouping(&args);

    let report = ReportBuilder::new(&ctx.repo).build(scope, grouping, now)?;

    let payload = if args.json {
        serde_json::to_string_pretty(&report)?
    } else {
        format_one_liner(&report)
    };

    if args.copy {
        match crate::clipboard::copy(&payload) {
            Ok(tool) => {
                eprintln!("Copied to clipboard via {tool}");
                Ok(0)
            }
            Err(e) => {
                eprintln!("clipboard copy failed: {e}");
                Ok(1)
            }
        }
    } else if args.json {
        println!("{payload}");
        Ok(0)
    } else if report.rows.is_empty() {
        println!("No time tracked in this scope.");
        Ok(0)
    } else {
        print_table(&report);
        Ok(0)
    }
}

fn format_one_liner(report: &crate::domain::Report) -> String {
    report.one_liner()
}

fn resolve_scope(
    args: &ReportArgs,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<crate::domain::Scope, String> {
    use crate::domain::Scope;
    if args.week {
        Ok(Scope::week(now))
    } else if args.month {
        Ok(Scope::month(now))
    } else if args.all {
        Ok(Scope::all(now))
    } else if let Some(raw) = args.range.as_deref() {
        Scope::range(raw).map_err(|e| e.to_string())
    } else {
        // Default and explicit --today both land here.
        Ok(Scope::today(now))
    }
}

fn resolve_grouping(args: &ReportArgs) -> crate::domain::Grouping {
    use crate::domain::Grouping;
    if args.by_epic {
        Grouping::Epic
    } else if args.by_day {
        Grouping::Day
    } else {
        Grouping::Task
    }
}

fn print_table(report: &crate::domain::Report) {
    use super::helpers::truncate;
    use crate::cli::format::{bar, duration_compact};
    use crate::domain::ScopeKind;
    let header = match report.scope.kind {
        ScopeKind::Today => "Today".to_string(),
        ScopeKind::Week => "This week".to_string(),
        ScopeKind::Month => "This month".to_string(),
        ScopeKind::All => "All time".to_string(),
        ScopeKind::Range => format!(
            "{} to {}",
            report
                .scope
                .from
                .with_timezone(&chrono::Local)
                .format("%Y-%m-%d"),
            (report.scope.to - chrono::Duration::days(1))
                .with_timezone(&chrono::Local)
                .format("%Y-%m-%d"),
        ),
    };
    println!("{header} ({} rows)", report.rows.len());

    let max = report
        .rows
        .iter()
        .map(|r| r.duration_seconds)
        .max()
        .unwrap_or(0);
    for row in &report.rows {
        let total = chrono::Duration::seconds(row.duration_seconds);
        println!(
            "{:<48}  {:>8}  {}",
            truncate(&row.label, 48),
            duration_compact(total),
            bar(row.duration_seconds, max, 30),
        );
    }
    let total = chrono::Duration::seconds(report.total_seconds);
    println!("{:<48}  {:>8}", "Total", duration_compact(total));
}
