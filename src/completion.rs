use std::collections::BTreeMap;

use objc2_app_kit::NSRunningApplication;
use usage::complete::{Candidate, CompleteCtx};

use crate::cli::{Cli, CliArgs};

pub(crate) fn app_candidates(
    _partial: &<CliArgs as usage::spec::CommandArgs>::Partial,
    _ctx: &CompleteCtx<'_>,
) -> Vec<Candidate<'static>> {
    let mut candidates = BTreeMap::<String, String>::new();
    for (_pid, (name, bundle_id)) in window_owners() {
        if let Some(bundle_id) = bundle_id.as_ref() {
            candidates.insert(
                bundle_id.clone(),
                name.clone()
                    .unwrap_or_else(|| "running application".to_owned()),
            );
        }
        if let Some(name) = name {
            let description = bundle_id.unwrap_or_else(|| "application with windows".to_owned());
            candidates.entry(name).or_insert(description);
        }
    }

    candidates
        .into_iter()
        .map(|(value, description)| Candidate::described(value, description))
        .collect()
}

pub(crate) fn pid_candidates(
    _partial: &<CliArgs as usage::spec::CommandArgs>::Partial,
    _ctx: &CompleteCtx<'_>,
) -> Vec<Candidate<'static>> {
    window_owners()
        .into_iter()
        .map(|(pid, (name, _bundle_id))| {
            Candidate::described(
                pid.to_string(),
                name.unwrap_or_else(|| "application with windows".to_owned()),
            )
        })
        .collect()
}

pub(crate) fn window_candidates(
    _partial: &<CliArgs as usage::spec::CommandArgs>::Partial,
    _ctx: &CompleteCtx<'_>,
) -> Vec<Candidate<'static>> {
    crate::collector::window_completion_candidates()
        .into_iter()
        .map(|(window_id, title, owner_name, owner_pid)| {
            let title = title.unwrap_or_else(|| "<untitled>".to_owned());
            let owner = owner_name.unwrap_or_else(|| "<unknown app>".to_owned());
            Candidate::described(
                window_id.to_string(),
                format!("{title} — {owner} (pid {owner_pid})"),
            )
        })
        .collect()
}

fn window_owners() -> BTreeMap<i32, (Option<String>, Option<String>)> {
    let mut owners = BTreeMap::new();

    for (_window_id, _title, owner_name, pid) in crate::collector::window_completion_candidates() {
        let application = NSRunningApplication::runningApplicationWithProcessIdentifier(pid);
        let name = owner_name.or_else(|| {
            application
                .as_ref()
                .and_then(|app| app.localizedName().map(|value| value.to_string()))
        });
        let bundle_id =
            application.and_then(|app| app.bundleIdentifier().map(|value| value.to_string()));

        owners.entry(pid).or_insert((name, bundle_id));
    }

    owners
}

pub(crate) fn handle_request() -> Option<i32> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.first().and_then(|arg| arg.to_str()) != Some("completion") {
        return None;
    }

    let shell_name = match args.get(1).and_then(|arg| arg.to_str()) {
        Some("--shell") => args.get(2).and_then(|arg| arg.to_str()),
        Some(shell) => Some(shell),
        None => None,
    };
    let Some(shell_name) = shell_name else {
        eprintln!("usage: screen-dump completion <bash|elvish|fish|nu|powershell|zsh>");
        return Some(2);
    };
    let Some(shell) = usage::complete::Shell::from_name(shell_name) else {
        eprintln!("screen-dump: unsupported completion shell: {shell_name}");
        return Some(2);
    };

    print!("{}", Cli::completion_script(shell));
    Some(0)
}
