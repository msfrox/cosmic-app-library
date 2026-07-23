use crate::config::APP_ID;
use crate::fl;
use cosmic::cosmic_config::cosmic_config_derive::CosmicConfigEntry;
use cosmic::cosmic_config::{
    CosmicConfigEntry, {self},
};
use cosmic::desktop::DesktopEntryData;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, LazyLock};
use std::vec;

static HOME: LazyLock<AppGroup> = LazyLock::new(|| AppGroup {
    name: "cosmic-library-home".to_string(),
    icon: "user-home-symbolic".to_string(),
    filter: FilterType::None,
    keep_in_home: false,
});

/// Sentinel group index for the built-in Favorites group. Favorites entries are
/// stored separately from `groups` so favorited apps still appear in Home.
pub const FAVORITES_GROUP: usize = usize::MAX;

/// A Windows-11-style folder: a small tile grouping a handful of apps inside
/// Home or Favorites, shown in place of the individual app tiles. Unlike
/// `AppGroup`, folders are not a separate view reached from the bottom bar —
/// they render inline in whichever view they belong to.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct AppFolder {
    pub name: String,
    pub apps: Vec<String>,
    /// If true, this folder lives in Favorites (its apps are excluded from
    /// `AppLibraryConfig::favorites` while inside it, and are returned to
    /// `favorites` if the folder dissolves). If false, it lives in Home
    /// (its apps are hidden from the Home grid while inside it).
    #[serde(default)]
    pub in_favorites: bool,
}

static FAVORITES: LazyLock<AppGroup> = LazyLock::new(|| AppGroup {
    name: "cosmic-favorites".to_string(),
    icon: "starred-symbolic".to_string(),
    filter: FilterType::None,
    keep_in_home: false,
});

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub enum FilterType {
    /// A list of application IDs to include in the group.
    AppIds(Vec<String>),
    Categories {
        categories: Vec<String>,
        /// The ID of applications which may not match the categories, but should be included anyway.
        exclude: Vec<String>,
        /// The ID of applications which should be excluded from the results.
        include: Vec<String>,
    },
    /// No filter is applied.
    /// This is intended for use with Home.
    None,
}

impl Default for FilterType {
    fn default() -> Self {
        FilterType::AppIds(Vec::new())
    }
}

impl Ord for FilterType {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (FilterType::AppIds(_), FilterType::AppIds(_)) => std::cmp::Ordering::Equal,
            (FilterType::None, FilterType::None) => std::cmp::Ordering::Equal,
            (FilterType::Categories { .. }, FilterType::Categories { .. }) => {
                std::cmp::Ordering::Equal
            }
            (FilterType::Categories { .. } | FilterType::None, FilterType::AppIds(_)) => {
                std::cmp::Ordering::Less
            }
            (FilterType::AppIds(_), FilterType::Categories { .. } | FilterType::None) => {
                std::cmp::Ordering::Greater
            }
            (FilterType::Categories { .. }, FilterType::None) => std::cmp::Ordering::Greater,
            (FilterType::None, FilterType::Categories { .. }) => std::cmp::Ordering::Less,
        }
    }
}

impl PartialOrd for FilterType {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

// Object holding the state
#[derive(Default, Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct AppGroup {
    pub name: String,
    pub icon: String,
    pub filter: FilterType,
    // pub popup: bool,
    /// If true, apps in this group also remain visible in Library Home
    /// instead of being exclusively shown in this group (favorites-style).
    #[serde(default)]
    pub keep_in_home: bool,
}

