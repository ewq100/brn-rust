//! Native keyboard equivalents supplied by GPUI text widgets.

pub(super) const SELECT_ALL_KEY: &str = if cfg!(target_os = "macos") {
    "cmd-a"
} else {
    "ctrl-a"
};

pub(super) const LINE_START_KEY: &str = if cfg!(target_os = "macos") {
    "cmd-left"
} else {
    "home"
};
