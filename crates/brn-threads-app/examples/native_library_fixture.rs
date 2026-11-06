//! Prepare an offline S5 Library fixture (UX rebuild) through checked services:
//! ordinary current articles, one updated under the standing delegation, a
//! protected full imported reference with its Source, a loose Source, topic
//! pages linked from the Topics note (one article under two topics), two
//! projects with a project-only document and chats, and an archived chat.
//! `--notes N` adds N more synthetic notes for the list responsiveness check.
//! Starts no provider and claims no native interaction.
use brn_threads_app::*;
use std::{fs, path::PathBuf};

const LONG_ARTICLE: &str =
    include_str!("../../../docs/product/ux-rebuild/fixtures/reading-quality-article.md");

fn applied(
    outcome: ApplyOutcome,
    what: &str,
) -> std::result::Result<Receipt, Box<dyn std::error::Error>> {
    match outcome {
        ApplyOutcome::Applied(receipt) => Ok(receipt),
        _ => Err(format!("{what} did not apply").into()),
    }
}

fn put(id: String, data: RecordData) -> Put {
    Put {
        id,
        expected_version: None,
        archived: false,
        data,
    }
}

fn link(from: &Record, to: &Record) -> RecordData {
    RecordData::Link(Link {
        from: from.id.clone(),
        to: to.id.clone(),
        relation: relations::RELATED.into(),
    })
}