impl PartialOrd for AppGroup {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for AppGroup {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (&self.filter, &other.filter) {
            (FilterType::AppIds(_), FilterType::AppIds(_)) => {
                self.name.to_lowercase().cmp(&other.name.to_lowercase())
            }
            (FilterType::Categories { categories, .. }, FilterType::AppIds(_)) => {
                if let Some(cat_name) = categories.first() {
                    cat_name.to_lowercase().cmp(&other.name.to_lowercase())
                } else {
                    self.name.to_lowercase().cmp(&other.name.to_lowercase())
                }
            }
            (FilterType::AppIds(_), FilterType::Categories { categories, .. }) => {
                if let Some(other_name) = categories.first() {
                    self.name.to_lowercase().cmp(&other_name.to_lowercase())
                } else {
                    self.name.to_lowercase().cmp(&other.name.to_lowercase())
                }
            }
            (a, b) => a.cmp(b),
        }
    }
}

impl AppGroup {
    pub fn filtered(
        &self,
        input_value: &str,
        exceptions: &[Self],
        all_entries: &[Arc<DesktopEntryData>],
    ) -> Vec<Arc<DesktopEntryData>> {
        all_entries
            .iter()
            .filter(|de| {
                let mut keep_de = self.matches(de);
                keep_de &= if input_value.is_empty() {
                    // Groups with `keep_in_home == true` are favorites-style:
                    // their apps stay visible in Home instead of being
                    // exclusively moved into the group.
                    !exceptions.iter().any(|x| !x.keep_in_home && x.matches(de))
                } else {
                    de.name.to_lowercase().contains(&input_value.to_lowercase())
                        || de
                            .categories
                            .iter()
                            .any(|acat| acat.to_lowercase() == input_value.to_lowercase())
                };
                keep_de
            })
            .cloned()
            .collect()
    }

    fn matches(&self, entry: &DesktopEntryData) -> bool {
        match &self.filter {
            FilterType::AppIds(names) => names.iter().any(|id| id == &entry.id),
            FilterType::Categories {
                categories,
                include,
                exclude,
                ..
            } => {
                categories.iter().any(|cat| {
                    entry
                        .categories
                        .iter()
                        .any(|acat| acat.to_lowercase() == cat.to_lowercase())
                }) && exclude.iter().all(|id| id != &entry.id)
                    || include.iter().any(|id| id == &entry.id)
            }
            FilterType::None => true,
        }
    }

    pub fn name(&self) -> String {
        if &self.name == "cosmic-library-home" {
            fl!("cosmic-library-home")
        } else if &self.name == "cosmic-favorites" {
            fl!("favorites")
        } else if &self.name == "cosmic-office" {
            fl!("cosmic-office")
        } else if &self.name == "cosmic-system" {
            fl!("cosmic-system")
        } else if &self.name == "cosmic-utilities" {
            fl!("cosmic-utilities")
        } else {
            self.name.clone()
        }
    }
}

/// Which edge of the screen the app library opens from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LibraryPosition {
    /// Follow the panel/dock: bottom if a bottom dock is present and there is no
    /// top panel, otherwise top.
    Auto,
    /// Always anchor to the top of the screen.
    Top,
    /// Always anchor to the bottom of the screen.
    Bottom,
    /// Always float vertically centered on the screen.
    Center,
}

impl Default for LibraryPosition {
    fn default() -> Self {
        Self::Auto
    }
}

/// Which view the library opens on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DefaultPage {
    /// Favorites when any exist, otherwise Home (the historical behaviour).
    Auto,
    /// Always open on Home.
    Home,
    /// Always open on Favorites, even when it is empty.
    Favorites,
}

impl Default for DefaultPage {
    fn default() -> Self {
        Self::Auto
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, CosmicConfigEntry)]
pub struct AppLibraryConfig {
    pub(crate) groups: Vec<AppGroup>,
    /// Which edge of the screen the library opens from.
    #[serde(default)]
    pub position: LibraryPosition,
    /// App IDs in the built-in Favorites group.
    #[serde(default)]
    pub favorites: Vec<String>,
    /// Number of app columns in the grid (drives the window width).
    #[serde(default = "default_grid_columns")]
    pub grid_columns: u32,
    /// Number of visible app rows in the grid (drives the window height).
    #[serde(default = "default_grid_rows")]
    pub grid_rows: u32,
    /// Windows-11-style folders shown inline in Home/Favorites.
    #[serde(default)]
    pub folders: Vec<AppFolder>,
    /// Show the cosmic-settings shortcut button in the header.
    #[serde(default = "default_true")]
    pub show_settings_button: bool,
    /// Show the power-menu button in the header.
    #[serde(default = "default_true")]
    pub show_power_button: bool,
    /// Which view the library opens on.
    #[serde(default)]
    pub default_page: DefaultPage,
}

