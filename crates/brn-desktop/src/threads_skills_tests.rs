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

#[gpui_kit::test]
fn native_stale_skill_binding_can_inspect_and_explicitly_enable_current_digest(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = root.path().join("workspace");
    let mut workspace = Workspace::open(&data).unwrap();
    focus_tests::configure_offline_fixture(&mut workspace);
    let current = workspace
        .skills()
        .unwrap()
        .into_iter()
        .find(|package| package.id == "maintain-notes")
        .expect("the packaged maintenance skill is in the current inventory");
    let stale_digest = "0000000000000000000000000000000000000000000000000000000000000001";
    let op = workspace
        .store
        .allocate_operation("native-skill-stale-binding")
        .unwrap();
    workspace
        .store
        .change_skill_policy(
            &op,
            &brn_threads_core::SkillPolicyChange::Set(brn_threads_core::SkillBinding {
                id: current.id.clone(),
                digest: stale_digest.into(),
                project: None,
                enabled: true,
                selected: true,
            }),
            &brn_threads_core::HostAuthority::owner("synthetic stale skill policy fixture"),
        )
        .unwrap();
    let project = workspace
        .project_command(
            "native-stale-skill-project",
            ProjectCommand::Create {
                title: "Scope-change fixture".into(),
            },
        )
        .unwrap()
        .writes[0]
        .after
        .id
        .clone();
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
        let inspect = window.find("skill-inspect-maintain-notes");
        assert_ne!(inspect.disabled(), Some(true));
        assert!(window.find("skill-adopt-current-maintain-notes").visible());
        assert_eq!(
            window.find("skill-enable-maintain-notes").label(),
            Some("Disable"),
            "the former Enable action cannot adopt a different digest"
        );
        view.update(cx, |ui, _| {
            let row = ui
                .skill_view
                .rows
                .iter()
                .find(|row| row.id == current.id)
                .unwrap();
            assert!(!row.available && row.enabled && row.selected);
            assert!(
                ui.skill_view
                    .inventory
                    .iter()
                    .any(|package| package.id == current.id && package.digest == current.digest)
            );
            assert_eq!(
                ui.workspace
                    .settings()
                    .unwrap()
                    .skills
                    .into_iter()
                    .find(|binding| binding.id == current.id && binding.project.is_none())
                    .unwrap()
                    .digest,
                stale_digest
            );
        });
        window.click("skill-inspect-maintain-notes", cx);
    })
    .unwrap();
    cx.run_until_parked();

    cx.update_window(window, |_, _, cx| {
        view.update(cx, |ui, _| {
            let inspection = ui.skill_view.inspection.as_ref().unwrap();
            assert!(inspection.contains(&current.digest));
            assert!(inspection.contains("Reviewing a mechanical knowledge check"));
            let before = ui.workspace.settings().unwrap().skills;
            ui.force_refresh_skills();
            assert_eq!(ui.workspace.settings().unwrap().skills, before);

            ui.selected_project = Some(project.clone());
            ui.refresh_skills();
            ui.skill_action(
                skills_view::SkillAction::AdoptCurrent,
                &current.id,
                &current.digest,
                None,
                cx,
            );
            assert_eq!(ui.workspace.settings().unwrap().skills, before);

            ui.selected_project = None;
            ui.refresh_skills();
            ui.skill_action(
                skills_view::SkillAction::AdoptCurrent,
                &current.id,
                stale_digest,
                None,
                cx,
            );
            assert_eq!(ui.workspace.settings().unwrap().skills, before);
        });
        window.render_frame(cx);
        window.click("skill-adopt-current-maintain-notes", cx);
    })
    .unwrap();
    cx.run_until_parked();

    cx.update_window(window, |_, _, cx| {
        view.update(cx, |ui, _| {
            let binding = ui
                .workspace
                .settings()
                .unwrap()
                .skills
                .into_iter()
                .find(|binding| binding.id == current.id && binding.project.is_none())
                .unwrap();
            assert_eq!(binding.digest, current.digest);
            assert!(binding.enabled);
            assert!(
                !binding.selected,
                "enable does not select or load the skill"
            );
        });
    })
    .unwrap();
}
