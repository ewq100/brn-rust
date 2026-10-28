//! Prepare a fresh offline fixture for follow-up views and Capture Cancel observation.
//! This generator never starts a Run, confirms Capture, archives a chat or invokes a picker.
use brn_threads_app::*;
use serde_json::json;
use std::{fs, path::PathBuf};

const CHAT_TITLE: &str = "Synthetic outcomes conversation";
const SAVED_BODY: &str = "Already retained: the synthetic team chose the north loading bay.\n";
const SENTINEL_BODY: &str = "Unrelated native outcomes sentinel — café, õun, 日本語.\n";

#[cfg(unix)]
fn local_dates() -> std::result::Result<[String; 3], Box<dyn std::error::Error>> {
    let epoch: libc::time_t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs()
        .try_into()?;
    let mut dates = Vec::new();
    for delta in [-1, 0, 1] {
        // libc writes one valid tm into our owned initialized storage; no pointer escapes.
        let mut local: libc::tm = unsafe { std::mem::zeroed() };
        if unsafe { libc::localtime_r(&epoch, &mut local) }.is_null() {
            return Err("local calendar date unavailable; fixture refuses to guess".into());
        }
        // Noon plus calendar-day normalization handles DST; this is not epoch ±24 hours.
        local.tm_hour = 12;
        local.tm_min = 0;
        local.tm_sec = 0;
        local.tm_mday += delta;
        local.tm_isdst = -1;
        if unsafe { libc::mktime(&mut local) } == -1 {
            return Err("cannot normalize local fixture date".into());
        }
        dates.push(format!(
            "{:04}-{:02}-{:02}",
            local.tm_year + 1900,
            local.tm_mon + 1,
            local.tm_mday
        ));
    }
    dates.try_into().map_err(|_| "expected three dates".into())
}
#[cfg(not(unix))]
fn local_dates() -> std::result::Result<[String; 3], Box<dyn std::error::Error>> {
    Err("native outcomes observation currently requires a supported Unix host".into())
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 2 || args[0] != "--output" {
        return Err("Usage: native_outcomes_fixture --output /absolute/new-root".into());
    }
    let root = PathBuf::from(&args[1]);
    if !root.is_absolute() || root.file_name().is_none() {
        return Err("--output must name a new absolute root directory".into());
    }
    let parent = root.parent().ok_or("--output needs an existing parent")?;
    if fs::canonicalize(parent)? != parent {
        return Err("--output parent must exist and use its canonical path".into());
    }
    let [yesterday, today, tomorrow] = local_dates()?;
    // Refuse existing paths atomically; no existing workspace is opened or modified.
    fs::create_dir(&root)?;
    let data = root.join("workspace");
    let mut app = Workspace::open_without_default_credentials(&data)?;
    let mut settings = app.settings()?;
    settings.provider = "offline-fixture".into();
    settings.model = "synthetic-no-provider".into();
    settings.auth_file = None;
    settings.credentials_dir = None;
    settings.helper = None;
    app.configure(settings)?;
    let saved = app.new_note("Already saved synthetic decision", SAVED_BODY)?;
    let sentinel = app.new_note("Unrelated native outcomes sentinel", SENTINEL_BODY)?;
    let chat = app.new_thread(CHAT_TITLE)?;
    app.post_message(
        &chat.id,
        "Offline synthetic conversation: the north loading bay decision is already saved in 'Already saved synthetic decision'. A later idea to label the shelves is only in this chat. No owner or date was agreed for that idea. Do not start a model or save additional outcomes during this native Cancel observation.",
    )?;
    let key = "native-outcomes-actions";
    let op = app.store.allocate_operation(key)?;
    let specifications = [
        (
            "Yesterday open follow-up",
            ActionState::Open,
            Some(yesterday.clone()),
            Some(tomorrow.clone()),
            None,
        ),
        (
            "Today waiting follow-up",
            ActionState::Waiting,
            Some(today.clone()),
            Some(yesterday.clone()),
            Some("Synthetic supplier"),
        ),
        (
            "Tomorrow open follow-up",
            ActionState::Open,
            Some(tomorrow.clone()),
            Some(yesterday.clone()),
            None,
        ),
        (
            "Undated waiting follow-up",
            ActionState::Waiting,
            None,
            Some(yesterday.clone()),
            Some("Synthetic approval"),
        ),
        (
            "Undated open follow-up",
            ActionState::Open,
            None,
            Some(tomorrow.clone()),
            None,
        ),
    ];
    let writes = specifications
        .into_iter()
        .enumerate()
        .map(
            |(i, (description, state, follow_up, due, waiting_on))| Put {
                id: op.creation_id(i),
                expected_version: None,
                archived: false,
                data: RecordData::Action(Action {
                    kept_outside_project: false,
                    chat: None,
                    description: description.into(),
                    state,
                    internal: false,
                    evidence: None,
                    due,
                    owner: Some("Synthetic owner".into()),
                    waiting_on: waiting_on.map(str::to_owned),
                    follow_up,
                }),
            },
        )
        .collect();
    let result = app.owner_change(
        key,
        &ChangeRequest {
            reason: "Explicit synthetic owner follow-up fixture; no model or completion event"
                .into(),
            writes,
            inputs: vec![],
        },
    )?;
    let ApplyOutcome::Applied(receipt) = result else {
        return Err("checked synthetic Action creation did not apply".into());
    };
    drop(app);
    let app = Workspace::open_without_default_credentials(&data)?;
    for expected in [&saved, &sentinel, &chat] {
        if app.store.record(&expected.id)?.as_ref() != Some(expected) {
            return Err("saved fixture record changed while constructing Actions".into());
        }
    }
    let records = app.records()?;
    if records.iter().any(|r| matches!(r.data, RecordData::Run(_))) {
        return Err("offline native fixture unexpectedly contains a Run".into());
    }
    let manifest = json!({
        "format": "BRN native outcomes fixture v1", "workspace": "workspace",
        "provider": "offline-fixture", "model": "synthetic-no-provider", "live_runs": 0,
        "chat_id": chat.id, "chat_title": CHAT_TITLE, "saved_note": saved, "sentinel": sentinel,
        "local_dates": { "yesterday": yesterday, "today": today, "tomorrow": tomorrow },
        "expected_groups": { "Overdue follow-ups": "Yesterday open follow-up", "Follow up today": "Today waiting follow-up", "Upcoming follow-ups": "Tomorrow open follow-up", "Waiting": "Undated waiting follow-up", "Open": "Undated open follow-up" },
        "actions_receipt": receipt,
        "capture_observation": "Open Capture outcomes confirmation, inspect disclosure, Cancel. Never choose Propose outcomes.",
        "expected_unchanged": "all accepted records/history/candidate inventory; no Run, capture request, completion evidence or archive",
        "native_observation": "not performed by fixture generator"
    });
    for (name, value) in [
        ("manifest.json", manifest),
        ("initial-records.json", serde_json::to_value(records)?),
        (
            "initial-history.json",
            serde_json::to_value(app.history()?)?,
        ),
    ] {
        fs::write(
            root.join(name),
            format!("{}\n", serde_json::to_string_pretty(&value)?),
        )?;
    }
    println!("Fixture workspace: {}", data.display());
    println!("Fixture manifest: {}", root.join("manifest.json").display());
    Ok(())
}