fn default_true() -> bool {
    true
}

/// Move `id` within `list` so it ends up at `insert_at`, using insert-before
/// semantics (`insert_at == list.len()` appends). If `id` isn't already in
/// `list` it is inserted at that position. Shared by the Favorites reorder and
/// the in-folder reorder, which differ only in which vec they act on.
pub fn move_within(list: &mut Vec<String>, id: &str, insert_at: usize) {
    let insert_at = if let Some(old) = list.iter().position(|x| x == id) {
        list.remove(old);
        // Removing the old entry shifts everything after it left by one, so an
        // insertion index that was past it must shift too.
        if old < insert_at {
            insert_at - 1
        } else {
            insert_at
        }
    } else {
        insert_at
    }
    .min(list.len());
    list.insert(insert_at, id.to_string());
}

fn default_grid_columns() -> u32 {
    7
}

fn default_grid_rows() -> u32 {
    3
}

impl AppLibraryConfig {
    pub fn version() -> u64 {
        1
    }

    pub fn helper() -> Option<cosmic_config::Config> {
        cosmic_config::Config::new(APP_ID, Self::version()).ok()
    }

    pub fn home() -> &'static AppGroup {
        &HOME
    }

    pub fn favorites_group() -> &'static AppGroup {
        &FAVORITES
    }

    pub fn is_favorite(&self, id: &str) -> bool {
        self.favorites.iter().any(|f| f == id)
    }

    pub fn add(&mut self, name: String, keep_in_home: bool) {
        self.groups.push(AppGroup {
            name,
            icon: "folder-symbolic".to_string(),
            filter: FilterType::AppIds(Vec::new()),
            keep_in_home,
        });
    }

    pub fn remove(&mut self, i: usize) {
        if i < self.groups.len() {
            self.groups.remove(i);
        }
    }

    pub fn set_name(&mut self, i: usize, name: String) {
        if let Some(group) = self.groups.get_mut(i) {
            group.name = name;
        }
    }

    pub fn remove_entry(&mut self, group: Option<usize>, id: &str) {
        if group == Some(FAVORITES_GROUP) {
            self.favorites.retain(|f| f != id);
            return;
        }
        let Some(group) = group.and_then(|i| self.groups.get_mut(i)) else {
            return;
        };
        match &mut group.filter {
            FilterType::AppIds(ids) => ids.retain(|conf_id| conf_id != id),
            FilterType::Categories {
                exclude, include, ..
            } => {
                include.retain(|conf_id| conf_id != id);
                exclude.retain(|conf_id| conf_id != id);
                exclude.push(id.to_string());
            }
            FilterType::None => {}
        }
    }

    pub fn add_entry(&mut self, group: Option<usize>, id: &str) {
        if group == Some(FAVORITES_GROUP) {
            if !self.is_favorite(id) {
                self.favorites.push(id.to_string());
            }
            return;
        }
        if let Some(group) = group.and_then(|i| self.groups.get_mut(i)) {
            match &mut group.filter {
                FilterType::AppIds(ids) => {
                    if ids.iter().all(|s| s != id) {
                        ids.push(id.to_string());
                    }
                }
                FilterType::Categories {
                    exclude, include, ..
                } => {
                    include.retain(|conf_id| conf_id != id);
                    exclude.retain(|conf_id| conf_id != id);
                    include.push(id.to_string());
                }
                FilterType::None => {}
            }
        } else {
            for group in &mut self.groups {
                // Favorites-style groups keep their apps even when the app is
                // (re-)added to Home directly; only exclusive groups get
                // stripped.
                if group.keep_in_home {
                    continue;
                }
                match &mut group.filter {
                    FilterType::AppIds(ids) => {
                        ids.retain(|conf_id| conf_id != id);
                    }
                    FilterType::Categories {
                        exclude, include, ..
                    } => {
                        include.retain(|conf_id| conf_id != id);
                        if exclude.iter().all(|conf_id| conf_id != id) {
                            exclude.push(id.to_string());
                        }
                    }
                    FilterType::None => {}
                }
            }
        }
    }

    /// Index of the folder currently containing `id`, if any. An app lives
    /// in at most one folder.
    pub fn folder_containing(&self, id: &str) -> Option<usize> {
        self.folders
            .iter()
            .position(|f| f.apps.iter().any(|a| a == id))
    }

    /// Remove `id` from whichever folder currently holds it (auto-dissolving
    /// that folder if it drops under 2 apps). No-op if `id` isn't in a
    /// folder. Used before inserting `id` elsewhere so it lives in ≤1 folder.
    fn take_from_any_folder(&mut self, id: &str) {
        if let Some(i) = self.folder_containing(id) {
            self.remove_from_folder(i, id);
        }
    }

    /// Create a new folder containing `target_id` and `dropped_id` (in that
    /// order), taking both out of any folder/favorites they were already in.
    /// Returns the new folder's index.
    pub fn create_folder_from_drop(
        &mut self,
        target_id: &str,
        dropped_id: &str,
        in_favorites: bool,
    ) -> usize {
        self.take_from_any_folder(target_id);
        self.take_from_any_folder(dropped_id);
        if in_favorites {
            self.favorites.retain(|f| f != target_id && f != dropped_id);
        }
        self.folders.push(AppFolder {
            name: fl!("folder"),
            apps: vec![target_id.to_string(), dropped_id.to_string()],
            in_favorites,
        });
        self.folders.len() - 1
    }

    /// Add `id` to folder `i`, first removing it from any other folder it
    /// was in. If the folder is favorites-style, `id` is also dropped from
    /// `favorites` — it's now represented by the folder tile instead.
    pub fn add_to_folder(&mut self, i: usize, id: &str) {
        let Some(folder) = self.folders.get(i) else {
            return;
        };
        if folder.apps.iter().any(|a| a == id) {
            return;
        }
        let in_favorites = folder.in_favorites;
        // Taking `id` out of its old folder can dissolve that folder, which
        // shifts every folder index after it left by one — including the
        // target's.
        let mut i = i;
        if let Some(j) = self.folder_containing(id)
            && self.remove_from_folder(j, id)
            && j < i
        {
            i -= 1;
        }
        if in_favorites {
            self.favorites.retain(|f| f != id);
        }
        if let Some(folder) = self.folders.get_mut(i) {
            folder.apps.push(id.to_string());
        }
    }

    /// Remove `id` from folder `i`. If this drops the folder under 2 apps it
    /// dissolves: the survivor (if any) returns to its host view, rejoining
    /// `favorites` if the folder was favorites-style. Returns `true` if the
    /// folder was dissolved.
    pub fn remove_from_folder(&mut self, i: usize, id: &str) -> bool {
        let Some(folder) = self.folders.get_mut(i) else {
            return false;
        };
        let in_favorites = folder.in_favorites;
        folder.apps.retain(|a| a != id);
        // A favorites-style folder represents its apps in place of favorite
        // tiles; pulling an app out must return it to `favorites` so it
        // reappears as its own tile instead of vanishing.
        if in_favorites && !self.favorites.iter().any(|f| f == id) {
            self.favorites.push(id.to_string());
        }
        if self.folders[i].apps.len() >= 2 {
            return false;
        }
        let folder = self.folders.remove(i);
        if folder.in_favorites {
            for survivor in folder.apps {
                if !self.favorites.iter().any(|f| f == &survivor) {
                    self.favorites.push(survivor);
                }
            }
        }
        true
    }

    /// Move `id` within folder `i`'s app list so it ends up at `insert_at`
    /// (insert-before semantics; `insert_at == len` appends). No-op if the
    /// folder doesn't hold `id` — an app is only reorderable inside the folder
    /// it already lives in.
    pub fn reorder_folder_app(&mut self, i: usize, id: &str, insert_at: usize) {
        let Some(folder) = self.folders.get_mut(i) else {
            return;
        };
        if !folder.apps.iter().any(|a| a == id) {
            return;
        }
        move_within(&mut folder.apps, id, insert_at);
    }

    /// Move folder `from` so it sits at index `to` in `folders`, shifting the
    /// folders in between. Both indices are into the global `folders` vec (the
    /// per-view tile order is just this vec filtered by `in_favorites`, so a
    /// plain move preserves the relative order of the other view's folders).
    pub fn reorder_folder(&mut self, from: usize, to: usize) {
        if from >= self.folders.len() || from == to {
            return;
        }
        let folder = self.folders.remove(from);
        // Removing `from` shifts everything after it left by one.
        let to = to.min(self.folders.len());
        self.folders.insert(to, folder);
    }

    /// Rename folder `i`.
    pub fn rename_folder(&mut self, i: usize, name: String) {
        if let Some(folder) = self.folders.get_mut(i) {
            folder.name = name;
        }
    }

    /// Dissolve folder `i` outright, returning all its apps to the host view
    /// (rejoining `favorites` if it was favorites-style).
    pub fn ungroup_folder(&mut self, i: usize) {
        if i >= self.folders.len() {
            return;
        }
        let folder = self.folders.remove(i);
        if folder.in_favorites {
            for id in folder.apps {
                if !self.favorites.iter().any(|f| f == &id) {
                    self.favorites.push(id);
                }
            }
        }
    }

    pub fn filtered(
        &self,
        group: Option<usize>,
        input_value: &str,
        entries: &[Arc<DesktopEntryData>],
    ) -> Vec<Arc<DesktopEntryData>> {
        match group {
            None => {
                let mut result = HOME.filtered(input_value, &self.groups, entries);
                if input_value.is_empty() {
                    // Apps inside a Home folder (`in_favorites == false`) are
                    // shown via the folder tile instead of individually.
                    // Favorites-folder apps are left alone here — favorites
                    // never hide from Home.
                    result.retain(|de| {
                        !self
                            .folders
                            .iter()
                            .any(|f| !f.in_favorites && f.apps.iter().any(|a| a == &de.id))
                    });
                }
                result
            }
            Some(FAVORITES_GROUP) => {
                if input_value.is_empty() {
                    // Order by position in `self.favorites`, not global entry order.
                    self.favorites
                        .iter()
                        .filter_map(|fav_id| entries.iter().find(|e| &e.id == fav_id))
                        .cloned()
                        .collect()
                } else {
                    // Searching in Favorites searches all apps, like Home.
                    HOME.filtered(input_value, &self.groups, entries)
                }
            }
            Some(i) => self
                .groups
                .get(i)
                .map(|g| g.filtered(input_value, &Vec::new(), entries))
                .unwrap_or_default(),
        }
    }
}

