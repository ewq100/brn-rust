//! Real owner service and rendered native controls, synthetic files only.
use super::*;
use gpui_kit::{TestAppContext, test::TestWindowExt};
#[gpui_kit::test]
fn native_skill_install_enable_select_disable_and_reopen(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = root.path().join("workspace");
    let package = root.path().join("package");
    std::fs::create_dir(&package).unwrap();
    std::fs::write(package.join("SKILL.md"),"---\nname: native-review\ndescription: Synthetic native instructions.\n---\nNATIVE_SKILL_CANARY\n").unwrap();
    let mut workspace = Workspace::open(&data).unwrap();
    focus_tests::configure_offline_fixture(&mut workspace);
    let (window, view) = cx.update(|cx| {
        gpui_kit::open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                    point(px(0.), px(0.)),
                    size(px(1400.), px(1600.)),
                ))),
                ..Default::default()
            },
            cx,
            |window, cx| {
                cx.new(|cx| {
                    let mut ui = ThreadsUi::new(workspace, data.clone(), window, cx);
                    ui.poll = None;
                    ui.layout.open(TileKind::Settings);
                    ui.settings_section = SettingsSection::Skills;
                    ui
                })
            },
        )
        .unwrap()
    });
    cx.run_until_parked();
    cx.update_window(window, |_, window, cx| {
        assert!(window.find("skill-install-folder").visible());
        view.update(cx, |ui, cx| ui.skill_folder_chosen(package.clone(), cx));
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(window, |_, window, cx| {
        assert!(
            window
                .find("skill-enable-installed:native-review")
                .visible()
        );
        view.update(cx, |ui, cx| {
            let info = ui.workspace.inspect_skill("native-review").unwrap().info;
            assert!(
                !ui.workspace
                    .settings()
                    .unwrap()
                    .skills
                    .iter()
                    .any(|b| b.id == info.id)
            );
            ui.skill_action(
                skills_view::SkillAction::Enable,
                &info.id,
                &info.digest,
                None,
                cx,
            );
            ui.skill_action(
                skills_view::SkillAction::Select,
                &info.id,
                &info.digest,
                None,
                cx,
            );
            assert!(
                ui.workspace
                    .settings()
                    .unwrap()
                    .skills
                    .iter()
                    .any(|b| b.id == info.id && b.selected)
            );
            ui.skill_action(
                skills_view::SkillAction::Inspect,
                &info.id,
                &info.digest,
                None,
                cx,
            );
            assert!(
                ui.skill_view
                    .inspection
                    .as_ref()
                    .unwrap()
                    .contains("NATIVE_SKILL_CANARY")
            );
            ui.skill_action(
                skills_view::SkillAction::Disable,
                &info.id,
                &info.digest,
                None,
                cx,
            );
            assert!(
                ui.workspace
                    .settings()
                    .unwrap()
                    .skills
                    .iter()
                    .any(|b| b.id == info.id && !b.enabled && !b.selected)
            );
        });
    })
    .unwrap();
    cx.run_until_parked();
    let reopened = Workspace::open_existing_no_upgrade(&data).unwrap();
    assert_eq!(
        reopened.inspect_skill("native-review").unwrap().info.name,
        "native-review"
    );
    assert!(
        reopened
            .settings()
            .unwrap()
            .skills
            .iter()
            .any(|b| b.id == "installed:native-review" && !b.enabled)
    );
    cx.update_window(window, |_, _, cx| {
        view.update(cx, |ui, cx| {
            let info = ui.workspace.inspect_skill("native-review").unwrap().info;
            ui.skill_action(skills_view::SkillAction::Remove, &info.id, &info.digest, None, cx);
            assert!(ui.workspace.inspect_skill("native-review").is_err());
            let retained = ui.skill_view.rows.iter().find(|row| row.id == info.id).unwrap();
            assert!(!retained.enabled && !retained.available, "removing an already disabled artifact must invalidate the native inventory snapshot");
        });
    }).unwrap();
}

#[gpui_kit::test]
fn native_inventory_can_enable_one_package_in_each_scope_without_copying_policy(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = root.path().join("workspace");
    let package = root.path().join("package");
    std::fs::create_dir(&package).unwrap();
    std::fs::write(package.join("SKILL.md"), "---\nname: scoped-native-review\ndescription: Global owner inventory, scoped opt-in.\n---\nNative instruction fixture.\n").unwrap();
    let mut workspace = Workspace::open(&data).unwrap();
    focus_tests::configure_offline_fixture(&mut workspace);
    let info = workspace.install_skill(&package).unwrap();
    let p1 = workspace
        .project_command(
            "native-skills-p1",
            ProjectCommand::Create {
                title: "P1 policy private".into(),
            },
        )
        .unwrap()
        .writes[0]
        .after
        .id
        .clone();
    let p2 = workspace
        .project_command(
            "native-skills-p2",
            ProjectCommand::Create { title: "P2".into() },
        )
        .unwrap()
        .writes[0]
        .after
        .id
        .clone();
    workspace
        .set_skill_enabled(&info.id, &info.digest, Some(&p1), true)
        .unwrap();
    workspace.select_skill(Some(&info.id), Some(&p1)).unwrap();
    let p1_binding = workspace.settings().unwrap().skills[0].clone();
    assert!(p1_binding.enabled && p1_binding.selected);
    assert_eq!(p1_binding.project.as_deref(), Some(p1.as_str()));
    let (window, view) = cx.update(|cx| {
        gpui_kit::open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                    point(px(0.), px(0.)),
                    size(px(1400.), px(1600.)),
                ))),
                ..Default::default()
            },
            cx,
            |window, cx| {
                cx.new(|cx| {
                    let mut ui = ThreadsUi::new(workspace, data.clone(), window, cx);
                    ui.poll = None;
                    ui.layout.open(TileKind::Settings);
                    ui.settings_section = SettingsSection::Skills;
                    ui
                })
            },
        )
        .unwrap()
    });
    cx.run_until_parked();
    for project in [None, Some(p2.clone())] {
        cx.update_window(window, |_, _, cx| {
            view.update(cx, |ui, _| {
                ui.chat = ChatTarget::New;
                ui.selected_project = project.clone();
                ui.refresh_skills();
                let row = ui
                    .skill_view
                    .rows
                    .iter()
                    .find(|r| r.id == info.id)
                    .expect("global owner inventory must remain available in another scope");
                assert!(row.available && !row.enabled && !row.selected);
                assert_eq!(row.project, project);
                assert!(
                    !serde_json::to_string(&ui.skill_view.rows)
                        .unwrap()
                        .contains(&p1)
                );
                assert!(
                    !ui.workspace
                        .scoped_skills(project.as_deref())
                        .unwrap()
                        .iter()
                        .any(|r| r.id == info.id)
                );
            })
        })
        .unwrap();
        cx.run_until_parked();
        cx.update_window(window, |_, window, cx| {
            assert!(
                window
                    .find("skill-enable-installed:scoped-native-review")
                    .visible()
            );
            view.update(cx, |ui, cx| {
                ui.skill_action(
                    skills_view::SkillAction::Enable,
                    &info.id,
                    &info.digest,
                    project.clone(),
                    cx,
                );
                let policy = ui.workspace.settings().unwrap().skills;
                assert!(policy.contains(&p1_binding));
                let current = policy
                    .iter()
                    .find(|b| b.id == info.id && b.project == project)
                    .unwrap();
                assert!(current.enabled && !current.selected);
                let scoped = ui.workspace.scoped_skills(project.as_deref()).unwrap();
                let current = scoped.iter().find(|s| s.id == info.id).unwrap();
                assert!(current.enabled && !current.selected);
                assert_eq!(current.project, project);
                assert!(!serde_json::to_string(&scoped).unwrap().contains(&p1));
            });
        })
        .unwrap();
        cx.run_until_parked();
    }
}
