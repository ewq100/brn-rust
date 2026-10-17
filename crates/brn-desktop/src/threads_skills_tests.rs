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