impl Default for AppLibraryConfig {
    fn default() -> Self {
        AppLibraryConfig {
            groups: vec![
                AppGroup {
                    name: "cosmic-office".to_string(),
                    icon: "folder-symbolic".to_string(),
                    filter: FilterType::Categories {
                        categories: vec!["Office".to_string()],
                        include: vec![
                            "org.gnome.Totem".to_string(),
                            "org.gnome.eog".to_string(),
                            "simple-scan".to_string(),
                            "thunderbird".to_string(),
                        ],
                        exclude: Vec::new(),
                    },
                    keep_in_home: false,
                },
                AppGroup {
                    name: "cosmic-system".to_string(),
                    icon: "folder-symbolic".to_string(),
                    filter: FilterType::Categories {
                        categories: vec!["System".to_string()],
                        include: vec![
                            "gnome-language-selector".to_string(),
                            "im-config".to_string(),
                            "org.freedesktop.IBus.Setup".to_string(),
                            "system76-driver".to_string(),
                        ],
                        exclude: vec![
                            "com.system76.CosmicStore".to_string(),
                            "com.system76.CosmicTerm".to_string(),
                        ],
                    },
                    keep_in_home: false,
                },
                AppGroup {
                    name: "cosmic-utilities".to_string(),
                    icon: "folder-symbolic".to_string(),
                    filter: FilterType::Categories {
                        categories: vec!["Utility".to_string()],
                        include: vec!["nm-connection-editor".to_string()],
                        exclude: vec![
                            "com.system76.CosmicEdit".to_string(),
                            "com.system76.CosmicFiles".to_string(),
                        ],
                    },
                    keep_in_home: false,
                },
            ],
            position: LibraryPosition::default(),
            favorites: Vec::new(),
            grid_columns: default_grid_columns(),
            grid_rows: default_grid_rows(),
            folders: Vec::new(),
            show_settings_button: true,
            show_power_button: true,
            default_page: DefaultPage::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folders(spec: &[(&str, bool)]) -> AppLibraryConfig {
        AppLibraryConfig {
            folders: spec
                .iter()
                .map(|(name, in_favorites)| AppFolder {
                    name: name.to_string(),
                    apps: vec!["a".into(), "b".into()],
                    in_favorites: *in_favorites,
                })
                .collect(),
            ..AppLibraryConfig::default()
        }
    }

    fn names(config: &AppLibraryConfig) -> Vec<&str> {
        config.folders.iter().map(|f| f.name.as_str()).collect()
    }

    #[test]
    fn reorder_folder_moves_left() {
        let mut c = folders(&[("A", false), ("B", false), ("C", false)]);
        c.reorder_folder(2, 0);
        assert_eq!(names(&c), ["C", "A", "B"]);
    }

    #[test]
    fn reorder_folder_moves_right() {
        let mut c = folders(&[("A", false), ("B", false), ("C", false)]);
        // Dragging A onto C lands it after C: removing A shifts C left to 1,
        // so inserting at 2 puts A last.
        c.reorder_folder(0, 2);
        assert_eq!(names(&c), ["B", "C", "A"]);
    }

    #[test]
    fn reorder_folder_ignores_noop_and_out_of_range() {
        let mut c = folders(&[("A", false), ("B", false)]);
        c.reorder_folder(1, 1);
        assert_eq!(names(&c), ["A", "B"]);
        c.reorder_folder(9, 0);
        assert_eq!(names(&c), ["A", "B"]);
    }

    #[test]
    fn reorder_folder_preserves_the_other_views_order() {
        // Home tiles are A and C; Favorites tiles are B and D. Reordering the
        // Home pair must not disturb the Favorites pair's relative order.
        let mut c = folders(&[("A", false), ("B", true), ("C", false), ("D", true)]);
        c.reorder_folder(2, 0);
        assert_eq!(names(&c), ["C", "A", "B", "D"]);
        let home: Vec<_> = c
            .folders
            .iter()
            .filter(|f| !f.in_favorites)
            .map(|f| f.name.as_str())
            .collect();
        let fav: Vec<_> = c
            .folders
            .iter()
            .filter(|f| f.in_favorites)
            .map(|f| f.name.as_str())
            .collect();
        assert_eq!(home, ["C", "A"]);
        assert_eq!(fav, ["B", "D"]);
    }

    fn list(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn move_within_moves_backwards() {
        let mut l = list(&["a", "b", "c", "d"]);
        move_within(&mut l, "d", 1);
        assert_eq!(l, list(&["a", "d", "b", "c"]));
    }

    #[test]
    fn move_within_moves_forwards() {
        // Insert-before semantics against the *original* indices: asking for
        // index 3 while "a" sits at 0 lands it after "c", not after "d".
        let mut l = list(&["a", "b", "c", "d"]);
        move_within(&mut l, "a", 3);
        assert_eq!(l, list(&["b", "c", "a", "d"]));
    }

    #[test]
    fn move_within_appends_at_len() {
        let mut l = list(&["a", "b", "c"]);
        move_within(&mut l, "a", 3);
        assert_eq!(l, list(&["b", "c", "a"]));
    }

    #[test]
    fn move_within_is_a_noop_for_the_same_slot() {
        let mut l = list(&["a", "b", "c"]);
        move_within(&mut l, "b", 1);
        assert_eq!(l, list(&["a", "b", "c"]));
    }

    #[test]
    fn move_within_inserts_an_absent_id() {
        let mut l = list(&["a", "b"]);
        move_within(&mut l, "z", 1);
        assert_eq!(l, list(&["a", "z", "b"]));
    }

    #[test]
    fn move_within_clamps_an_out_of_range_index() {
        let mut l = list(&["a", "b"]);
        move_within(&mut l, "a", 99);
        assert_eq!(l, list(&["b", "a"]));
    }

    #[test]
    fn reorder_folder_app_reorders_within_the_folder() {
        let mut c = folders(&[("A", false)]);
        c.folders[0].apps = list(&["x", "y", "z"]);
        c.reorder_folder_app(0, "z", 0);
        assert_eq!(c.folders[0].apps, list(&["z", "x", "y"]));
    }

    #[test]
    fn reorder_folder_app_ignores_apps_not_in_the_folder() {
        // Dropping a stray id into a folder's strips must not smuggle a new
        // app in — only `add_to_folder` may do that.
        let mut c = folders(&[("A", false)]);
        c.folders[0].apps = list(&["x", "y"]);
        c.reorder_folder_app(0, "outsider", 0);
        assert_eq!(c.folders[0].apps, list(&["x", "y"]));
        c.reorder_folder_app(9, "x", 0);
        assert_eq!(c.folders[0].apps, list(&["x", "y"]));
    }

    #[test]
    fn remove_from_favorites_folder_returns_app_to_favorites() {
        // A favorites folder with three apps: pulling one out keeps the folder
        // (two apps remain) and the removed app rejoins `favorites` as a tile.
        let mut c = folders(&[("A", true)]);
        c.folders[0].apps = list(&["a", "b", "c"]);
        let dissolved = c.remove_from_folder(0, "b");
        assert!(!dissolved);
        assert_eq!(c.folders[0].apps, list(&["a", "c"]));
        assert_eq!(c.favorites, list(&["b"]));
    }

    #[test]
    fn remove_from_home_folder_does_not_touch_favorites() {
        // Home folders don't back favorite tiles, so removal must not leak the
        // app into `favorites`.
        let mut c = folders(&[("A", false)]);
        c.folders[0].apps = list(&["a", "b", "c"]);
        c.remove_from_folder(0, "b");
        assert!(c.favorites.is_empty());
    }

    #[test]
    fn dissolving_favorites_folder_returns_both_apps() {
        // Removing an app from a two-app favorites folder dissolves it; both
        // the removed app and the survivor land back in `favorites`.
        let mut c = folders(&[("A", true)]);
        let dissolved = c.remove_from_folder(0, "a");
        assert!(dissolved);
        assert!(c.folders.is_empty());
        assert_eq!(c.favorites, list(&["a", "b"]));
    }

    #[test]
    fn default_page_defaults_to_auto() {
        assert_eq!(AppLibraryConfig::default().default_page, DefaultPage::Auto);
    }
}
