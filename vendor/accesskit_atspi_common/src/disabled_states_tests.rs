// BRN regression witnesses for upstream AccessKit fix 6ee0558b.
use super::*;
use accesskit::{Node as Data, Tree as TreeData, TreeUpdate};
use accesskit_consumer::TreeChangeHandler;

fn control(role: Role, disabled: bool, read_only: bool) -> Data {
    let mut node = Data::new(role);
    if disabled {
        node.set_disabled();
    }
    if read_only {
        node.set_read_only();
    }
    node
}

fn update(node: Data, initial: bool) -> TreeUpdate {
    TreeUpdate {
        nodes: vec![(LocalNodeId(0), node)],
        tree: initial.then(|| TreeData::new(LocalNodeId(0))),
        tree_id: TreeId::ROOT,
        focus: LocalNodeId(0),
    }
}

fn states(tree: &Tree) -> StateSet {
    let node = tree.state().root();
    NodeWrapper(&node).state(true)
}

#[test]
fn disabled_controls_never_report_enabled_or_sensitive() {
    for role in [Role::Button, Role::TextInput] {
        for disabled in [false, true] {
            for read_only in [false, true] {
                let tree = Tree::new(update(control(role, disabled, read_only), true), true);
                let states = states(&tree);
                let expected_read_only = !disabled && role == Role::TextInput && read_only;
                let expected_enabled = !disabled && !expected_read_only;
                assert_eq!(
                    states.contains(State::Enabled),
                    expected_enabled,
                    "{role:?}, disabled={disabled}, read_only={read_only}"
                );
                assert_eq!(states.contains(State::Sensitive), expected_enabled);
                assert_eq!(states.contains(State::ReadOnly), expected_read_only);
            }
        }
    }
}

struct Updates(usize);
impl TreeChangeHandler for Updates {
    fn node_added(&mut self, _: &Node) {}
    fn node_updated(&mut self, _: &Node, _: &Node) {
        self.0 += 1;
    }
    fn focus_moved(&mut self, _: Option<&Node>, _: Option<&Node>) {}
    fn node_removed(&mut self, _: &Node) {}
}

#[test]
fn disabled_enabled_disabled_updates_preserve_translation() {
    let mut tree = Tree::new(update(control(Role::Button, true, false), true), true);
    let mut changes = Updates(0);
    assert!(!states(&tree).contains(State::Enabled | State::Sensitive));
    for disabled in [false, true] {
        tree.update_and_process_changes(
            update(control(Role::Button, disabled, false), false),
            &mut changes,
        );
        assert_eq!(states(&tree).contains(State::Enabled), !disabled);
        assert_eq!(states(&tree).contains(State::Sensitive), !disabled);
    }
    assert_eq!(changes.0, 2);
}
