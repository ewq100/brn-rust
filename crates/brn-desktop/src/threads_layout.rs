//! Tile arrangement state. Pure and independent of rendering.

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) enum TileKind {
    Threads,
    Note,
    Chat,
    Linked,
    Notes,
    AllThreads,
    Overview,
    Actions,
    History,
    Settings,
    Compare,
    Connections,
}
impl TileKind {
    pub(super) const ALL: [Self; 12] = [
        Self::Threads,
        Self::Note,
        Self::Chat,
        Self::Linked,
        Self::Notes,
        Self::AllThreads,
        Self::Overview,
        Self::Actions,
        Self::History,
        Self::Settings,
        Self::Compare,
        Self::Connections,
    ];
    pub(super) fn key(self) -> &'static str {
        match self {
            Self::Threads => "threads",
            Self::Note => "note",
            Self::Chat => "chat",
            Self::Linked => "linked",
            Self::Notes => "notes",
            Self::AllThreads => "all-threads",
            Self::Overview => "overview",
            Self::Actions => "actions",
            Self::History => "history",
            Self::Settings => "settings",
            Self::Compare => "compare",
            Self::Connections => "connections",
        }
    }
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Threads => "chats",
            Self::Note => "note",
            Self::Chat => "chat",
            Self::Linked => "linked work",
            Self::Notes => "notes",
            Self::AllThreads => "all chats",
            Self::Overview => "overview",
            Self::Actions => "actions",
            Self::History => "what changed",
            Self::Settings => "settings",
            Self::Compare => "compare",
            Self::Connections => "connections",
        }
    }
    pub(super) fn default_weight(self) -> f32 {
        match self {
            Self::Threads => 0.62,
            Self::Note => 2.1,
            Self::Chat => 1.45,
            Self::Compare => 1.6,
            _ => 1.3,
        }
    }
    /// Narrow windows scroll the tile row rather than squeezing below this.
    pub(super) fn min_width(self) -> f32 {
        match self {
            Self::Threads => 170.,
            Self::Note | Self::Compare => 320.,
            _ => 260.,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Preset {
    Write,
    Chat,
    Solo,
}
impl Preset {
    pub(super) const ALL: [Self; 3] = [Self::Write, Self::Chat, Self::Solo];
    pub(super) fn key(self) -> &'static str {
        match self {
            Self::Write => "write",
            Self::Chat => "chat",
            Self::Solo => "solo",
        }
    }
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Write => "Write",
            Self::Chat => "Chat",
            Self::Solo => "Solo",
        }
    }
    fn tiles(self) -> &'static [(TileKind, f32)] {
        match self {
            Self::Write => &[
                (TileKind::Threads, 0.62),
                (TileKind::Note, 2.7),
                (TileKind::Chat, 1.45),
            ],
            Self::Chat => &[
                (TileKind::Threads, 0.62),
                (TileKind::Chat, 1.9),
                (TileKind::Linked, 0.95),
            ],
            Self::Solo => &[],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Tile {
    pub(super) kind: TileKind,
    pub(super) weight: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Layout {
    /// The preset last chosen while its arrangement is unchanged.
    preset: Option<Preset>,
    tiles: Vec<Tile>,
    focus: Option<TileKind>,
    maximized: Option<TileKind>,
}
impl Default for Layout {
    fn default() -> Self {
        let mut layout = Self {
            preset: None,
            tiles: vec![],
            focus: Some(TileKind::Chat),
            maximized: None,
        };
        layout.apply(Preset::Write);
        layout
    }
}
impl Layout {
    pub(super) fn preset(&self) -> Option<Preset> {
        self.preset
    }
    pub(super) fn focus(&self) -> Option<TileKind> {
        self.focus
    }
    pub(super) fn maximized(&self) -> Option<TileKind> {
        self.maximized
    }
    #[cfg(test)]
    pub(super) fn tiles(&self) -> &[Tile] {
        &self.tiles
    }
    pub(super) fn contains(&self, kind: TileKind) -> bool {
        self.tiles.iter().any(|tile| tile.kind == kind)
    }
    /// Tiles drawn now: only the maximized tile while one is maximized.
    pub(super) fn visible(&self) -> Vec<Tile> {
        match self.maximized {
            Some(kind) => self
                .tiles
                .iter()
                .filter(|tile| tile.kind == kind)
                .map(|tile| Tile {
                    weight: 1.,
                    ..*tile
                })
                .collect(),
            None => self.tiles.clone(),
        }
    }
    pub(super) fn apply(&mut self, preset: Preset) {
        self.maximized = None;
        self.preset = Some(preset);
        if preset == Preset::Solo {
            let kind = self
                .focus
                .filter(|kind| self.contains(*kind))
                .or_else(|| self.tiles.first().map(|tile| tile.kind))
                .unwrap_or(TileKind::Chat);
            self.tiles = vec![Tile { kind, weight: 1. }];
            self.focus = Some(kind);
            return;
        }
        self.tiles = preset
            .tiles()
            .iter()
            .map(|(kind, weight)| Tile {
                kind: *kind,
                weight: *weight,
            })
            .collect();
        if !self.focus.is_some_and(|kind| self.contains(kind)) {
            self.focus = Some(self.tiles[1].kind);
        }
    }
    /// Focus an open tile, or insert it after the focused tile.
    pub(super) fn open(&mut self, kind: TileKind) {
        if self.maximized.is_some_and(|max| max != kind) {
            self.maximized = None;
        }
        if !self.contains(kind) {
            let at = self
                .focus
                .and_then(|focus| self.tiles.iter().position(|tile| tile.kind == focus))
                .map_or(self.tiles.len(), |index| index + 1);
            self.tiles.insert(
                at,
                Tile {
                    kind,
                    weight: kind.default_weight(),
                },
            );
            self.preset = None;
        }
        self.focus = Some(kind);
    }
    pub(super) fn close(&mut self, kind: TileKind) {
        let Some(index) = self.tiles.iter().position(|tile| tile.kind == kind) else {
            return;
        };
        self.tiles.remove(index);
        self.preset = None;
        if self.maximized == Some(kind) {
            self.maximized = None;
        }
        if self.focus == Some(kind) {
            self.focus = self
                .tiles
                .get(index.saturating_sub(1))
                .or_else(|| self.tiles.first())
                .map(|tile| tile.kind);
        }
    }
    pub(super) fn toggle(&mut self, kind: TileKind) {
        if self.contains(kind) {
            self.close(kind);
        } else {
            self.open(kind);
        }
    }
    pub(super) fn set_focus(&mut self, kind: TileKind) {
        if self.contains(kind) {
            self.focus = Some(kind);
        }
    }
    pub(super) fn swap(&mut self, a: TileKind, b: TileKind) {
        let (Some(i), Some(j)) = (
            self.tiles.iter().position(|tile| tile.kind == a),
            self.tiles.iter().position(|tile| tile.kind == b),
        ) else {
            return;
        };
        if i != j {
            // Positions exchange; each slot keeps its width.
            let (wi, wj) = (self.tiles[i].weight, self.tiles[j].weight);
            self.tiles.swap(i, j);
            self.tiles[i].weight = wi;
            self.tiles[j].weight = wj;
            self.preset = None;
        }
    }
    pub(super) fn move_by(&mut self, kind: TileKind, delta: isize) {
        let Some(index) = self.tiles.iter().position(|tile| tile.kind == kind) else {
            return;
        };
        let target = index as isize + delta;
        if target >= 0 && (target as usize) < self.tiles.len() {
            let other = self.tiles[target as usize].kind;
            self.swap(kind, other);
        }
    }
    pub(super) fn can_move(&self, kind: TileKind, delta: isize) -> bool {
        self.tiles
            .iter()
            .position(|tile| tile.kind == kind)
            .is_some_and(|index| {
                let target = index as isize + delta;
                target >= 0 && (target as usize) < self.tiles.len()
            })
    }
    /// Move the gap after visible tile `boundary` to `fraction` of the row,
    /// keeping both neighbours at least 8% of the row.
    pub(super) fn resize(&mut self, boundary: usize, fraction: f32) {
        if self.maximized.is_some() || boundary + 1 >= self.tiles.len() || !fraction.is_finite() {
            return;
        }
        let total: f32 = self.tiles.iter().map(|tile| tile.weight).sum();
        let before: f32 = self.tiles[..boundary].iter().map(|tile| tile.weight).sum();
        let pair = self.tiles[boundary].weight + self.tiles[boundary + 1].weight;
        let minimum = total * 0.08;
        if pair < minimum * 2. {
            return;
        }
        let left = (fraction.clamp(0., 1.) * total - before).clamp(minimum, pair - minimum);
        self.tiles[boundary].weight = left;
        self.tiles[boundary + 1].weight = pair - left;
    }
    pub(super) fn toggle_maximize(&mut self, kind: TileKind) {
        if !self.contains(kind) {
            return;
        }
        self.maximized = if self.maximized == Some(kind) {
            None
        } else {
            Some(kind)
        };
        self.focus = Some(kind);
    }
    pub(super) fn cycle_focus(&mut self, delta: isize) {
        let tiles = self.visible();
        if tiles.is_empty() {
            return;
        }
        let index = self
            .focus
            .and_then(|focus| tiles.iter().position(|tile| tile.kind == focus))
            .unwrap_or(0) as isize;
        let next = (index + delta).rem_euclid(tiles.len() as isize) as usize;
        self.focus = Some(tiles[next].kind);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(layout: &Layout) -> Vec<TileKind> {
        layout.tiles().iter().map(|tile| tile.kind).collect()
    }

    #[test]
    fn presets_and_solo_follow_the_focused_tile() {
        let mut layout = Layout::default();
        assert_eq!(layout.preset(), Some(Preset::Write));
        assert_eq!(
            kinds(&layout),
            [TileKind::Threads, TileKind::Note, TileKind::Chat]
        );
        layout.apply(Preset::Chat);
        assert_eq!(
            kinds(&layout),
            [TileKind::Threads, TileKind::Chat, TileKind::Linked]
        );
        assert_eq!(layout.focus(), Some(TileKind::Chat));
        layout.set_focus(TileKind::Linked);
        layout.apply(Preset::Solo);
        assert_eq!(kinds(&layout), [TileKind::Linked]);
        layout.apply(Preset::Write);
        assert_eq!(
            layout.focus(),
            Some(TileKind::Note),
            "unavailable focus moves"
        );
    }

    #[test]
    fn open_close_toggle_insert_after_focus_and_mark_custom() {
        let mut layout = Layout::default();
        layout.set_focus(TileKind::Note);
        layout.open(TileKind::Notes);
        assert_eq!(
            kinds(&layout),
            [
                TileKind::Threads,
                TileKind::Note,
                TileKind::Notes,
                TileKind::Chat
            ]
        );
        assert_eq!(layout.focus(), Some(TileKind::Notes));
        assert_eq!(layout.preset(), None);
        layout.close(TileKind::Notes);
        assert_eq!(layout.focus(), Some(TileKind::Note));
        layout.toggle(TileKind::Threads);
        assert!(!layout.contains(TileKind::Threads));
        layout.toggle(TileKind::Threads);
        assert!(layout.contains(TileKind::Threads));
        for kind in kinds(&layout) {
            layout.close(kind);
        }
        assert!(layout.tiles().is_empty());
        assert_eq!(layout.focus(), None);
        layout.open(TileKind::Settings);
        assert_eq!(kinds(&layout), [TileKind::Settings]);
    }

    #[test]
    fn swap_and_move_keep_slot_widths() {
        let mut layout = Layout::default();
        let widths: Vec<f32> = layout.tiles().iter().map(|tile| tile.weight).collect();
        layout.swap(TileKind::Threads, TileKind::Chat);
        assert_eq!(
            kinds(&layout),
            [TileKind::Chat, TileKind::Note, TileKind::Threads]
        );
        let after: Vec<f32> = layout.tiles().iter().map(|tile| tile.weight).collect();
        assert_eq!(widths, after);
        assert!(!layout.can_move(TileKind::Chat, -1));
        layout.move_by(TileKind::Chat, 1);
        assert_eq!(
            kinds(&layout),
            [TileKind::Note, TileKind::Chat, TileKind::Threads]
        );
        layout.move_by(TileKind::Threads, 1);
        assert_eq!(layout.tiles()[2].kind, TileKind::Threads, "edge is a no-op");
    }

    #[test]
    fn resize_moves_one_gap_within_bounds() {
        let mut layout = Layout::default();
        let total: f32 = layout.tiles().iter().map(|tile| tile.weight).sum();
        let third = layout.tiles()[2].weight;
        layout.resize(0, 0.30);
        let tiles = layout.tiles();
        assert!((tiles[0].weight / total - 0.30).abs() < 1e-4);
        assert_eq!(tiles[2].weight, third, "other tiles keep width");
        let sum: f32 = tiles.iter().map(|tile| tile.weight).sum();
        assert!((sum - total).abs() < 1e-4);
        layout.resize(0, -5.);
        assert!((layout.tiles()[0].weight / total - 0.08).abs() < 1e-4);
        layout.resize(1, 2.0);
        assert!((layout.tiles()[2].weight / total - 0.08).abs() < 1e-4);
        let before = layout.clone();
        layout.resize(2, 0.5);
        layout.resize(0, f32::NAN);
        assert_eq!(layout, before, "invalid gaps are ignored");
    }

    #[test]
    fn maximize_restores_and_focus_cycles_visible_tiles() {
        let mut layout = Layout::default();
        layout.toggle_maximize(TileKind::Note);
        assert_eq!(layout.visible().len(), 1);
        assert_eq!(layout.visible()[0].kind, TileKind::Note);
        layout.resize(0, 0.5);
        layout.toggle_maximize(TileKind::Note);
        assert_eq!(layout.visible().len(), 3);
        layout.set_focus(TileKind::Chat);
        layout.cycle_focus(1);
        assert_eq!(layout.focus(), Some(TileKind::Threads));
        layout.cycle_focus(-1);
        assert_eq!(layout.focus(), Some(TileKind::Chat));
        layout.toggle_maximize(TileKind::Chat);
        layout.open(TileKind::Settings);
        assert_eq!(layout.maximized(), None, "opening another tile restores");
    }
}