fn project(app: &mut Workspace, key: &str, title: &str) -> AppResult<Record> {
    app.project_command(
        key,
        ProjectCommand::Create {
            title: title.into(),
        },
    )?
    .writes
    .into_iter()
    .find(|write| matches!(write.after.data, RecordData::Project(_)))
    .map(|write| write.after)
    .ok_or_else(|| AppError::Invalid("Project creation returned no Project".into()))
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (root, extra) = match args.as_slice() {
        [flag, root] if flag == "--output" => (PathBuf::from(root), 0usize),
        [flag, root, count, n] if flag == "--output" && count == "--notes" => {
            (PathBuf::from(root), n.parse()?)
        }
        _ => {
            return Err(
                "Usage: native_library_fixture --output /absolute/new-root [--notes N]".into(),
            );
        }
    };
    if !root.is_absolute() || root.file_name().is_none() {
        return Err("--output must name a new absolute root directory".into());
    }
    fs::create_dir(&root)?;
    let data = root.join("workspace");
    let mut app = Workspace::open_without_default_credentials(&data)?;
    let mut settings = app.settings()?;
    settings.provider = "offline-fixture".into();
    settings.auth_file = None;
    settings.credentials_dir = None;
    settings.helper = None;
    app.configure(settings)?;
    applied(
        app.choose_delegation(DelegationChoice::MaintainOrdinaryNotes)?,
        "delegation choice",
    )?;

    // Ordinary current articles.
    let delivery = app.new_note(
        "Harbor delivery plan",
        "# Harbor delivery plan\n\nDeliveries arrive at berth one on weekday mornings; the crane crew confirms slots a day ahead.\n\n## Open questions\n\n- Weekend slots\n",
    )?;
    let berth = app.new_note(
        "Berth schedule",
        "# Berth schedule\n\nBerth one serves cargo, berth two serves the ferry. Changes are posted on Fridays.\n",
    )?;
    let tides = app.new_note(
        "Tide windows",
        "# Tide windows\n\nLoaded barges need at least 3.2 m of water; the morning window is usually the safer one.\n",
    )?;
    let crew = app.new_note(
        "Crew onboarding",
        "# Crew onboarding\n\nNew crew members shadow a shift, then complete the crane safety check before working alone.\n",
    )?;
    let long = app.new_note("Reading quality fixture", LONG_ARTICLE)?;

    // Topic pages and the Topics note; Harbor delivery sits under both.
    let operations = app.new_note(
        "Harbor operations",
        "# Harbor operations\n\nHow cargo moves through the harbor: deliveries, berths and tides.\n",
    )?;
    let people = app.new_note(
        "People and onboarding",
        "# People and onboarding\n\nWho works the harbor and how new people start.\n",
    )?;
    let (topics, outcome) = app.create_topics_note()?;
    applied(outcome, "Topics note")?;
    let key = "s5-topic-links";
    let op = app.store.allocate_operation(key)?;
    let pairs = [
        (&topics, &operations),
        (&topics, &people),
        (&operations, &delivery),
        (&operations, &berth),
        (&operations, &tides),
        (&people, &crew),
        (&people, &delivery),
        (&berth, &tides),
    ];
    let writes = pairs
        .iter()
        .enumerate()
        .map(|(i, (from, to))| put(op.creation_id(i), link(from, to)))
        .collect();
    applied(
        app.owner_change(
            key,
            &ChangeRequest {
                reason: "Synthetic topic and related links".into(),
                writes,
                inputs: vec![],
            },
        )?,
        "topic links",
    )?;

    // A protected full imported reference with its Source, and a loose
    // Source whose original BRN did not retain.
    let key = "s5-reference";
    let op = app.store.allocate_operation(key)?;
    let source = |label: &str, locator: &str, version: Option<&str>| {
        RecordData::Source(Source {
            chat: None,
            locator: locator.into(),
            label: label.into(),
            observed_at: "2026-11-02T08:30:00Z".into(),
            external_version: version.map(str::to_owned),
            outcome: "Converted to a readable note".into(),
            gaps: vec![],
            report: None,
        })
    };
    let survey_source = op.creation_id(0);
    let mut survey = Note::working(
        "Harbor survey 2025",
        "# Harbor survey 2025\n\nThe 2025 depth survey of berths one and two, imported in full.\n\n| Berth | Depth |\n| --- | --- |\n| One | 3.4 m |\n| Two | 4.1 m |\n",
    );
    survey.protected = true;
    survey.import = Some(ImportCoverage {
        source: survey_source.clone(),
        full_note: true,
        gaps: vec![],
        report: None,
    });
    let reference = applied(
        app.owner_change(
            key,
            &ChangeRequest {
                reason: "Synthetic full imported reference".into(),
                writes: vec![
                    put(
                        survey_source,
                        source(
                            "harbor-survey-2025.pdf",
                            "/private/tmp/brn-s5-missing/harbor-survey-2025.pdf",
                            Some("Survey edition 2025-06"),
                        ),
                    ),
                    put(
                        op.creation_id(1),
                        source("tide-gauge-export.csv", "tide-gauge-export.csv", None),
                    ),
                    put(op.creation_id(2), RecordData::Note(survey)),
                ],
                inputs: vec![],
            },
        )?,
        "reference import",
    )?;

    // One ordinary update committed under the standing delegation.
    let current = app
        .store
        .record(&berth.id)?
        .ok_or("berth article disappeared")?;
    let RecordData::Note(mut updated) = current.data.clone() else {
        return Err("berth record is not a Note".into());
    };
    updated
        .markdown
        .push_str("\nFrom December, berth two also takes small cargo on Tuesdays.\n");
    let op = app.store.allocate_operation("s5-delegated-update")?;
    app.store.prepare(
        &op,
        &ChangeRequest {
            reason: "Update the berth schedule from the harbor notice".into(),
            writes: vec![Put {
                id: berth.id.clone(),
                expected_version: Some(current.version),
                archived: false,
                data: RecordData::Note(updated),
            }],
            inputs: vec![],
        },
    )?;
    applied(
        app.store.apply(
            &op,
            &HostAuthority::maintenance("Synthetic delegated maintenance"),
        )?,
        "delegated update",
    )?;

    // Two projects: a project-only document, a shared linked article, chats.
    let harbor = project(&mut app, "s5-project-harbor", "Harbor expansion")?;
    app.project_command(
        "s5-project-harbor-configure",
        ProjectCommand::Configure {
            project: harbor.id.clone(),
            version: harbor.version,
            title: "Harbor expansion".into(),
            instructions: "Plan the second berth expansion. Prefer short answers with dates."
                .into(),
            use_brn_knowledge: true,
        },
    )?;
    app.project_command(
        "s5-project-harbor-doc",
        ProjectCommand::CreateDocument {
            project: harbor.id.clone(),
            title: "Expansion budget draft".into(),
            markdown:
                "# Expansion budget draft\n\nProject-only working figures for the second berth.\n"
                    .into(),
        },
    )?;
    app.project_command(
        "s5-project-harbor-link",
        ProjectCommand::LinkDocument {
            project: harbor.id.clone(),
            note: delivery.id.clone(),
        },
    )?;
    app.project_command(
        "s5-project-harbor-chat",
        ProjectCommand::CreateChat {
            project: harbor.id.clone(),
            title: "Expansion planning".into(),
        },
    )?;
    let onboarding = project(&mut app, "s5-project-onboarding", "Crew onboarding 2027")?;
    app.project_command(
        "s5-project-onboarding-chat",
        ProjectCommand::CreateChat {
            project: onboarding.id.clone(),
            title: "Spring intake questions".into(),
        },
    )?;

    // General chats: one open with a saved message, one archived.
    let weekly = app.new_thread("Weekly tide questions")?;
    app.post_message(&weekly.id, "Which tide window suits the Friday barge?")?;
    let old = app.new_thread("Old berth question")?;
    app.post_message(&old.id, "Was berth two closed in October?")?;
    applied(app.resolve_thread(&old.id)?, "archive chat")?;

    for i in 0..extra {
        app.new_note(
            &format!("Synthetic note {i:05}"),
            &format!(
                "# Synthetic note {i:05}\n\nGenerated for the Library responsiveness check.\n"
            ),
        )?;
    }

    let notes = [
        &delivery,
        &berth,
        &tides,
        &crew,
        &long,
        &operations,
        &people,
        &topics,
    ];
    println!("Fixture workspace: {}", data.display());
    for note in notes {
        println!("note {} {}", note.id, title_of(note));
    }
    println!(
        "reference {}",
        reference
            .writes
            .iter()
            .find(|w| matches!(w.after.data, RecordData::Note(_)))
            .map_or("", |w| w.after.id.as_str())
    );
    println!("projects {} {}", harbor.id, onboarding.id);
    Ok(())
}

fn title_of(record: &Record) -> &str {
    match &record.data {
        RecordData::Note(note) => &note.title,
        _ => "",
    }
}
