use std::collections::HashMap;
use std::fmt::{Debug, Display};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::str::FromStr;
use std::sync::{Arc, LazyLock};
use std::time::{Duration, Instant};

use clap::Parser;
use cosmic::iced::event::wayland::OutputEvent;
use cosmic::iced::platform_specific::shell::commands::layer_surface::set_padding;
use cosmic::iced::runtime::platform_specific::wayland::CornerRadius;
use cosmic::iced::runtime::platform_specific::wayland::layer_surface::IcedMargin;
use cosmic::iced::runtime::{Action, platform_specific, task};
use cosmic::iced::window;
use cosmic::surface::action::{LiveSettings, app_layer_shell, simple_layer_shell, simple_popup};
use cosmic::widget::menu::menu_column::MenuColumn;
use cosmic::widget::reorderable_flex_row;
use cosmic::widget::space::horizontal;
use cosmic::{
    Element,
    app::{Core, CosmicFlags, Settings, Task},
    cctk::sctk::{
        self,
        shell::wlr_layer::{Anchor, KeyboardInteractivity},
    },
    cosmic_config::{Config, CosmicConfigEntry},
    cosmic_theme::Spacing,
    dbus_activation,
    desktop::{DesktopEntryData, IconSourceExt, fde::PathSource, load_desktop_file},
    iced::{
        self, Alignment, Color, Length, Limits, Size, Subscription,
        event::{listen_with, wayland::OverlapNotifyEvent},
        executor,
        id::Id,
        /*wayland::actions::{
            data_device::ActionInner,
        },*/
        widget::{
            column, container, mouse_area, row, rule::horizontal as horizontal_rule,
            scrollable::RelativeOffset,
        },
        window::Event as WindowEvent,
    },
    iced::{
        core::{
            Border, Padding, Rectangle, Shadow,
            alignment::Vertical,
            event::{
                PlatformSpecific,
                wayland::{self, LayerEvent},
            },
            keyboard::{Key, key::Named},
            widget::operation::{
                self,
                focusable::{find_focused, focus},
            },
            widget::tree,
            window::Id as SurfaceId,
        },
        platform_specific::shell::wayland::commands::{
            self,
            activation::request_token,
            layer_surface::{destroy_layer_surface, get_layer_surface},
            overlap_notify::overlap_notify,
            popup::destroy_popup,
        },
        runtime::{
            self as iced_runtime,
            dnd::end_dnd,
            platform_specific::wayland::{
                layer_surface::SctkLayerSurfaceSettings,
                popup::{SctkPopupSettings, SctkPositioner},
            },
        },
        widget::stack,
    },
    keyboard_nav,
    theme::{self, Button, TextInput},
    widget::{
        self,
        autosize::autosize,
        button, divider,
        dnd_destination::dnd_destination_for_data,
        dnd_source,
        icon::{self, from_name},
        popover::{self, popover},
        scrollable, search_input, space, svg, text, text_input, tooltip,
    },
};
use cosmic_app_list_config::AppListConfig;
use itertools::Itertools;
use log::error;
use sctk::shell::wlr_layer;
use serde::{Deserialize, Serialize};
use switcheroo_control::Gpu;

use crate::app_group::{
    AppFolder, AppGroup, AppLibraryConfig, DefaultPage, FAVORITES_GROUP, LibraryPosition,
};
use crate::fl;
use crate::power::PowerAction;
use crate::subscriptions::desktop_files::desktop_files;
use crate::widgets::application::{AppletString, ApplicationButton};

// popovers should show options, but also the desktop info options
// should be a way to add apps to groups
// should be a way to remove apps from groups

static SEARCH_ID: LazyLock<Id> = LazyLock::new(|| Id::new("search"));
static EDIT_GROUP_ID: LazyLock<Id> = LazyLock::new(|| Id::new("edit_group"));
static NEW_GROUP_ID: LazyLock<Id> = LazyLock::new(|| Id::new("new_group"));
static SUBMIT_DELETE_ID: LazyLock<Id> = LazyLock::new(|| Id::new("cancel_delete"));

static CREATE_NEW: LazyLock<String> = LazyLock::new(|| fl!("create-new"));
static ADD_GROUP: LazyLock<String> = LazyLock::new(|| fl!("add-group"));
static SEARCH_PLACEHOLDER: LazyLock<String> = LazyLock::new(|| fl!("search-placeholder"));
static NEW_GROUP_PLACEHOLDER: LazyLock<String> = LazyLock::new(|| fl!("new-group-placeholder"));
static SAVE: LazyLock<String> = LazyLock::new(|| fl!("save"));
static CANCEL: LazyLock<String> = LazyLock::new(|| fl!("cancel"));
static RUN: LazyLock<String> = LazyLock::new(|| fl!("run"));
static REMOVE: LazyLock<String> = LazyLock::new(|| fl!("remove"));
static FLATPAK: LazyLock<String> = LazyLock::new(|| fl!("flatpak"));
static LOCAL: LazyLock<String> = LazyLock::new(|| fl!("local"));
static NIX: LazyLock<String> = LazyLock::new(|| fl!("nix"));
static SNAP: LazyLock<String> = LazyLock::new(|| fl!("snap"));
static SYSTEM: LazyLock<String> = LazyLock::new(|| fl!("system"));

/// Labels for the library-settings position dropdown, indexed the same way
/// as `Message::SetPosition` (0 = Auto, 1 = Top, 2 = Bottom, 3 = Center).
static POSITION_LABELS: LazyLock<Vec<String>> = LazyLock::new(|| {
    vec![
        fl!("position-auto"),
        fl!("position-top"),
        fl!("position-bottom"),
        fl!("position-center"),
    ]
});

/// Labels for the library-settings default-page dropdown, indexed the same way
/// as `Message::SetDefaultPage` (0 = Auto, 1 = Home, 2 = Favorites).
static DEFAULT_PAGE_LABELS: LazyLock<Vec<String>> = LazyLock::new(|| {
    vec![
        fl!("default-page-auto"),
        fl!("cosmic-library-home"),
        fl!("favorites"),
    ]
});

static NEW_GROUP_WINDOW_ID: LazyLock<SurfaceId> = LazyLock::new(SurfaceId::unique);
static NEW_GROUP_AUTOSIZE_ID: LazyLock<cosmic::widget::Id> =
    LazyLock::new(cosmic::widget::Id::unique);
static DELETE_GROUP_WINDOW_ID: LazyLock<SurfaceId> = LazyLock::new(SurfaceId::unique);
static DELETE_GROUP_AUTOSIZE_ID: LazyLock<cosmic::widget::Id> =
    LazyLock::new(cosmic::widget::Id::unique);
pub(crate) static MENU_ID: LazyLock<SurfaceId> = LazyLock::new(SurfaceId::unique);
pub(crate) static MENU_AUTOSIZE_ID: LazyLock<cosmic::widget::Id> =
    LazyLock::new(cosmic::widget::Id::unique);

/// Base drag id for per-tile favorites-reorder dnd destinations, offset well
/// clear of the group row's drag ids (`0..=groups.len()+1`) so the two sets
/// never collide.
const FAVORITE_TILE_DRAG_ID_BASE: u64 = 1_000_000;
/// Base drag id for Home app tiles acting as folder-creation drop targets
/// (dropping app A onto app B's Home tile creates a folder holding both).
const HOME_TILE_DRAG_ID_BASE: u64 = 2_000_000;
/// Base drag id for folder tiles acting as drop targets (dropping an app on
/// a folder tile adds it to that folder).
const FOLDER_TILE_DRAG_ID_BASE: u64 = 4_000_000;
/// Base drag id for the narrow reorder strips flanking each favorites tile
/// (left strip = `base + 2i`, right strip = `base + 2i + 1`). Dropping on a
/// strip inserts the dragged app before/after the tile; dropping on the tile
/// itself combines both into a favorites folder. Three SIBLING destinations
/// per cell — never nested, so none of the nested-destination fragility that
/// blocked this in Phase 11.
const FAV_STRIP_DRAG_ID_BASE: u64 = 3_000_000;

#[derive(Parser, Debug, Serialize, Deserialize, Clone)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Args {
    #[clap(subcommand)]
    pub subcommand: Option<ApplicationsTasks>,
}

impl CosmicFlags for Args {
    type SubCommand = ApplicationsTasks;
    type Args = Vec<String>;

    fn action(&self) -> Option<&ApplicationsTasks> {
        self.subcommand.as_ref()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, clap::Subcommand)]
pub enum ApplicationsTasks {
    #[clap(about = "Start app-library with an input")]
    Input { input: Option<String> },
    #[clap(about = "Close app-library if open")]
    Close,
    #[clap(about = "Run a standalone instance (not single-instance)")]
    Run,
}

impl Display for ApplicationsTasks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", serde_json::ser::to_string(self).unwrap())
    }
}

impl FromStr for ApplicationsTasks {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::de::from_str(s)
    }
}

pub fn run() -> cosmic::iced::Result {
    let args = Args::parse();
    let settings = Settings::default()
        .antialiasing(true)
        .client_decorations(true)
        .debug(false)
        .default_text_size(16.0)
        .scale_factor(1.0)
        .no_main_window(true)
        .exit_on_close(false);

    // Use standalone run if requested, otherwise use single-instance
    if matches!(args.subcommand, Some(ApplicationsTasks::Run)) {
        cosmic::app::run::<CosmicAppLibrary>(settings, args)
    } else {
        cosmic::app::run_single_instance::<CosmicAppLibrary>(settings, args)
    }
}

pub struct AppSource(PathSource);

impl AppSource {
    pub fn as_icon(&self) -> Option<widget::icon::Handle> {
        let name = match &self.0 {
            PathSource::Local | PathSource::LocalDesktop => "app-source-local-symbolic",
            PathSource::System | PathSource::SystemLocal => "app-source-system-symbolic",
            PathSource::LocalFlatpak | PathSource::SystemFlatpak => "app-source-flatpak",
            PathSource::SystemSnap => "app-source-snap",
            PathSource::Nix | PathSource::LocalNix => "app-source-nix",
            PathSource::Other(_) => return None,
        };
        let handle = crate::icon_cache::icon_cache_handle(name, 16);
        Some(handle)
    }
}

impl<'a> From<&'a Path> for AppSource {
    fn from(path: &'a Path) -> Self {
        AppSource(PathSource::guess_from(path))
    }
}

impl Display for AppSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:.7}",
            match &self.0 {
                PathSource::Local | PathSource::LocalDesktop => LOCAL.as_str(),
                PathSource::SystemFlatpak | PathSource::LocalFlatpak => FLATPAK.as_str(),
                PathSource::SystemSnap => SNAP.as_str(),
                PathSource::Nix | PathSource::LocalNix => NIX.as_str(),
                PathSource::System | PathSource::SystemLocal => SYSTEM.as_str(),
                PathSource::Other(s) => s.as_str(),
            }
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SurfaceState {
    Visible,
    Hidden,
    WaitingToBeShown,
}

struct CosmicAppLibrary {
    search_value: String,
    entry_path_input: Vec<Arc<DesktopEntryData>>,
    all_entries: Vec<Arc<DesktopEntryData>>,
    menu: Option<usize>,
    power_menu_open: bool,
    /// Global `config.folders` index of the folder tile currently being
    /// dragged, if any. Drop destinations read this at view-build time to tell
    /// a folder drag (reorder) apart from an app drag (add-to-folder), since
    /// both travel over the wire as the same `AppletString` mime type.
    dragging_folder: Option<usize>,
    helper: Option<Config>,
    config: AppLibraryConfig,
    cur_group: Option<usize>,
    locale: Option<String>,
    edit_name: Option<String>,
    new_group: Option<String>,
    new_group_keep_in_home: bool,
    dnd_icon: Option<usize>,
    offer_group: Option<Option<usize>>,
    waiting_for_filtered: bool,
    scroll_offset: f32,
    core: Core,
    group_to_delete: Option<usize>,
    gpus: Option<Vec<Gpu>>,
    last_hide: Option<Instant>,
    duplicates: HashMap<PathBuf, (AppSource, Option<widget::icon::Handle>)>,
    app_list_config: AppListConfig,
    overlap: HashMap<String, Rectangle>,
    margin: f32,
    bottom_margin: f32,
    size: iced::Size,
    needs_clear: bool,
    focused_id: Option<widget::Id>,
    entry_ids: Vec<widget::Id>,
    entry_icon_handles: Vec<widget::icon::Handle>,
    scrollable_id: widget::Id,
    surface_state: SurfaceState,
    hand_over: String,
    group_keys: Vec<u64>,
    next_group_key: u64,
    dummy_id: Option<window::Id>,
    /// Index into `config.folders` of the folder currently shown in-place
    /// (Windows-11-style folder view), or `None` when showing the normal
    /// Home/Favorites/group grid.
    open_folder: Option<usize>,
    /// Editable buffer for the folder-view rename text_input, seeded from
    /// the folder's name on open and persisted on submit/close.
    folder_name_buffer: String,
    /// Favorites drop zone a drag currently hovers `(tile index, zone)`;
    /// renders the insertion bar / combine highlight while dragging.
    fav_drop_hint: Option<(usize, FavDropZone)>,
    /// In-place library-settings page open (replaces the grid area).
    settings_view: bool,
}

impl Default for CosmicAppLibrary {
    fn default() -> Self {
        Self {
            search_value: Default::default(),
            entry_path_input: Default::default(),
            all_entries: Default::default(),
            menu: Default::default(),
            power_menu_open: false,
            dragging_folder: None,
            helper: Default::default(),
            config: Default::default(),
            cur_group: Default::default(),
            locale: Default::default(),
            edit_name: Default::default(),
            new_group: Default::default(),
            new_group_keep_in_home: Default::default(),
            dnd_icon: Default::default(),
            offer_group: Default::default(),
            waiting_for_filtered: Default::default(),
            scroll_offset: Default::default(),
            core: Default::default(),
            group_to_delete: Default::default(),
            gpus: Default::default(),
            last_hide: Default::default(),
            duplicates: Default::default(),
            app_list_config: Default::default(),
            overlap: Default::default(),
            margin: Default::default(),
            bottom_margin: Default::default(),
            size: Size::ZERO,
            needs_clear: Default::default(),
            focused_id: Default::default(),
            entry_ids: Default::default(),
            entry_icon_handles: Default::default(),
            scrollable_id: widget::Id::unique(),
            surface_state: SurfaceState::Hidden,
            hand_over: String::default(),
            group_keys: Default::default(),
            next_group_key: Default::default(),
            dummy_id: None,
            open_folder: Default::default(),
            folder_name_buffer: Default::default(),
            fav_drop_hint: Default::default(),
            settings_view: false,
        }
    }
}

async fn try_get_gpus() -> Option<Vec<Gpu>> {
    let connection = zbus::Connection::system().await.ok()?;
    let proxy = switcheroo_control::SwitcherooControlProxy::new(&connection)
        .await
        .ok()?;

    if !proxy.has_dual_gpu().await.ok()? {
        return None;
    }

    let gpus = proxy.get_gpus().await.ok()?;
    if gpus.is_empty() {
        return None;
    }
    Some(gpus)
}

impl CosmicAppLibrary {
    fn create_dummy_layer_surface(&mut self) -> Task<Message> {
        self.needs_clear = true;
        let id = window::Id::unique();
        self.dummy_id = Some(id);
        cosmic::surface::surface_task(simple_layer_shell::<Message>(
            || LiveSettings {
                padding: Some(IcedMargin::default()),
                corners: Some(CornerRadius::default()),
                blur: Some(false),
            },
            move || {
                SctkLayerSurfaceSettings {
                    id,
                    layer: wlr_layer::Layer::Bottom,
                    keyboard_interactivity: wlr_layer::KeyboardInteractivity::None,
                    input_zone: Some(Vec::new()),
                    // Anchor to every edge so the sensor spans the whole output.
                    // A top-only 1200x200 surface could never overlap a bottom
                    // panel/dock, so bottom-edge detection always saw nothing.
                    anchor: wlr_layer::Anchor::all(),
                    output:
                        cosmic::iced::runtime::platform_specific::wayland::layer_surface::IcedOutput::Active,
                    namespace: "cosmic_launcher_dummy".into(),
                    margin: IcedMargin::default(),
                    size: Some((None, None)),
                    exclusive_zone: -1,
                    size_limits: Limits::NONE,
                }
            },
            None::<fn() -> Element<'static, cosmic::Action<Message>>>,
        ))
    }

    pub fn activate(&mut self) -> Task<Message> {
        if matches!(self.surface_state, SurfaceState::Visible) {
            return self.hide();
        } else if matches!(self.surface_state, SurfaceState::Hidden)
            && self
                .last_hide
                .is_none_or(|i| i.elapsed() >= Duration::from_millis(100))
        {
            self.surface_state = SurfaceState::WaitingToBeShown;
            self.edit_name = None;
            self.search_value = "".to_string();
            self.scroll_offset = 0.0;
            self.cur_group = match self.config.default_page {
                DefaultPage::Home => None,
                DefaultPage::Favorites => Some(FAVORITES_GROUP),
                // Favorites when any exist, otherwise Home.
                DefaultPage::Auto => (!self.config.favorites.is_empty()).then_some(FAVORITES_GROUP),
            };
            self.load_apps();
            self.needs_clear = true;
            let fetch_gpus = Task::perform(try_get_gpus(), |gpus| {
                cosmic::Action::App(Message::GpuUpdate(gpus))
            });
            return Task::batch(vec![
                cosmic::surface::surface_task(app_layer_shell(
                    |app: &CosmicAppLibrary| LiveSettings {
                        padding: Some(app.layer_padding()),
                        corners: None,
                        blur: None,
                    },
                    move |_: &mut CosmicAppLibrary| SctkLayerSurfaceSettings {
                        id: SurfaceId::RESERVED,
                        keyboard_interactivity: KeyboardInteractivity::Exclusive,
                        anchor: Anchor::all(),
                        namespace: "app-library".into(),
                        size: Some((None, None)),
                        exclusive_zone: -1,
                        ..Default::default()
                    },
                    None,
                )),
                fetch_gpus,
            ]);
        }
        Task::none()
    }

    fn handle_overlap(&mut self) -> Task<Message> {
        let mid_height = self.size.height / 2.;
        self.margin = 0.;
        self.bottom_margin = 0.;

        for o in self.overlap.values() {
            if self.margin + mid_height < o.y
                || self.margin > o.y + o.height
                || mid_height < o.y + o.height
            {
                continue;
            }

            self.margin = o.y + o.height;
        }

        // Detect a bottom-anchored panel/dock: an overlap whose top edge sits in
        // the lower half of the screen. `bottom_margin` is the distance from the
        // screen bottom to the panel's top edge, plus the panel's own edge gap so
        // the library floats above the panel with matching spacing (not flush).
        for o in self.overlap.values() {
            if o.y >= mid_height {
                let edge_gap = (self.size.height - (o.y + o.height)).max(0.);
                self.bottom_margin = self.bottom_margin.max(self.size.height - o.y + edge_gap);
            }
        }

        let mut cmds = Vec::with_capacity(2);
        // set the padding
        let margin = self.layer_padding();
        cmds.push(set_padding::<()>(SurfaceId::RESERVED, margin).discard());
        cmds.push(
            if self.core.system_theme().cosmic().frosted_system_interface {
                // Full-surface blur (frosted everywhere, upstream #387 bleed and
                // all). A window-sized rect here does NOT work: this action's
                // surface lookup misses the layer surface (the surface subsystem
                // assigns its own runtime id, so the request parks in
                // pending_blur and never reaches the compositor) — the frost
                // actually comes from libcosmic's automatic EnableBlur path,
                // which always uses a MAX rect. Tightening the region needs a
                // different mechanism; see PLAN.md Phase 9.
                task::effect(Action::PlatformSpecific(
                    platform_specific::Action::Wayland(
                        cosmic::iced::runtime::platform_specific::wayland::Action::BlurSurface(
                            SurfaceId::RESERVED,
                            Some(vec![Rectangle {
                                x: 0.,
                                y: 0.,
                                width: f32::MAX,
                                height: f32::MAX,
                            }]),
                        ),
                    ),
                ))
            } else {
                task::effect(Action::PlatformSpecific(
                    platform_specific::Action::Wayland(
                        cosmic::iced::runtime::platform_specific::wayland::Action::BlurSurface(
                            SurfaceId::RESERVED,
                            None,
                        ),
                    ),
                ))
            },
        );
        Task::batch(cmds)
    }

    /// Resolve the configured position into a concrete edge (Top or Bottom),
    /// honoring `Auto` by following the detected panel/dock layout.
    fn effective_position(&self) -> LibraryPosition {
        match self.config.position {
            LibraryPosition::Top => LibraryPosition::Top,
            LibraryPosition::Bottom => LibraryPosition::Bottom,
            LibraryPosition::Center => LibraryPosition::Center,
            // Auto: open from the bottom only when there is a bottom dock and no
            // top panel, so the default top-panel layout is unchanged.
            LibraryPosition::Auto => {
                if self.bottom_margin > 0. && self.margin == 0. {
                    LibraryPosition::Bottom
                } else {
                    LibraryPosition::Top
                }
            }
        }
    }

    /// Configured window width, clamped to a sane minimum and to the screen size.
    fn window_width(&self) -> f32 {
        let cols = self.config.grid_columns.clamp(4, 12);
        (cols as f32 * 160.0 + 80.0).clamp(600.0, self.size.width.max(600.0))
    }

    /// Configured window height, clamped to a sane minimum and to the screen size.
    fn window_height(&self) -> f32 {
        let rows = self.config.grid_rows.clamp(2, 8);
        (rows as f32 * 148.0 + 246.0).clamp(400.0, self.size.height.max(400.0))
    }

    /// Number of app columns in the grid, clamped to a sane range.
    fn grid_columns(&self) -> usize {
        self.config.grid_columns.clamp(4, 12) as usize
    }

    /// Max height of the app grid scrollable, derived from configured rows.
    fn grid_max_height(&self) -> f32 {
        self.config.grid_rows.clamp(2, 8) as f32 * 148.0
    }

    #[allow(clippy::cast_possible_truncation)]
    fn layer_padding(&self) -> IcedMargin {
        let width = self.window_width();
        let height = self.window_height();
        let horizontal = ((self.size.width - width) / 2.).max(0.) as i32;
        let (top, bottom) = match self.effective_position() {
            LibraryPosition::Bottom => (
                (self.size.height - height - 16. - self.bottom_margin).max(0.) as i32,
                self.bottom_margin as i32 + 16,
            ),
            LibraryPosition::Center => {
                let symmetric = ((self.size.height - height) / 2.).max(0.) as i32;
                (symmetric, symmetric)
            }
            // Top (and any resolved non-bottom).
            _ => (
                self.margin as i32 + 16,
                (self.size.height - height - 16. - self.margin).max(0.) as i32,
            ),
        };
        IcedMargin {
            top,
            left: horizontal,
            right: horizontal,
            bottom,
        }
    }

    /// Update entry IDs and their icon handles.
    fn update_entry_metadata(&mut self) {
        self.entry_ids = (0..self.entry_path_input.len())
            .map(|i| widget::Id::from(format!("app-entry-{i}")))
            .collect();

        self.entry_icon_handles = self
            .entry_path_input
            .iter()
            .map(|e| e.icon.as_cosmic_icon())
            .collect();
    }
}

/// Which part of a favorites tile a drag is currently hovering: the narrow
/// strip before/after the icon (insert-reorder) or the icon area itself
/// (combine into a favorites folder). Drives the drop-intent hover hint.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FavDropZone {
    Before,
    Onto,
    After,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum GroupRowKey {
    Home,
    Favorites,
    Custom(u64),
    NewGroup,
}

#[derive(Clone, Debug)]
enum Message {
    Activate,
    UpdateFocused(Option<widget::Id>),
    InputChanged(String),
    KeyboardNav(keyboard_nav::Action),
    PrevRow,
    NextRow,
    Layer(LayerEvent, SurfaceId),
    Hide,
    /// Raw ESC key release: closes the folder view if one is open, otherwise
    /// hides the library. Kept distinct from `Hide` because the other `Hide`
    /// call sites (focus loss, power actions, ...) should still hide the
    /// whole library even while a folder is open.
    EscapePressed,
    ActivateApp(usize, Option<usize>),
    StartCurAppFocus,
    ActivationToken(Option<String>, String, String, Option<usize>, bool),
    SelectGroup(Option<usize>),
    ReorderGroup(Vec<GroupRowKey>),
    Delete(usize),
    ConfirmDelete,
    CancelDelete,
    StartEditName(String),
    EditName(String),
    SubmitName,
    StartNewGroup,
    NewGroup(String),
    NewGroupKeepInHome(bool),
    SubmitNewGroup,
    CancelNewGroup,
    LoadApps,
    FilterApps(String, Vec<Arc<DesktopEntryData>>),
    OpenContextMenu(Rectangle, usize),
    CloseContextMenu,
    /// Close the header power popover. Separate from `CloseContextMenu` and
    /// fired on mouse RELEASE: closing on press destroys the popover before
    /// its buttons (which emit on release) can deliver their action.
    ClosePowerMenu,
    OpenSettings,
    ToggleFavorite(usize),
    TogglePowerMenu,
    Power(PowerAction),
    PowerResult(Option<String>),
    SelectAction(MenuAction),
    StartDrag(usize),
    FinishDrag(bool),
    CancelDrag,
    FavDragEnter(usize, FavDropZone),
    FavDragLeave(usize, FavDropZone),
    /// A folder tile drag started/ended — sets/clears `dragging_folder` so
    /// drop destinations can distinguish it from an app drag.
    StartFolderDrag(usize),
    EndFolderDrag,
    /// Move the folder at `config.folders[from]` to index `to`.
    ReorderFolder {
        from: usize,
        to: usize,
    },
    /// A drop that isn't meaningful for the thing being dragged (e.g. a
    /// folder released on an app tile). Deliberately does nothing.
    IgnoredDrop,
    StartDndOffer(Option<usize>),
    FinishDndOffer(Option<usize>, Option<DesktopEntryData>),
    LeaveDndOffer(Option<usize>),
    /// Move (or insert) the favorite app with this id so it sits at `index`
    /// in `config.favorites` (insert-before semantics; `index == len` appends).
    ReorderFavorite(String, usize),
    /// Move the app with this id to `index` within the currently open folder's
    /// app list (same insert-before semantics as `ReorderFavorite`).
    ReorderFolderApp(String, usize),
    /// Move (or insert) the app with this id so it sits at `index` in
    /// `config.home_order` (same insert-before semantics as `ReorderFavorite`).
    ReorderHome(String, usize),
    ScrollYOffset(f32),
    GpuUpdate(Option<Vec<Gpu>>),
    PinToAppTray(usize),
    UnPinFromAppTray(usize),
    AppListConfig(AppListConfig),
    Opened(Size, SurfaceId),
    Overlap(OverlapNotifyEvent),
    Output(OutputEvent),
    /// Open the in-place folder view for `config.folders[i]`.
    OpenFolder(usize),
    /// Leave the folder view, persisting a non-empty changed name.
    CloseFolder,
    FolderNameInput(String),
    /// Drop of `dropped_id` onto `target_id`'s tile: create a new folder
    /// holding both. `in_favorites` selects which view/list the new folder
    /// belongs to.
    CreateFolderFromDrop {
        target_id: String,
        dropped_id: String,
        in_favorites: bool,
    },
    /// Drop of an app onto folder tile `usize`: add it to that folder.
    AddToFolder(usize, String),
    /// Remove an app id from the currently open folder (`open_folder`).
    RemoveFromFolder(String),
    /// Dissolve folder `usize`, returning its apps to the host view.
    UngroupFolder(usize),
    /// Toggle the in-place library-settings page.
    ToggleLibrarySettings,
    /// Set `config.position` from a dropdown index (0..=3).
    SetPosition(usize),
    /// Set `config.default_page` from a dropdown index (0..=2).
    SetDefaultPage(usize),
    /// Set `config.grid_columns`.
    SetGridColumns(u32),
    /// Set `config.grid_rows`.
    SetGridRows(u32),
    /// Show or hide the header's cosmic-settings shortcut button.
    SetShowSettings(bool),
    /// Show or hide the header's power-menu button.
    SetShowPower(bool),
}

#[derive(Clone, Debug)]
enum MenuAction {
    Remove,
    DesktopAction(String),
}

pub fn menu_button<'a, Message: Clone + 'a>(
    content: impl Into<Element<'a, Message>>,
) -> cosmic::widget::Button<'a, Message> {
    cosmic::widget::button::custom(content)
        .class(Button::MenuItem)
        .padding(menu_control_padding())
        .width(Length::Fill)
}

pub fn menu_control_padding() -> Padding {
    let theme = cosmic::theme::active();
    let cosmic = theme.cosmic();
    [cosmic.space_xxs(), cosmic.space_m()].into()
}

impl CosmicAppLibrary {
    fn current_group(&self) -> &AppGroup {
        match self.cur_group {
            None => AppLibraryConfig::home(),
            Some(FAVORITES_GROUP) => AppLibraryConfig::favorites_group(),
            Some(i) => &self.config.groups[i],
        }
    }

    /// A folder tile: same footprint as an `ApplicationButton` app tile
    /// (`IconVertical`, `FillPortion(1)` width, `space_s` padding), showing a
    /// rounded 2x2 grid of up to the folder's first four app icons plus the
    /// ellipsized folder name below. Also a drop destination for adding
    /// apps to the folder; clicking opens the in-place folder view.
    fn folder_tile<'a>(&'a self, i: usize, folder: &'a AppFolder) -> Element<'a, Message> {
        let Spacing {
            space_xxxs,
            space_xxs,
            space_s,
            ..
        } = theme::spacing();

        let mini_icon = |handle: Option<widget::icon::Handle>| -> Element<'a, Message> {
            match handle {
                Some(h) => h
                    .icon()
                    .width(Length::Fixed(26.0))
                    .height(Length::Fixed(26.0))
                    .into(),
                None => space::horizontal()
                    .width(Length::Fixed(26.0))
                    .height(Length::Fixed(26.0))
                    .into(),
            }
        };
        let icon_at = |j: usize| {
            folder
                .apps
                .get(j)
                .and_then(|id| self.all_entries.iter().find(|e| &e.id == id))
                .map(|e| e.icon.as_cosmic_icon())
        };

        let icon_grid = column![
            row![mini_icon(icon_at(0)), mini_icon(icon_at(1))].spacing(space_xxxs),
            row![mini_icon(icon_at(2)), mini_icon(icon_at(3))].spacing(space_xxxs),
        ]
        .spacing(space_xxxs)
        .align_x(Alignment::Center);

        let tile_content = column![
            container(icon_grid)
                .center(Length::Fixed(64.0))
                .class(theme::Container::Secondary),
            container(
                text(folder.name.clone())
                    .size(14.0)
                    .width(Length::Fill)
                    .center()
                    .wrapping(cosmic::iced::core::text::Wrapping::WordOrGlyph)
                    .ellipsize(cosmic::iced::core::text::Ellipsize::End(
                        cosmic::iced::core::text::EllipsizeHeightLimit::Lines(2),
                    )),
            )
            .width(Length::Fill)
            .height(Length::Fixed(40.0))
        ]
        .height(Length::Fixed(120.0))
        .spacing(space_xxs)
        .align_x(Alignment::Center)
        .width(Length::Fill);

        let btn = button::custom(tile_content)
            .width(Length::FillPortion(1))
            .class(theme::Button::IconVertical)
            .padding(space_s)
            .on_press(Message::OpenFolder(i));

        // Folder tiles are drag sources as well as drop targets, so folders can
        // be reordered by dragging one onto another. The wire payload has to be
        // an `AppletString` (the one mime type every destination here accepts),
        // so we send the folder's first app path and let `dragging_folder` —
        // set by `on_start` before any drop can land — say what is really being
        // dragged. Folders whose apps have no desktop path can't be dragged;
        // they stay plain drop targets.
        let drag_path = folder
            .apps
            .first()
            .and_then(|id| self.all_entries.iter().find(|e| &e.id == id))
            .and_then(|e| e.path.clone());
        let dragged = self.dragging_folder;

        let tile: Element<'a, Message> = match drag_path {
            Some(path) => {
                let first_icon = icon_at(0);
                dnd_source(btn)
                    .drag_icon(move |_| {
                        let handle = first_icon.clone();
                        let element: Element<'static, ()> = match handle {
                            Some(h) => h
                                .icon()
                                .width(Length::Fixed(72.0))
                                .height(Length::Fixed(72.0))
                                .into(),
                            None => space::horizontal()
                                .width(Length::Fixed(72.0))
                                .height(Length::Fixed(72.0))
                                .into(),
                        };
                        (element, tree::State::None, cosmic::iced::Vector::ZERO)
                    })
                    .drag_content(move || AppletString(path.clone()))
                    .on_start(Some(Message::StartFolderDrag(i)))
                    .on_finish(Some(Message::EndFolderDrag))
                    .on_cancel(Some(Message::EndFolderDrag))
                    .into()
            }
            None => btn.into(),
        };

        dnd_destination_for_data::<AppletString, Message>(
            tile,
            move |data: Option<AppletString>, _| match dragged {
                // Folder dropped on a folder: reorder rather than merge.
                Some(from) if from != i => Message::ReorderFolder { from, to: i },
                Some(_) => Message::IgnoredDrop,
                None => {
                    let id = data
                        .and_then(|data| load_desktop_file(&[], data.0))
                        .map(|entry| entry.id)
                        .unwrap_or_default();
                    Message::AddToFolder(i, id)
                }
            },
        )
        .drag_id(FOLDER_TILE_DRAG_ID_BASE + i as u64)
        .into()
    }

    pub fn load_apps(&mut self) {
        let xdg_current_desktop = std::env::var("XDG_CURRENT_DESKTOP").ok();
        self.all_entries = cosmic::desktop::load_applications(
            self.locale.as_slice(),
            false,
            xdg_current_desktop.as_deref(),
        )
        .filter(|d| d.exec.is_some())
        .map(Arc::new)
        .collect();
        self.all_entries.sort_by(|a, b| a.name.cmp(&b.name));

        self.entry_path_input =
            self.config
                .filtered(self.cur_group, &self.search_value, &self.all_entries);

        // collect duplicates
        self.duplicates.clear();
        self.duplicates = self
            .all_entries
            .iter()
            .enumerate()
            .fold(
                (std::mem::take(&mut self.duplicates), 0, "", ""),
                |(mut dups, cur_count, cur_name, cur_id): (HashMap<_, _>, usize, &str, &str),
                 (i, e)| {
                    if cur_name.to_lowercase().trim() == e.name.to_lowercase().trim()
                        || e.id == cur_id
                    {
                        if cur_count == 1 {
                            // insert previous entry
                            if let Some(path) = self.all_entries[i - 1].path.as_ref() {
                                let source = AppSource::from(path.as_ref());
                                let icon_handle = source.as_icon();
                                dups.insert(path.clone(), (source, icon_handle));
                            }
                        }
                        if let Some(path) = e.path.as_ref() {
                            let source = AppSource::from(path.as_ref());
                            let icon_handle = source.as_icon();
                            dups.insert(path.clone(), (source, icon_handle));
                        }
                        (dups, cur_count + 1, cur_name, cur_id)
                    } else {
                        (dups, 1, e.name.as_str(), e.id.as_str())
                    }
                },
            )
            .0;
        self.update_entry_metadata();
    }

    fn filter_apps(&mut self) -> Task<Message> {
        let config = self.config.clone();
        let all_entries = self.all_entries.clone();
        let cur_group = self.cur_group;
        let input = self.search_value.clone();
        // The folder view has no search bar, so it only takes over the grid
        // when there's no active search; a non-empty `input` (shouldn't
        // normally happen while open) falls back to the regular group view,
        // same as every other "search escapes the current view" case below.
        let open_folder = self.open_folder.filter(|_| input.is_empty());
        if !self.waiting_for_filtered {
            self.waiting_for_filtered = true;
            iced::Task::perform(
                async move {
                    let mut apps = if let Some(fi) = open_folder {
                        // Folder view: entries are the folder's own apps, in
                        // the folder's stored order.
                        config
                            .folders
                            .get(fi)
                            .map(|f| {
                                f.apps
                                    .iter()
                                    .filter_map(|id| all_entries.iter().find(|e| &e.id == id))
                                    .cloned()
                                    .collect()
                            })
                            .unwrap_or_default()
                    } else {
                        config.filtered(cur_group, &input, &all_entries)
                    };
                    // Favorites (with no active search) keep the user's custom
                    // `config.favorites` order instead of being alphabetized;
                    // the folder view keeps `AppFolder::apps` order the same
                    // way. Every other view (including searching within
                    // Favorites, which falls back to a global search) stays
                    // alphabetical.
                    if open_folder.is_none()
                        && (cur_group != Some(FAVORITES_GROUP) || !input.is_empty())
                    {
                        apps.sort_by(|a, b| a.name.cmp(&b.name));
                        // Home (with no active search) then layers the user's
                        // custom `config.home_order` on top of that
                        // alphabetical base — same idea as Favorites, but
                        // merged rather than authoritative, since Home holds
                        // every app rather than a curated list.
                        if cur_group.is_none() && input.is_empty() {
                            apps = crate::app_group::apply_home_order(&config.home_order, apps);
                        }
                    }
                    (input, apps)
                },
                |(input, apps)| Message::FilterApps(input, apps),
            )
            .map(cosmic::Action::App)
        } else {
            iced::Task::none()
        }
    }

    pub fn hide(&mut self) -> Task<Message> {
        if !matches!(self.surface_state, SurfaceState::Visible) {
            return Task::none();
        }
        // cancel existing dnd if it exists then try again...
        if self.dnd_icon.take().is_some() {
            return Task::batch(vec![
                end_dnd(),
                Task::perform(async {}, |_| cosmic::Action::App(Message::Hide)),
            ]);
        }
        self.focused_id = None;
        self.entry_ids.clear();
        self.entry_icon_handles.clear();
        self.new_group = None;
        self.new_group_keep_in_home = false;
        self.search_value.clear();
        self.edit_name = None;
        self.cur_group = None;
        self.open_folder = None;
        self.folder_name_buffer.clear();
        self.menu = None;
        self.power_menu_open = false;
        self.group_to_delete = None;
        self.fav_drop_hint = None;
        self.dragging_folder = None;
        self.scroll_offset = 0.0;
        self.settings_view = false;
        self.surface_state = SurfaceState::Hidden;
        self.hand_over.clear();

        iced::Task::batch(vec![
            destroy_popup(*MENU_ID),
            destroy_layer_surface(*NEW_GROUP_WINDOW_ID),
            destroy_layer_surface(*DELETE_GROUP_WINDOW_ID),
            destroy_layer_surface(SurfaceId::RESERVED),
        ])
    }

    fn activate_app(
        &mut self,
        i: usize,
        gpu_idx: Option<usize>,
    ) -> Task<<Self as cosmic::Application>::Message> {
        self.edit_name = None;
        if let Some(de) = self.entry_path_input.get(i) {
            let app_id = de.id.clone();
            let exec = de.exec.clone().unwrap();
            let terminal = de.terminal;
            request_token(
                Some(String::from(<Self as cosmic::Application>::APP_ID)),
                Some(SurfaceId::RESERVED),
            )
            .map(move |t| {
                cosmic::Action::App(Message::ActivationToken(
                    t,
                    app_id.clone(),
                    exec.clone(),
                    gpu_idx,
                    terminal,
                ))
            })
        } else {
            Task::none()
        }
    }
}

impl cosmic::Application for CosmicAppLibrary {
    type Message = Message;
    type Executor = executor::Default;
    type Flags = Args;
    const APP_ID: &'static str = "com.system76.CosmicAppLibrary";

    fn core(&self) -> &Core {
        &self.core
    }

    fn update(&mut self, message: Message) -> Task<Self::Message> {
        match message {
            Message::Activate => {
                return self.activate();
            }
            Message::Output(event) => {
                if matches!(event, OutputEvent::Created(_) | OutputEvent::InfoUpdate(_))
                    && self.dummy_id.is_none()
                {
                    return self.create_dummy_layer_surface();
                }
            }
            Message::UpdateFocused(id) => {
                self.focused_id = id;
                let cols = self.grid_columns();
                let i = self
                    .focused_id
                    .as_ref()
                    .and_then(|focused| self.entry_ids.iter().position(|i| i == focused))
                    .unwrap_or(0);
                let y = ((i / cols) as f32 / ((self.entry_path_input.len() / cols) as f32).max(1.))
                    .max(0.0);

                return iced_runtime::task::widget(operation::scrollable::snap_to(
                    self.scrollable_id.clone(),
                    RelativeOffset {
                        x: None,
                        y: Some(y),
                    },
                ));
            }
            Message::KeyboardNav(message) => match message {
                keyboard_nav::Action::FocusNext => {
                    return iced::Task::batch(vec![
                        iced::widget::operation::focus_next()
                            .map(|id| cosmic::Action::App(Message::UpdateFocused(id))),
                        iced_runtime::task::widget(find_focused())
                            .map(|id| cosmic::Action::App(Message::UpdateFocused(Some(id)))),
                    ]);
                }
                keyboard_nav::Action::FocusPrevious => {
                    return iced::Task::batch(vec![
                        iced::widget::operation::focus_previous()
                            .map(|id| cosmic::Action::App(Message::UpdateFocused(id))),
                        iced_runtime::task::widget(find_focused())
                            .map(|id| cosmic::Action::App(Message::UpdateFocused(Some(id)))),
                    ]);
                }
                keyboard_nav::Action::Escape => return self.on_escape(),
                keyboard_nav::Action::Search => return self.on_search(),

                keyboard_nav::Action::Fullscreen => {}
            },

            Message::PrevRow => {
                let cols = self.grid_columns();
                let mut i = self
                    .focused_id
                    .as_ref()
                    .and_then(|focused| self.entry_ids.iter().position(|i| i == focused))
                    .unwrap_or(self.entry_ids.len().saturating_add(cols - 1));
                if i == 0 {
                    self.focused_id = None;

                    return iced::Task::batch(vec![
                        iced::widget::operation::focus_previous()
                            .map(|id| cosmic::Action::App(Message::UpdateFocused(id))),
                        iced_runtime::task::widget(find_focused())
                            .map(|id| cosmic::Action::App(Message::UpdateFocused(Some(id)))),
                    ]);
                }
                i = i.saturating_sub(cols);
                let y = ((i / cols) as f32 / ((self.entry_path_input.len() / cols) as f32).max(1.))
                    .max(0.0);

                let Some(focused) = self.entry_ids.get(i).cloned() else {
                    return Task::none();
                };
                self.focused_id = Some(focused.clone());
                return Task::batch(vec![
                    iced_runtime::task::widget(focus(focused))
                        .map(|id| cosmic::Action::App(Message::UpdateFocused(Some(id)))),
                    iced_runtime::task::widget(operation::scrollable::snap_to(
                        self.scrollable_id.clone(),
                        RelativeOffset {
                            x: None,
                            y: Some(y),
                        },
                    )),
                ]);
            }
            Message::NextRow => {
                let cols = self.grid_columns();
                let mut i: i32 = self
                    .focused_id
                    .as_ref()
                    .and_then(|focused| self.entry_ids.iter().position(|i| i == focused))
                    .map(|i| i as i32)
                    .unwrap_or(-(cols as i32));
                if i == self.entry_ids.len() as i32 - 1 {
                    self.focused_id = None;
                    return iced::Task::batch(vec![
                        iced::widget::operation::focus_next()
                            .map(|id| cosmic::Action::App(Message::UpdateFocused(id))),
                        iced_runtime::task::widget(find_focused())
                            .map(|id| cosmic::Action::App(Message::UpdateFocused(Some(id)))),
                    ]);
                }
                i += cols as i32;
                i = i.min(self.entry_ids.len() as i32 - 1);
                let Some(focused) = self.entry_ids.get(i as usize).cloned() else {
                    return Task::none();
                };
                self.focused_id = Some(focused.clone());
                let y = ((i / cols as i32) as f32
                    / ((self.entry_path_input.len() / cols) as f32).max(1.))
                .max(0.0);

                return Task::batch(vec![
                    iced_runtime::task::widget(operation::scrollable::snap_to(
                        self.scrollable_id.clone(),
                        RelativeOffset {
                            x: None,
                            y: Some(y),
                        },
                    )),
                    iced_runtime::task::widget(focus(focused))
                        .map(|id| cosmic::Action::App(Message::UpdateFocused(Some(id)))),
                ]);
            }
            Message::InputChanged(value) => {
                self.search_value = value;
                return self.filter_apps();
            }
            Message::Layer(e, id) => {
                match e {
                    LayerEvent::Focused if self.menu.is_none() => {
                        if id == SurfaceId::RESERVED {
                            return text_input::focus(SEARCH_ID.clone()).chain(
                                iced_runtime::task::widget(find_focused()).map(|id| {
                                    cosmic::Action::App(Message::UpdateFocused(Some(id)))
                                }),
                            );
                        } else if id == *DELETE_GROUP_WINDOW_ID {
                            return button::focus(SUBMIT_DELETE_ID.clone());
                        } else if id == *NEW_GROUP_WINDOW_ID {
                            return text_input::focus(NEW_GROUP_ID.clone());
                        }
                    }
                    LayerEvent::Unfocused => {
                        self.last_hide = Some(Instant::now());
                        if matches!(self.surface_state, SurfaceState::Visible)
                            && id == SurfaceId::RESERVED
                            && self.menu.is_none()
                            && self.new_group.is_none()
                            && self.group_to_delete.is_none()
                        {
                            return self.hide();
                        }
                    }
                    LayerEvent::Done if id == SurfaceId::RESERVED => {
                        // no need for commands here
                        _ = self.hide();
                    }
                    _ => {}
                }
            }
            Message::Hide => {
                return self.hide();
            }
            Message::EscapePressed => {
                if self.settings_view {
                    self.settings_view = false;
                    return Task::none();
                }
                if self.open_folder.is_some() {
                    return self.update(Message::CloseFolder);
                }
                return self.hide();
            }
            Message::ActivateApp(i, gpu_idx) => {
                return self.activate_app(i, gpu_idx);
            }
            Message::StartCurAppFocus => {
                let i = if self
                    .focused_id
                    .as_ref()
                    .is_some_and(|cur_focus| cur_focus == &*SEARCH_ID)
                {
                    0
                } else {
                    self.focused_id
                        .as_ref()
                        .and_then(|focus| self.entry_ids.iter().position(|id| focus == id))
                        .unwrap_or_default()
                };
                let gpu_idx = None;
                return self.activate_app(i, gpu_idx);
            }
            Message::ActivationToken(token, app_id, exec, gpu_idx, terminal) => {
                let mut env_vars = Vec::new();
                if let Some(token) = token {
                    env_vars.push(("XDG_ACTIVATION_TOKEN".to_string(), token.clone()));
                    env_vars.push(("DESKTOP_STARTUP_ID".to_string(), token));
                }
                if let (Some(gpus), Some(idx)) = (self.gpus.as_ref(), gpu_idx) {
                    env_vars.extend(gpus[idx].environment.clone());
                }
                tokio::spawn(async move {
                    cosmic::desktop::spawn_desktop_exec(exec, env_vars, Some(&app_id), terminal)
                        .await
                });
                return self.update(Message::Hide);
            }
            Message::SelectGroup(group) => {
                self.edit_name = None;
                self.search_value.clear();
                self.cur_group = group;
                self.open_folder = None;
                self.folder_name_buffer.clear();
                self.scroll_offset = 0.0;
                self.scrollable_id = Id::new(format!("group-{group:?}"));
                let mut cmds = vec![self.filter_apps()];
                if self.cur_group.is_none() || self.cur_group == Some(FAVORITES_GROUP) {
                    cmds.push(text_input::focus(SEARCH_ID.clone()));
                }
                return iced::Task::batch(cmds);
            }
            Message::ReorderGroup(new_order) => {
                let prev_selected_key =
                    self.cur_group.and_then(|i| self.group_keys.get(i).copied());

                let reorder_keys: Vec<u64> = new_order
                    .into_iter()
                    .filter_map(|key| match key {
                        GroupRowKey::Custom(k) => Some(k),
                        GroupRowKey::Home | GroupRowKey::Favorites | GroupRowKey::NewGroup => None,
                    })
                    .collect();

                if reorder_keys.len() != self.config.groups.len() {
                    return Task::none();
                }

                let key_to_index: HashMap<u64, usize> = self
                    .group_keys
                    .iter()
                    .enumerate()
                    .map(|(i, &k)| (k, i))
                    .collect();

                let reordered: Vec<crate::app_group::AppGroup> = reorder_keys
                    .iter()
                    .filter_map(|k| {
                        key_to_index
                            .get(k)
                            .and_then(|&i| self.config.groups.get(i).cloned())
                    })
                    .collect();

                if reordered.len() != self.config.groups.len() {
                    return Task::none();
                }

                self.config.groups = reordered;
                self.group_keys = reorder_keys.clone();

                if let Some(key) = prev_selected_key {
                    self.cur_group = reorder_keys.iter().position(|&k| k == key);
                }

                if let Some(helper) = self.helper.as_ref()
                    && let Err(err) = self.config.write_entry(helper)
                {
                    error!("{:?}", err);
                }
            }
            Message::LoadApps => {
                return self.filter_apps();
            }
            Message::Delete(group) => {
                self.group_to_delete = Some(group);
                return Task::batch(vec![
                    get_layer_surface(SctkLayerSurfaceSettings {
                        id: *DELETE_GROUP_WINDOW_ID,
                        keyboard_interactivity: KeyboardInteractivity::Exclusive,
                        anchor: Anchor::empty(),
                        namespace: "dialog".into(),
                        size: None,
                        ..Default::default()
                    }),
                    button::focus(SUBMIT_DELETE_ID.clone()),
                ]);
            }
            Message::EditName(name) => {
                self.edit_name = Some(name);
            }
            Message::SubmitName => {
                if let Some(name) = self.edit_name.take()
                    && let Some(i) = self.cur_group
                {
                    self.config.set_name(i, name);
                }
                if let Some(helper) = self.helper.as_ref()
                    && let Err(err) = self.config.write_entry(helper)
                {
                    error!("{:?}", err);
                }
            }
            Message::StartEditName(name) => {
                self.edit_name = Some(name);
                return text_input::focus(EDIT_GROUP_ID.clone());
            }
            Message::StartNewGroup => {
                if self.new_group.is_some() {
                    return Task::none();
                }
                self.new_group = Some(String::new());
                self.new_group_keep_in_home = false;
                return Task::batch(vec![
                    get_layer_surface(SctkLayerSurfaceSettings {
                        id: *NEW_GROUP_WINDOW_ID,
                        keyboard_interactivity: KeyboardInteractivity::Exclusive,
                        anchor: Anchor::empty(),
                        namespace: "dialog".into(),
                        size: None,
                        ..Default::default()
                    }),
                    text_input::focus(NEW_GROUP_ID.clone()),
                ]);
            }
            Message::NewGroup(group_name) => {
                self.new_group = Some(group_name);
            }
            Message::NewGroupKeepInHome(keep_in_home) => {
                self.new_group_keep_in_home = keep_in_home;
            }
            Message::SubmitNewGroup => {
                if let Some(group_name) = self.new_group.take() {
                    self.config.add(group_name, self.new_group_keep_in_home);
                    self.group_keys.push(self.next_group_key);
                    self.next_group_key += 1;
                }
                self.new_group_keep_in_home = false;
                if let Some(helper) = self.helper.as_ref()
                    && let Err(err) = self.config.write_entry(helper)
                {
                    error!("{:?}", err);
                }
                return destroy_layer_surface(*NEW_GROUP_WINDOW_ID);
            }
            Message::CancelNewGroup => {
                self.new_group = None;
                self.new_group_keep_in_home = false;
                return destroy_layer_surface(*NEW_GROUP_WINDOW_ID);
            }
            Message::OpenContextMenu(rect, i) => {
                if self.menu.take().is_some() {
                    return destroy_popup(*MENU_ID);
                } else {
                    self.menu = Some(i);
                    let offset = self.scroll_offset as i32;
                    return cosmic::surface::surface_task(simple_popup(
                        LiveSettings::default,
                        move || {
                            SctkPopupSettings {
                        parent: SurfaceId::RESERVED,
                        id: *MENU_ID,
                        positioner: SctkPositioner {
                            size: None,
                            size_limits: Limits::NONE.min_width(1.0).min_height(1.0).max_width(300.0).max_height(800.0),
                            anchor_rect: Rectangle {
                                x: rect.x as i32,
                                y: rect.y as i32 - offset,
                                width: rect.width as i32,
                                height: rect.height as i32,
                            },
                            anchor:
                                sctk::reexports::protocols::xdg::shell::client::xdg_positioner::Anchor::Right,
                            gravity: sctk::reexports::protocols::xdg::shell::client::xdg_positioner::Gravity::Right,
                            reactive: true,
                            ..Default::default()
                        },
                        grab: false,
                        parent_size: None,
                        close_with_children: true,
                        input_zone: None,
                    }
                        },
                        None::<Box<fn() -> cosmic::Element<'static, cosmic::Action<Message>>>>,
                    ));
                }
            }
            Message::CloseContextMenu => {
                self.menu = None;
                return commands::popup::destroy_popup(*MENU_ID);
            }
            Message::ClosePowerMenu => {
                self.power_menu_open = false;
            }
            Message::StartFolderDrag(i) => {
                self.dragging_folder = Some(i);
            }
            Message::EndFolderDrag => {
                self.dragging_folder = None;
            }
            Message::IgnoredDrop => {
                self.dragging_folder = None;
                self.fav_drop_hint = None;
            }
            Message::ReorderFolder { from, to } => {
                self.dragging_folder = None;
                self.fav_drop_hint = None;
                self.config.reorder_folder(from, to);
                if let Some(helper) = self.helper.as_ref()
                    && let Err(err) = self.config.write_entry(helper)
                {
                    error!("{:?}", err);
                }
                return Task::batch(vec![end_dnd(), self.filter_apps()]);
            }
            Message::ToggleFavorite(i) => {
                self.menu = None;
                let mut tasks = vec![commands::popup::destroy_popup(*MENU_ID)];
                if let Some(info) = self.entry_path_input.get(i) {
                    let id = info.id.clone();
                    if self.config.is_favorite(&id) {
                        self.config.remove_entry(Some(FAVORITES_GROUP), &id);
                    } else {
                        self.config.add_entry(Some(FAVORITES_GROUP), &id);
                    }
                    if let Some(helper) = self.helper.as_ref()
                        && let Err(err) = self.config.write_entry(helper)
                    {
                        error!("{:?}", err);
                    }
                    tasks.push(self.filter_apps());
                }
                return Task::batch(tasks);
            }
            Message::OpenSettings => {
                self.power_menu_open = false;
                return request_token(
                    Some(String::from(<Self as cosmic::Application>::APP_ID)),
                    Some(SurfaceId::RESERVED),
                )
                .map(move |t| {
                    cosmic::Action::App(Message::ActivationToken(
                        t,
                        "com.system76.CosmicSettings".to_string(),
                        "cosmic-settings".to_string(),
                        None,
                        false,
                    ))
                });
            }
            Message::TogglePowerMenu => {
                self.power_menu_open = !self.power_menu_open;
            }
            Message::Power(action) => {
                self.power_menu_open = false;
                let mut tasks = vec![self.hide()];
                // Confirmation-style actions go through cosmic-osd (same dialog
                // as the power applet); fall back to DBus if it can't spawn.
                let use_dbus = match action.osd_arg() {
                    Some(arg) => std::process::Command::new("cosmic-osd")
                        .arg(arg)
                        .spawn()
                        .is_err(),
                    None => true,
                };
                if use_dbus {
                    tasks.push(
                        iced::Task::perform(action.perform(), |res| {
                            Message::PowerResult(res.err().map(|e| e.to_string()))
                        })
                        .map(cosmic::Action::App),
                    );
                }
                return Task::batch(tasks);
            }
            Message::PowerResult(err) => {
                if let Some(err) = err {
                    error!("power action failed: {err}");
                }
            }
            Message::SelectAction(action) => {
                let mut tasks = vec![commands::popup::destroy_popup(*MENU_ID)];
                if let Some(info) = self.menu.take().and_then(|i| self.entry_path_input.get(i)) {
                    match action {
                        MenuAction::Remove => {
                            self.config.remove_entry(self.cur_group, &info.id);
                            if let Some(helper) = self.helper.as_ref()
                                && let Err(err) = self.config.write_entry(helper)
                            {
                                error!("{:?}", err);
                            }
                            tasks.push(self.filter_apps());
                        }
                        MenuAction::DesktopAction(exec) => {
                            let mut exec = shlex::Shlex::new(&exec);

                            let mut cmd = match exec.next() {
                                Some(cmd) if !cmd.contains('=') => {
                                    tokio::process::Command::new(cmd)
                                }
                                _ => return Task::none(),
                            };
                            for arg in exec {
                                // TODO handle "%" args here if necessary?
                                if !arg.starts_with('%') {
                                    cmd.arg(arg);
                                }
                            }
                            let _ = cmd.spawn();
                            return self.hide();
                        }
                    }
                }
                return cosmic::Task::batch(tasks);
            }
            Message::StartDrag(i) => {
                self.dnd_icon = Some(i);
            }
            Message::FinishDrag(copy) => {
                self.fav_drop_hint = None;
                // In the Favorites view a finished drag is a reorder — the tile
                // drop targets already handled it via ReorderFavorite. The
                // "moved to a group" removal below must not run there, or the
                // favorite is deleted right after it was reordered. Same for
                // the folder view: its drop targets (add/remove-from-folder)
                // already handled the drag, and it isn't backed by a group.
                if self.cur_group == Some(FAVORITES_GROUP) || self.open_folder.is_some() {
                    self.dnd_icon = None;
                } else if !copy
                    && let Some(info) = self
                        .dnd_icon
                        .take()
                        .and_then(|i| self.entry_path_input.get(i))
                {
                    self.config.remove_entry(self.cur_group, &info.id);
                    if let Some(helper) = self.helper.as_ref()
                        && let Err(err) = self.config.write_entry(helper)
                    {
                        error!("{:?}", err);
                    }
                    return self.filter_apps();
                }
            }
            Message::CancelDrag => {
                self.dnd_icon = None;
                self.fav_drop_hint = None;
            }
            Message::FavDragEnter(i, zone) => {
                self.fav_drop_hint = Some((i, zone));
            }
            Message::FavDragLeave(i, zone) => {
                // Leave events can arrive after the enter for the next zone
                // (zones are adjacent), so only clear our own hint.
                if self.fav_drop_hint == Some((i, zone)) {
                    self.fav_drop_hint = None;
                }
            }
            Message::StartDndOffer(group) => {
                self.offer_group = Some(group);
            }
            Message::FinishDndOffer(group, entry) => {
                self.offer_group = None;
                let Some(entry) = entry else {
                    return Task::none();
                };
                self.config.add_entry(group, &entry.id);
                if let Some(helper) = self.helper.as_ref()
                    && let Err(err) = self.config.write_entry(helper)
                {
                    error!("{:?}", err);
                }
            }
            Message::LeaveDndOffer(group) => {
                self.offer_group = self.offer_group.filter(|g| *g != group);
            }
            Message::ReorderFavorite(id, index) => {
                self.fav_drop_hint = None;
                if id.is_empty() {
                    // Drag payload couldn't be resolved to a desktop entry; ignore.
                    return Task::none();
                }
                crate::app_group::move_within(&mut self.config.favorites, &id, index);
                if let Some(helper) = self.helper.as_ref()
                    && let Err(err) = self.config.write_entry(helper)
                {
                    error!("{:?}", err);
                }
                return self.filter_apps();
            }
            Message::ReorderFolderApp(id, index) => {
                self.fav_drop_hint = None;
                let Some(folder_idx) = self.open_folder else {
                    return Task::none();
                };
                if id.is_empty() {
                    // Drag payload couldn't be resolved to a desktop entry; ignore.
                    return Task::none();
                }
                self.config.reorder_folder_app(folder_idx, &id, index);
                if let Some(helper) = self.helper.as_ref()
                    && let Err(err) = self.config.write_entry(helper)
                {
                    error!("{:?}", err);
                }
                return self.filter_apps();
            }
            Message::ReorderHome(id, index) => {
                self.fav_drop_hint = None;
                if id.is_empty() {
                    // Drag payload couldn't be resolved to a desktop entry; ignore.
                    return Task::none();
                }
                crate::app_group::move_within(&mut self.config.home_order, &id, index);
                if let Some(helper) = self.helper.as_ref()
                    && let Err(err) = self.config.write_entry(helper)
                {
                    error!("{:?}", err);
                }
                return self.filter_apps();
            }
            Message::OpenFolder(i) => {
                self.menu = None;
                let Some(folder) = self.config.folders.get(i) else {
                    return Task::none();
                };
                self.folder_name_buffer = folder.name.clone();
                self.open_folder = Some(i);
                self.search_value.clear();
                return self.filter_apps();
            }
            Message::CloseFolder => {
                if let Some(i) = self.open_folder.take() {
                    let name = std::mem::take(&mut self.folder_name_buffer);
                    if !name.is_empty()
                        && self.config.folders.get(i).is_some_and(|f| f.name != name)
                    {
                        self.config.rename_folder(i, name);
                        if let Some(helper) = self.helper.as_ref()
                            && let Err(err) = self.config.write_entry(helper)
                        {
                            error!("{:?}", err);
                        }
                    }
                    return self.filter_apps();
                }
            }
            Message::FolderNameInput(name) => {
                self.folder_name_buffer = name;
            }
            Message::CreateFolderFromDrop {
                target_id,
                dropped_id,
                in_favorites,
            } => {
                self.fav_drop_hint = None;
                // Ignore unresolved drops and no-op self-drops (dropping an
                // app onto itself).
                if target_id.is_empty() || dropped_id.is_empty() || target_id == dropped_id {
                    return Task::none();
                }
                self.config
                    .create_folder_from_drop(&target_id, &dropped_id, in_favorites);
                if let Some(helper) = self.helper.as_ref()
                    && let Err(err) = self.config.write_entry(helper)
                {
                    error!("{:?}", err);
                }
                return self.filter_apps();
            }
            Message::AddToFolder(i, id) => {
                if id.is_empty() {
                    return Task::none();
                }
                self.config.add_to_folder(i, &id);
                if let Some(helper) = self.helper.as_ref()
                    && let Err(err) = self.config.write_entry(helper)
                {
                    error!("{:?}", err);
                }
                return self.filter_apps();
            }
            Message::RemoveFromFolder(id) => {
                if id.is_empty() {
                    return Task::none();
                }
                if let Some(i) = self.open_folder {
                    let dissolved = self.config.remove_from_folder(i, &id);
                    if dissolved {
                        self.open_folder = None;
                    }
                    if let Some(helper) = self.helper.as_ref()
                        && let Err(err) = self.config.write_entry(helper)
                    {
                        error!("{:?}", err);
                    }
                    return self.filter_apps();
                }
            }
            Message::UngroupFolder(i) => {
                self.config.ungroup_folder(i);
                // The "Ungroup" action only appears in the folder view's own
                // header (see `top_row`), so `open_folder` is exactly
                // `Some(i)` here — the folder it names no longer exists, so
                // leave the folder view.
                self.open_folder = None;
                if let Some(helper) = self.helper.as_ref()
                    && let Err(err) = self.config.write_entry(helper)
                {
                    error!("{:?}", err);
                }
                return self.filter_apps();
            }
            Message::ToggleLibrarySettings => {
                self.menu = None;
                self.power_menu_open = false;
                let mut cmds = Vec::with_capacity(2);
                if self.open_folder.is_some() {
                    cmds.push(self.update(Message::CloseFolder));
                }
                self.settings_view = !self.settings_view;
                return Task::batch(cmds);
            }
            Message::SetPosition(idx) => {
                self.config.position = match idx {
                    1 => LibraryPosition::Top,
                    2 => LibraryPosition::Bottom,
                    3 => LibraryPosition::Center,
                    _ => LibraryPosition::Auto,
                };
                if let Some(helper) = self.helper.as_ref()
                    && let Err(err) = self.config.write_entry(helper)
                {
                    error!("{:?}", err);
                }
                return set_padding::<()>(SurfaceId::RESERVED, self.layer_padding()).discard();
            }
            Message::SetDefaultPage(idx) => {
                self.config.default_page = match idx {
                    1 => DefaultPage::Home,
                    2 => DefaultPage::Favorites,
                    _ => DefaultPage::Auto,
                };
                if let Some(helper) = self.helper.as_ref()
                    && let Err(err) = self.config.write_entry(helper)
                {
                    error!("{:?}", err);
                }
            }
            Message::SetGridColumns(v) => {
                self.config.grid_columns = v.clamp(4, 12);
                if let Some(helper) = self.helper.as_ref()
                    && let Err(err) = self.config.write_entry(helper)
                {
                    error!("{:?}", err);
                }
                return set_padding::<()>(SurfaceId::RESERVED, self.layer_padding()).discard();
            }
            Message::SetGridRows(v) => {
                self.config.grid_rows = v.clamp(2, 8);
                if let Some(helper) = self.helper.as_ref()
                    && let Err(err) = self.config.write_entry(helper)
                {
                    error!("{:?}", err);
                }
                return set_padding::<()>(SurfaceId::RESERVED, self.layer_padding()).discard();
            }
            Message::SetShowSettings(show) => {
                self.config.show_settings_button = show;
                if let Some(helper) = self.helper.as_ref()
                    && let Err(err) = self.config.write_entry(helper)
                {
                    error!("{:?}", err);
                }
            }
            Message::SetShowPower(show) => {
                self.config.show_power_button = show;
                // A hidden button can't close its own open menu — close it here.
                if !show {
                    self.power_menu_open = false;
                }
                if let Some(helper) = self.helper.as_ref()
                    && let Err(err) = self.config.write_entry(helper)
                {
                    error!("{:?}", err);
                }
            }
            Message::ScrollYOffset(y) => {
                self.scroll_offset = y;
            }
            Message::ConfirmDelete => {
                let mut cmds = vec![destroy_layer_surface(*DELETE_GROUP_WINDOW_ID)];
                if let Some(group) = self.group_to_delete.take() {
                    self.config.remove(group);
                    if group < self.group_keys.len() {
                        self.group_keys.remove(group);
                    }
                    if let Some(helper) = self.helper.as_ref()
                        && let Err(err) = self.config.write_entry(helper)
                    {
                        error!("{:?}", err);
                    }
                    self.cur_group = None;
                    cmds.push(self.filter_apps());
                }
                return Task::batch(cmds);
            }
            Message::CancelDelete => {
                self.group_to_delete = None;
                return destroy_layer_surface(*DELETE_GROUP_WINDOW_ID);
            }
            Message::FilterApps(input, filtered_apps) => {
                self.entry_path_input = filtered_apps;
                self.update_entry_metadata();

                self.waiting_for_filtered = false;
                if self.search_value != input {
                    return self.filter_apps();
                }
            }
            Message::GpuUpdate(gpus) => {
                self.gpus = gpus;
            }
            Message::PinToAppTray(usize) => {
                let pinned_id = self.entry_path_input.get(usize).map(|e| e.id.clone());
                if let Some((pinned_id, app_list_helper)) = pinned_id
                    .zip(Config::new(cosmic_app_list_config::APP_ID, AppListConfig::VERSION).ok())
                {
                    self.app_list_config.add_pinned(pinned_id, &app_list_helper);
                }
                self.menu = None;
                return commands::popup::destroy_popup(*MENU_ID);
            }
            Message::UnPinFromAppTray(usize) => {
                let pinned_id = self.entry_path_input.get(usize).map(|e| e.id.clone());
                if let Some((pinned_id, app_list_helper)) = pinned_id
                    .zip(Config::new(cosmic_app_list_config::APP_ID, AppListConfig::VERSION).ok())
                {
                    self.app_list_config
                        .remove_pinned(&pinned_id, &app_list_helper);
                }
                self.menu = None;
                return commands::popup::destroy_popup(*MENU_ID);
            }
            Message::AppListConfig(config) => {
                self.app_list_config = config;
            }
            Message::Opened(size, window_id) => {
                let mut tasks = Vec::new();
                if let Some(dummy) = self.dummy_id
                    && window_id == dummy
                {
                    tasks.push(overlap_notify(window_id, true));
                } else if self.dummy_id.is_none() {
                    tasks.push(overlap_notify(SurfaceId::RESERVED, true));
                }
                if window_id == SurfaceId::RESERVED {
                    if matches!(self.surface_state, SurfaceState::WaitingToBeShown) {
                        self.surface_state = SurfaceState::Visible;
                    }
                    self.size = size;
                    tasks.push(self.handle_overlap());
                }
                if !self.hand_over.is_empty() {
                    let input = self.hand_over.clone();
                    self.hand_over.clear();
                    tasks.push(self.update(Message::InputChanged(input)));
                }
                return Task::batch(tasks);
            }
            Message::Overlap(overlap_notify_event) => match overlap_notify_event {
                OverlapNotifyEvent::OverlapLayerAdd {
                    identifier,
                    namespace,
                    logical_rect,
                    exclusive,
                    ..
                } => {
                    if exclusive > 0 || namespace == "Dock" || namespace == "Panel" {
                        if self.needs_clear {
                            self.needs_clear = false;
                            self.overlap.clear();
                        }
                        self.overlap.insert(identifier, logical_rect);
                    }
                    return self.handle_overlap();
                }
                OverlapNotifyEvent::OverlapLayerRemove { identifier } => {
                    self.overlap.remove(&identifier);
                    return self.handle_overlap();
                }
                _ => {}
            },
        }
        Task::none()
    }

    fn dbus_activation(&mut self, msg: dbus_activation::Message) -> Task<Self::Message> {
        match msg.msg {
            dbus_activation::Details::Activate => self.activate(),
            dbus_activation::Details::ActivateAction { action, .. } => {
                let Ok(cmd) = ApplicationsTasks::from_str(&action) else {
                    return Task::none();
                };
                match cmd {
                    ApplicationsTasks::Input { input } => {
                        if let Some(input) = input {
                            self.hand_over.push_str(&input);
                        }
                        if self.surface_state == SurfaceState::Hidden {
                            return self.activate();
                        }
                        Task::none()
                    }
                    ApplicationsTasks::Close => self.hide(),
                    // Run is handled at startup, not via D-Bus
                    ApplicationsTasks::Run => Task::none(),
                }
            }
            _ => Task::none(),
        }
    }

    fn view<'a>(&'a self) -> Element<'a, Message> {
        unimplemented!()
    }

    fn view_window<'a>(&'a self, id: SurfaceId) -> Element<'a, Message> {
        if self.dummy_id.is_some_and(|dummy| dummy == id) {
            return horizontal().into();
        }
        let Spacing {
            space_none,
            space_xxs,
            space_xs,
            space_s,
            space_l,
            space_xxl,
            ..
        } = theme::spacing();

        if id == *MENU_ID {
            let Some((menu, i)) = self
                .menu
                .as_ref()
                .and_then(|i| self.entry_path_input.get(*i).map(|e| (e, i)))
            else {
                return container(space::horizontal())
                    .width(Length::Fixed(1.0))
                    .height(Length::Fixed(1.0))
                    .into();
            };

            let mut list_column = Vec::new();

            if let Some(gpus) = self.gpus.as_ref() {
                for (j, gpu) in gpus.iter().enumerate() {
                    let default_idx = if menu.prefers_dgpu {
                        gpus.iter().position(|gpu| !gpu.default).unwrap_or(0)
                    } else {
                        gpus.iter().position(|gpu| gpu.default).unwrap_or(0)
                    };
                    list_column.push(
                        menu_button(text::body(format!(
                            "{} {}",
                            fl!("run-on", gpu = gpu.name.as_str()),
                            if j == default_idx {
                                fl!("run-on-default")
                            } else {
                                String::new()
                            }
                        )))
                        .on_press(Message::ActivateApp(*i, Some(j)))
                        .into(),
                    )
                }
            } else {
                list_column.push(
                    menu_button(text::body(RUN.clone()))
                        .on_press(Message::ActivateApp(*i, None))
                        .into(),
                );
            }

            if !menu.desktop_actions.is_empty() {
                list_column.push(divider::horizontal::light().into());
                for action in menu.desktop_actions.iter() {
                    list_column.push(
                        menu_button(text::body(&action.name))
                            .on_press(Message::SelectAction(MenuAction::DesktopAction(
                                action.exec.clone(),
                            )))
                            .into(),
                    );
                }
            }

            // add to pinned
            let svg_accent = Rc::new(|theme: &cosmic::Theme| {
                let color = theme.cosmic().accent_color().into();
                svg::Style { color: Some(color) }
            });
            let is_pinned = self.app_list_config.favorites.iter().any(|p| p == &menu.id);
            let pin_to_app_tray = menu_button(
                if is_pinned {
                    row![
                        icon::icon(icon::from_name("checkbox-checked-symbolic").size(16).into())
                            .class(cosmic::theme::Svg::Custom(svg_accent.clone())),
                        text::body(fl!("pin-to-app-tray"))
                    ]
                } else {
                    row![
                        space::horizontal().width(16.0),
                        text::body(fl!("pin-to-app-tray"))
                    ]
                }
                .spacing(space_xxs),
            )
            .on_press(if is_pinned {
                Message::UnPinFromAppTray(*i)
            } else {
                Message::PinToAppTray(*i)
            });
            let is_fav = self.config.is_favorite(&menu.id);
            list_column.push(divider::horizontal::light().into());
            list_column.push(
                menu_button(text::body(if is_fav {
                    fl!("remove-favorite")
                } else {
                    fl!("add-favorite")
                }))
                .on_press(Message::ToggleFavorite(*i))
                .into(),
            );

            list_column.push(divider::horizontal::light().into());
            list_column.push(pin_to_app_tray.into());

            if self.cur_group.is_some_and(|g| g != FAVORITES_GROUP) {
                list_column.push(divider::horizontal::light().into());
                list_column.push(
                    menu_button(text::body(REMOVE.clone()))
                        .on_press(Message::SelectAction(MenuAction::Remove))
                        .into(),
                );
            }

            if self.open_folder.is_some() {
                list_column.push(divider::horizontal::light().into());
                list_column.push(
                    menu_button(text::body(fl!("remove-from-folder")))
                        .on_press(Message::RemoveFromFolder(menu.id.clone()))
                        .into(),
                );
            }

            return autosize(
                container(scrollable(MenuColumn::with_children(list_column))).padding(1),
                MENU_AUTOSIZE_ID.clone(),
            )
            .max_height(800.)
            .max_width(300.)
            .into();
        }
        if id == *NEW_GROUP_WINDOW_ID {
            let Some(group_name) = self.new_group.as_ref() else {
                return container(space::horizontal())
                    .width(Length::Fixed(1.0))
                    .height(Length::Fixed(1.0))
                    .into();
            };
            let dialog = widget::dialog::dialog()
                .title(CREATE_NEW.as_str())
                .control(
                    column![
                        text_input("", group_name)
                            .label(&*NEW_GROUP_PLACEHOLDER)
                            .on_input(Message::NewGroup)
                            .on_submit(|_| Message::SubmitNewGroup)
                            .width(Length::Fixed(432.0))
                            .size(14)
                            .id(NEW_GROUP_ID.clone()),
                        widget::toggler(self.new_group_keep_in_home)
                            .label(fl!("keep-in-home"))
                            .on_toggle(Message::NewGroupKeepInHome)
                            .width(Length::Fixed(432.0)),
                    ]
                    .spacing(space_s)
                    .width(Length::Fixed(432.0)),
                )
                .primary_action(
                    button::custom(text::body(SAVE.as_str()).center().width(Length::Fill))
                        .class(Button::Suggested)
                        .on_press(Message::SubmitNewGroup)
                        .padding([space_xxs, space_s])
                        .width(142),
                )
                .secondary_action(
                    button::custom(text::body(CANCEL.as_str()).center().width(Length::Fill))
                        .on_press(Message::CancelNewGroup)
                        .padding([space_xxs, space_s])
                        .width(142),
                )
                .width(Length::Fixed(432.0));

            return autosize(dialog, NEW_GROUP_AUTOSIZE_ID.clone()).into();
        }
        if id == *DELETE_GROUP_WINDOW_ID {
            let dialog = widget::dialog::dialog()
                .icon(icon::from_name("edit-delete-symbolic").size(48))
                .title(fl!("delete-folder"))
                .body(fl!("delete-folder", "msg"))
                .primary_action(
                    button::custom(text::body(fl!("delete")).center().width(Length::Fill))
                        .id(SUBMIT_DELETE_ID.clone())
                        .class(Button::Destructive)
                        .on_press(Message::ConfirmDelete)
                        .padding([space_xxs, space_s])
                        .width(142),
                )
                .secondary_action(
                    button::custom(text::body(CANCEL.to_string()).center().width(Length::Fill))
                        .on_press(Message::CancelDelete)
                        .padding([space_xxs, space_s])
                        .width(142),
                )
                .width(Length::Fixed(432.0));

            return autosize(dialog, DELETE_GROUP_AUTOSIZE_ID.clone()).into();
        }

        let cur_group = self.current_group();
        let favorites_view = self.cur_group == Some(FAVORITES_GROUP);
        let folder_view = self.open_folder.is_some();
        // Home: the other view (besides Favorites and an open folder) with a
        // user-defined order, via `config.home_order`.
        let home_view = self.cur_group.is_none() && !folder_view;
        let top_row = if let Some(open_folder_idx) = self.open_folder {
            let folder_name = self
                .config
                .folders
                .get(open_folder_idx)
                .map(|f| f.name.clone())
                .unwrap_or_default();
            row![
                container(
                    button::custom(
                        icon::icon(from_name("go-previous-symbolic").into())
                            .width(Length::Fixed(32.0))
                            .height(Length::Fixed(32.0)),
                    )
                    .padding(space_xs)
                    .class(Button::Icon)
                    .on_press(Message::CloseFolder)
                )
                .height(Length::Fixed(96.0))
                .align_y(Vertical::Center)
                .width(Length::FillPortion(1)),
                container(
                    text_input(folder_name, &self.folder_name_buffer)
                        .on_input(Message::FolderNameInput)
                        .on_paste(Message::FolderNameInput)
                        .on_submit(|_| Message::CloseFolder)
                        .width(Length::Fixed(300.0))
                        .size(14),
                )
                .width(Length::Fill)
                .center_x(Length::FillPortion(8)),
                row![
                    space::horizontal(),
                    button::text(fl!("ungroup-folder"))
                        .on_press(Message::UngroupFolder(open_folder_idx)),
                ]
                .width(Length::FillPortion(1))
            ]
            .padding([0, space_l])
            .align_y(Alignment::Center)
        } else if self.cur_group.is_none() || favorites_view {
            let library_settings_button = tooltip(
                button::custom(
                    icon::icon(from_name("emblem-system-symbolic").into())
                        .width(Length::Fixed(32.0))
                        .height(Length::Fixed(32.0)),
                )
                .padding(space_xs)
                .class(Button::Icon)
                .on_press(Message::ToggleLibrarySettings),
                text(fl!("library-settings")),
                tooltip::Position::Bottom,
            );

            let settings_button = tooltip(
                button::custom(
                    // A toggle in a ring (bundled, symbolic) — the stock
                    // `preferences-system-symbolic` glyph is byte-identical to
                    // the `emblem-system` gear used by Library Settings next to
                    // it, which made the two buttons indistinguishable.
                    icon::icon(crate::icon_cache::icon_cache_handle(
                        "cosmic-settings-toggle-symbolic",
                        32,
                    ))
                    .width(Length::Fixed(32.0))
                    .height(Length::Fixed(32.0)),
                )
                .padding(space_xs)
                .class(Button::Icon)
                .on_press(Message::OpenSettings),
                text(fl!("settings")),
                tooltip::Position::Bottom,
            );

            let power_button = button::custom(
                icon::icon(from_name("system-shutdown-symbolic").into())
                    .width(Length::Fixed(32.0))
                    .height(Length::Fixed(32.0)),
            )
            .padding(space_xs)
            .class(Button::Icon)
            .on_press(Message::TogglePowerMenu);

            let power_element: Element<'_, Message> = if self.power_menu_open {
                let power_menu = container(MenuColumn::with_children(vec![
                    menu_button(text::body(fl!("lock-screen")))
                        .on_press(Message::Power(PowerAction::Lock))
                        .into(),
                    menu_button(text::body(fl!("suspend")))
                        .on_press(Message::Power(PowerAction::Suspend))
                        .into(),
                    divider::horizontal::light().into(),
                    menu_button(text::body(fl!("log-out")))
                        .on_press(Message::Power(PowerAction::LogOut))
                        .into(),
                    menu_button(text::body(fl!("restart")))
                        .on_press(Message::Power(PowerAction::Restart))
                        .into(),
                    menu_button(text::body(fl!("shutdown")))
                        .on_press(Message::Power(PowerAction::Shutdown))
                        .into(),
                ]))
                .width(Length::Fixed(220.0))
                .padding(1)
                .class(theme::Container::Dropdown);

                // Non-modal on purpose: a modal popover captures every mouse
                // event (libcosmic `popover::update`), which both swallows the
                // click-outside and stops `on_close` from ever firing, so the
                // menu could only be dismissed via the power button itself.
                // Non-modal publishes `on_close` on any press outside the
                // button's bounds; app tiles are made unclickable below while
                // the menu is open so a dismissing click can't also launch an
                // app through the popup.
                popover(power_button)
                    .popup(power_menu)
                    .position(popover::Position::Bottom)
                    .modal(false)
                    .on_close(Message::ClosePowerMenu)
                    .into()
            } else {
                tooltip(power_button, text(fl!("power")), tooltip::Position::Bottom).into()
            };

            row![
                space::horizontal().width(Length::FillPortion(1)),
                container(
                    search_input(SEARCH_PLACEHOLDER.as_str(), self.search_value.as_str())
                        .on_input(Message::InputChanged)
                        .on_paste(Message::InputChanged)
                        .on_submit(|_| Message::StartCurAppFocus)
                        .style(TextInput::Search)
                        .width(Length::Fixed(400.0))
                        .size(14)
                        .id(SEARCH_ID.clone())
                )
                .align_y(Vertical::Center)
                .height(Length::Fixed(96.0)),
                row![space::horizontal(), library_settings_button]
                    .push_maybe(
                        self.config
                            .show_settings_button
                            .then(|| Element::from(settings_button)),
                    )
                    .push_maybe(self.config.show_power_button.then_some(power_element))
                    .spacing(space_xxs)
                    .align_y(Alignment::Center)
                    .width(Length::FillPortion(1))
            ]
            .padding([0, space_l])
            .align_y(Alignment::Center)
            .spacing(space_xxs)
        } else {
            row![
                space::horizontal().width(Length::FillPortion(1)),
                if let Some(edit_name) = self.edit_name.as_ref() {
                    container(
                        text_input(cur_group.name(), edit_name)
                            .on_input(Message::EditName)
                            .on_paste(Message::EditName)
                            .on_clear(Message::EditName(String::new()))
                            .on_submit(|_| Message::SubmitName)
                            .id(EDIT_GROUP_ID.clone())
                            .width(Length::Fill)
                            .size(14),
                    )
                    .width(Length::Fill)
                    .center_x(Length::FillPortion(8))
                } else {
                    container(
                        text(cur_group.name())
                            .size(24)
                            .width(Length::Fill)
                            .center()
                            .ellipsize(cosmic::iced::core::text::Ellipsize::End(
                                cosmic::iced::core::text::EllipsizeHeightLimit::Lines(1),
                            )),
                    )
                    .width(Length::Fill)
                    .center_x(Length::FillPortion(8))
                },
                row![
                    space::horizontal(),
                    tooltip(
                        {
                            let mut b = button::custom(
                                icon::icon(icon::from_name("edit-symbolic").into())
                                    .width(Length::Fixed(32.0))
                                    .height(Length::Fixed(32.0)),
                            )
                            .padding(space_xs)
                            .class(Button::Icon);
                            if self.edit_name.is_none() {
                                b = b.on_press(Message::StartEditName(cur_group.name()));
                            }
                            container(b)
                                .height(Length::Fixed(96.0))
                                .align_y(Vertical::Center)
                        },
                        text(fl!("rename")),
                        tooltip::Position::Bottom
                    ),
                    tooltip(
                        container(
                            button::custom(
                                icon::icon(icon::from_name("edit-delete-symbolic").into())
                                    .width(Length::Fixed(32.0))
                                    .height(Length::Fixed(32.0)),
                            )
                            .padding(space_xs)
                            .class(Button::Icon)
                            .on_press_maybe(self.cur_group.map(Message::Delete))
                        )
                        .height(Length::Fixed(96.0))
                        .align_y(Vertical::Center),
                        text(fl!("delete")),
                        tooltip::Position::Bottom
                    )
                ]
                .spacing(space_xxs)
                .width(Length::FillPortion(1))
            ]
            .padding([0, space_l])
            .align_y(Alignment::Center)
        };

        // Trailing "append to end" drop zone shown after the last favorite tile
        // (favorites view only, no active search): dropping an app here moves or
        // inserts it at the end of `config.favorites`. Sized like a tile so it
        // slots into the grid; the row-filler below absorbs any leftover width.
        let append_zone =
            (favorites_view && self.search_value.is_empty() && !self.entry_path_input.is_empty())
                .then(|| {
                    let fav_len = self.config.favorites.len();
                    let dragging_folder = self.dragging_folder;
                    Element::from(
                        dnd_destination_for_data::<AppletString, Message>(
                            iced::widget::space::horizontal()
                                .width(Length::FillPortion(1))
                                .height(Length::Fixed(120.0 + 2.0 * space_s as f32)),
                            move |data: Option<AppletString>, _| {
                                if dragging_folder.is_some() {
                                    return Message::IgnoredDrop;
                                }
                                let id = data
                                    .and_then(|data| load_desktop_file(&[], data.0))
                                    .map(|entry| entry.id)
                                    .unwrap_or_default();
                                Message::ReorderFavorite(id, fav_len)
                            },
                        )
                        .drag_id(FAVORITE_TILE_DRAG_ID_BASE + 999_999),
                    )
                });

        // Folder tiles prepend the grid of their host view (Home or
        // Favorites), search empty only, and never inside the folder view
        // itself (folders don't nest).
        let show_folder_tiles = self.search_value.is_empty()
            && !folder_view
            && (self.cur_group.is_none() || favorites_view);
        let folder_tiles: Vec<Element<'_, Message>> = if show_folder_tiles {
            self.config
                .folders
                .iter()
                .enumerate()
                .filter(|(_, f)| f.in_favorites == favorites_view)
                .map(|(i, f)| self.folder_tile(i, f))
                .collect()
        } else {
            Vec::new()
        };
        let has_folder_tiles = !folder_tiles.is_empty();

        // TODO grid widget in libcosmic
        let app_tiles = self
            .entry_path_input
            .iter()
            .zip(self.entry_ids.iter())
            .zip(self.entry_icon_handles.iter())
            .enumerate()
            .map(|(i, ((entry, id), icon_handle))| {
                let gpu_idx = self.gpus.as_ref().map(|gpus| {
                    if entry.prefers_dgpu {
                        gpus.iter().position(|gpu| !gpu.default).unwrap_or(0)
                    } else {
                        gpus.iter().position(|gpu| gpu.default).unwrap_or(0)
                    }
                });
                let dup = entry
                    .path
                    .as_ref()
                    .and_then(|path| self.duplicates.get(path));
                let selected = self.menu.is_some_and(|m| m == i);
                // While a drag hovers this tile's icon area in favorites or
                // Home, light it up with the accent selected style — the
                // "release here combines these into a folder" hint.
                let combine_hint = (favorites_view || home_view)
                    && self.fav_drop_hint == Some((i, FavDropZone::Onto));

                let b = ApplicationButton::new(
                    id.clone(),
                    &entry.name,
                    icon_handle.clone(),
                    &entry.path,
                    move |rect| Message::OpenContextMenu(rect, i),
                    if self.power_menu_open {
                        // The power popover is non-modal, so a click meant to
                        // dismiss it would otherwise fall through and launch
                        // whatever tile is underneath.
                        None
                    } else if self.menu.is_none() {
                        Some(Message::ActivateApp(i, gpu_idx))
                    } else if selected {
                        Some(Message::CloseContextMenu)
                    } else {
                        None
                    },
                    // TODO add icon and text if duplicated
                    dup,
                    selected || combine_hint,
                    self.menu.is_none().then_some(Message::StartDrag(i)),
                    self.menu.is_none().then_some(Message::FinishDrag(false)),
                    self.menu.is_none().then_some(Message::CancelDrag),
                );

                if favorites_view || folder_view || home_view {
                    // Positional drops: the narrow strips flanking the tile
                    // insert-reorder the dragged app before/after it. In
                    // Favorites and Home the tile's own area additionally
                    // combines both apps into a folder; inside a folder view
                    // there's nothing to combine into (folders don't nest), so
                    // the tile is left plain and only the strips are
                    // destinations. Three (or two) SIBLING destinations per
                    // cell — never nested, so hit-testing stays unambiguous.
                    let cell_height = 120.0 + 2.0 * space_s as f32;
                    let strip = |zone: FavDropZone| -> Element<'a, Message> {
                        let active = self.fav_drop_hint == Some((i, zone));
                        let bar: Element<'a, Message> = if active {
                            // Accent insertion bar: "release here reorders".
                            container(space::vertical())
                                .width(Length::Fixed(4.0))
                                .height(Length::Fixed(104.0))
                                .class(theme::Container::Custom(Box::new(|theme| {
                                    let t = theme.cosmic();
                                    container::Style {
                                        background: Some(Color::from(t.accent_color()).into()),
                                        border: Border {
                                            radius: 2.0.into(),
                                            ..Default::default()
                                        },
                                        ..Default::default()
                                    }
                                })))
                                .into()
                        } else {
                            space::horizontal().width(Length::Fixed(4.0)).into()
                        };
                        container(bar)
                            .center_x(Length::Fixed(24.0))
                            .center_y(Length::Fixed(cell_height))
                            .into()
                    };
                    let dragging_folder = self.dragging_folder;
                    let reorder_zone =
                        |element: Element<'a, Message>, insert_at: usize, zone: FavDropZone| {
                            dnd_destination_for_data::<AppletString, Message>(
                                element,
                                move |data: Option<AppletString>, _| {
                                    // A folder can only be reordered against
                                    // other folder tiles, never slotted into an
                                    // app list.
                                    if dragging_folder.is_some() {
                                        return Message::IgnoredDrop;
                                    }
                                    let id = data
                                        .and_then(|data| load_desktop_file(&[], data.0))
                                        .map(|entry| entry.id)
                                        .unwrap_or_default();
                                    if folder_view {
                                        Message::ReorderFolderApp(id, insert_at)
                                    } else if home_view {
                                        Message::ReorderHome(id, insert_at)
                                    } else {
                                        Message::ReorderFavorite(id, insert_at)
                                    }
                                },
                            )
                            .drag_id(
                                FAV_STRIP_DRAG_ID_BASE
                                    + 2 * i as u64
                                    + u64::from(zone == FavDropZone::After),
                            )
                            .on_enter(move |_, _, _| Message::FavDragEnter(i, zone))
                            .on_leave(move || Message::FavDragLeave(i, zone))
                        };
                    // Inside a folder view the tile stays a plain button:
                    // there's no "combine" to offer, and making it a
                    // destination would only steal drops from the strips.
                    // Favorites and Home both offer combine-into-folder, just
                    // against different backing lists (`favorites` vs. a Home
                    // tile has none — `in_favorites` tags the new folder).
                    let center: Element<'a, Message> = if folder_view {
                        b.into()
                    } else {
                        let target_id = entry.id.clone();
                        let tile_drag_id_base = if home_view {
                            HOME_TILE_DRAG_ID_BASE
                        } else {
                            FAVORITE_TILE_DRAG_ID_BASE
                        };
                        dnd_destination_for_data::<AppletString, Message>(
                            b,
                            move |data: Option<AppletString>, _| {
                                // Dropping a folder onto an app tile has no
                                // meaning — folders don't nest.
                                if dragging_folder.is_some() {
                                    return Message::IgnoredDrop;
                                }
                                let dropped_id = data
                                    .and_then(|data| load_desktop_file(&[], data.0))
                                    .map(|entry| entry.id)
                                    .unwrap_or_default();
                                Message::CreateFolderFromDrop {
                                    target_id: target_id.clone(),
                                    dropped_id,
                                    in_favorites: !home_view,
                                }
                            },
                        )
                        .drag_id(tile_drag_id_base + i as u64)
                        .on_enter(move |_, _, _| Message::FavDragEnter(i, FavDropZone::Onto))
                        .on_leave(move || Message::FavDragLeave(i, FavDropZone::Onto))
                        .into()
                    };
                    row![
                        reorder_zone(strip(FavDropZone::Before), i, FavDropZone::Before),
                        center,
                        reorder_zone(strip(FavDropZone::After), i + 1, FavDropZone::After),
                    ]
                    .width(Length::FillPortion(1))
                    .into()
                } else {
                    b.into()
                }
            });

        let cols = self.grid_columns();
        let app_grid_list: Vec<_> = folder_tiles
            .into_iter()
            .chain(app_tiles)
            .chain(append_zone)
            .chunks(cols)
            .into_iter()
            .map(|row_chunk| {
                let mut new_row = row_chunk.collect_vec();
                let missing = cols - new_row.len();
                if missing > 0 {
                    new_row.push(
                        iced::widget::space::horizontal()
                            .width(Length::FillPortion(missing.try_into().unwrap()))
                            .into(),
                    );
                }
                row(new_row).spacing(space_xxs).into()
            })
            .collect();

        let app_scrollable = if favorites_view
            && self.entry_path_input.is_empty()
            && self.search_value.is_empty()
            && !has_folder_tiles
        {
            container(text::body(fl!("favorites-empty")))
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .max_height(self.grid_max_height())
        } else {
            container(
                scrollable(
                    column(app_grid_list)
                        .width(Length::Fill)
                        .spacing(space_xxs)
                        // padding on top needed to avoid focus highlight clipping
                        .padding([4, space_xxl, space_xxs, space_xxl]),
                )
                .on_scroll(|viewport| Message::ScrollYOffset(viewport.absolute_offset().y))
                .id(self.scrollable_id.clone())
                .height(Length::Fill),
            )
            .max_height(self.grid_max_height())
        };

        // TODO use the spacing variables from the theme
        let (group_icon_size, h_padding, group_width) = if self.config.groups.len() + 2 > 15 {
            (16.0, space_xxs, 96.0)
        } else {
            (32.0, space_s, 128.0)
        };
        let group_height =
            group_icon_size + 21.0 + (space_none as f32) + (space_xxs as f32) + (space_s as f32);

        let build_group_button = |group_ref: Option<usize>, group: &crate::app_group::AppGroup| {
            let is_active = self.offer_group == Some(group_ref)
                || (self.cur_group == group_ref && self.offer_group.is_none());
            dnd_destination_for_data::<AppletString, Message>(
                button::custom(
                    column![
                        container(
                            icon::icon(from_name(group.icon.clone()).into())
                                .width(Length::Fixed(group_icon_size))
                                .height(Length::Fixed(group_icon_size))
                        )
                        .padding(space_xxs),
                        text::body(group.name())
                            .width(Length::Fill)
                            .center()
                            .ellipsize(cosmic::iced::core::text::Ellipsize::End(
                                cosmic::iced::core::text::EllipsizeHeightLimit::Lines(1),
                            ))
                    ]
                    .align_x(Alignment::Center)
                    .width(Length::Fill),
                )
                .height(Length::Fixed(group_height))
                .width(Length::Fixed(group_width))
                .class(Button::IconVertical)
                // Use the standard cosmic selected-state styling (accent-tinted
                // overlay + accent text) instead of forcing the low-contrast
                // "pressed" background, which was barely visible (upstream #338).
                .selected(is_active)
                .padding([space_none, h_padding, space_xxs, h_padding])
                .on_press_maybe(
                    self.menu
                        .is_none()
                        .then_some(Message::SelectGroup(group_ref)),
                ),
                move |data, _| {
                    Message::FinishDndOffer(
                        group_ref,
                        data.and_then(|data| load_desktop_file(&[], data.0)),
                    )
                },
            )
            .drag_id(group_ref.map(|i| i as u64 + 1).unwrap_or(0))
            .on_enter(move |_, _, _| Message::StartDndOffer(group_ref))
            .on_leave(move || Message::LeaveDndOffer(group_ref))
        };

        let add_group_btn = button::custom(
            column![
                container(
                    icon::icon(icon::from_name("folder-new-symbolic").into())
                        .width(Length::Fixed(group_icon_size))
                        .height(Length::Fixed(group_icon_size))
                )
                .padding(space_xxs),
                text::body(ADD_GROUP.as_str())
                    .width(Length::Fill)
                    .center()
                    .ellipsize(cosmic::iced::core::text::Ellipsize::End(
                        cosmic::iced::core::text::EllipsizeHeightLimit::Lines(1),
                    ))
            ]
            .align_x(Alignment::Center)
            .width(Length::Fill),
        )
        .height(Length::Fixed(group_height))
        .width(Length::Fixed(group_width))
        .class(theme::Button::IconVertical)
        .padding([space_none, h_padding, space_xxs, h_padding])
        .on_press(Message::StartNewGroup);

        let home = AppLibraryConfig::home();
        let group_row = self
            .config
            .groups
            .iter()
            .enumerate()
            .fold(
                reorderable_flex_row::<GroupRowKey, Message>(Message::ReorderGroup)
                    .spacing(space_xxs)
                    .padding([space_s, space_none])
                    .push_locked(GroupRowKey::Home, build_group_button(None, home))
                    .push_locked(
                        GroupRowKey::Favorites,
                        build_group_button(
                            Some(FAVORITES_GROUP),
                            AppLibraryConfig::favorites_group(),
                        ),
                    ),
                |row, (i, group)| {
                    let key = self.group_keys.get(i).copied().unwrap_or(i as u64);
                    row.push(GroupRowKey::Custom(key), build_group_button(Some(i), group))
                },
            )
            .push_locked(GroupRowKey::NewGroup, add_group_btn);

        let content: Element<'_, Message> = if self.settings_view {
            let settings_top_row = row![
                container(
                    button::custom(
                        icon::icon(from_name("go-previous-symbolic").into())
                            .width(Length::Fixed(32.0))
                            .height(Length::Fixed(32.0)),
                    )
                    .padding(space_xs)
                    .class(Button::Icon)
                    .on_press(Message::ToggleLibrarySettings)
                )
                .height(Length::Fixed(96.0))
                .align_y(Vertical::Center)
                .width(Length::FillPortion(1)),
                container(
                    text(fl!("library-settings"))
                        .size(24)
                        .width(Length::Fill)
                        .center(),
                )
                .width(Length::Fill)
                .center_x(Length::FillPortion(8)),
                space::horizontal().width(Length::FillPortion(1)),
            ]
            .padding([0, space_l])
            .align_y(Alignment::Center);

            let pos_idx = match self.config.position {
                LibraryPosition::Auto => 0,
                LibraryPosition::Top => 1,
                LibraryPosition::Bottom => 2,
                LibraryPosition::Center => 3,
            };
            let position_row = row![
                text::body(fl!("position")).width(Length::Fill),
                cosmic::widget::dropdown(&POSITION_LABELS[..], Some(pos_idx), Message::SetPosition),
            ]
            .align_y(Alignment::Center)
            .height(Length::Fixed(48.0));

            let page_idx = match self.config.default_page {
                DefaultPage::Auto => 0,
                DefaultPage::Home => 1,
                DefaultPage::Favorites => 2,
            };
            let default_page_row = row![
                text::body(fl!("default-page")).width(Length::Fill),
                cosmic::widget::dropdown(
                    &DEFAULT_PAGE_LABELS[..],
                    Some(page_idx),
                    Message::SetDefaultPage
                ),
            ]
            .align_y(Alignment::Center)
            .height(Length::Fixed(48.0));

            let cur_cols = self.config.grid_columns.clamp(4, 12);
            let columns_row = row![
                text::body(fl!("grid-columns")).width(Length::Fill),
                button::custom(
                    icon::icon(from_name("list-remove-symbolic").into())
                        .width(Length::Fixed(16.0))
                        .height(Length::Fixed(16.0))
                )
                .padding(space_xs)
                .class(Button::Icon)
                .on_press_maybe((cur_cols > 4).then(|| Message::SetGridColumns(cur_cols - 1))),
                container(text::body(cur_cols.to_string()).center())
                    .width(Length::Fixed(40.0))
                    .center_x(Length::Fixed(40.0)),
                button::custom(
                    icon::icon(from_name("list-add-symbolic").into())
                        .width(Length::Fixed(16.0))
                        .height(Length::Fixed(16.0))
                )
                .padding(space_xs)
                .class(Button::Icon)
                .on_press_maybe((cur_cols < 12).then(|| Message::SetGridColumns(cur_cols + 1))),
            ]
            .align_y(Alignment::Center)
            .height(Length::Fixed(48.0));

            let cur_rows = self.config.grid_rows.clamp(2, 8);
            let rows_row = row![
                text::body(fl!("grid-rows")).width(Length::Fill),
                button::custom(
                    icon::icon(from_name("list-remove-symbolic").into())
                        .width(Length::Fixed(16.0))
                        .height(Length::Fixed(16.0))
                )
                .padding(space_xs)
                .class(Button::Icon)
                .on_press_maybe((cur_rows > 2).then(|| Message::SetGridRows(cur_rows - 1))),
                container(text::body(cur_rows.to_string()).center())
                    .width(Length::Fixed(40.0))
                    .center_x(Length::Fixed(40.0)),
                button::custom(
                    icon::icon(from_name("list-add-symbolic").into())
                        .width(Length::Fixed(16.0))
                        .height(Length::Fixed(16.0))
                )
                .padding(space_xs)
                .class(Button::Icon)
                .on_press_maybe((cur_rows < 8).then(|| Message::SetGridRows(cur_rows + 1))),
            ]
            .align_y(Alignment::Center)
            .height(Length::Fixed(48.0));

            let show_settings_row = row![
                text::body(fl!("show-settings-button")).width(Length::Fill),
                widget::toggler(self.config.show_settings_button)
                    .on_toggle(Message::SetShowSettings),
            ]
            .align_y(Alignment::Center)
            .height(Length::Fixed(48.0));

            let show_power_row = row![
                text::body(fl!("show-power-button")).width(Length::Fill),
                widget::toggler(self.config.show_power_button).on_toggle(Message::SetShowPower),
            ]
            .align_y(Alignment::Center)
            .height(Length::Fixed(48.0));

            let settings_body = column![
                position_row,
                default_page_row,
                columns_row,
                rows_row,
                show_settings_row,
                show_power_row
            ]
            .spacing(space_s)
            .padding([space_s, space_xxl]);

            column![
                settings_top_row,
                container(
                    container(settings_body)
                        .max_width(480.0)
                        .center_x(Length::Fill)
                )
                .height(Length::Fill)
            ]
            .align_x(Alignment::Center)
            .into()
        } else {
            column![
                top_row,
                app_scrollable,
                container(horizontal_rule(1))
                    .padding([space_none, space_xxl])
                    .width(Length::Fill),
                group_row
            ]
            .align_x(Alignment::Center)
            .into()
        };

        let window_width = self.window_width();
        let window_height = self.window_height();
        let window = container(content)
            .height(Length::Fixed(window_height))
            .max_height(window_height)
            .max_width(window_width)
            .class(theme::Container::Custom(Box::new(|theme| {
                let t = theme.cosmic();
                let radii = t.radius_s().map(|x| if x < 4.0 { x } else { x + 4.0 });

                container::Style {
                    text_color: Some(t.on_bg_color().into()),
                    icon_color: Some(t.on_bg_color().into()),
                    background: Some(Color::from(t.background(theme.transparent).base).into()),
                    border: Border {
                        radius: radii.into(),
                        width: 1.0,
                        color: t.bg_divider().into(),
                    },
                    shadow: Shadow::default(),
                    snap: true,
                }
            })))
            .center_x(Length::Fill)
            .width(Length::Fixed(window_width));
        // Closing the power menu is the popover's own `on_close`; an ancestor
        // mouse_area can't help here because the popover consumes the events
        // before they bubble this far.
        let window = mouse_area(window).on_press(Message::CloseContextMenu);
        let positioned = match self.effective_position() {
            LibraryPosition::Bottom => column!(
                space::vertical().height(Length::Fill),
                window,
                space::vertical().height(Length::Fixed(self.bottom_margin + 16.)),
            ),
            LibraryPosition::Center => column!(
                space::vertical().height(Length::Fill),
                window,
                space::vertical().height(Length::Fill),
            ),
            _ => column!(
                space::vertical().height(Length::Fixed(self.margin + 16.)),
                window,
            ),
        };
        stack![
            mouse_area(
                container(space::horizontal().width(Length::Fill))
                    .width(Length::Fill)
                    .height(Length::Fill)
            )
            .on_press(Message::Hide),
            positioned.align_x(Alignment::Center).width(Length::Fill)
        ]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch(vec![
            desktop_files(0).map(|_| Message::LoadApps),
            listen_with(|e, status, id| match e {
                cosmic::iced::Event::PlatformSpecific(PlatformSpecific::Wayland(
                    wayland::Event::Layer(e, _, id),
                )) => Some(Message::Layer(e, id)),
                cosmic::iced::Event::PlatformSpecific(PlatformSpecific::Wayland(
                    wayland::Event::OverlapNotify(event, ..),
                )) => Some(Message::Overlap(event)),
                cosmic::iced::Event::Keyboard(cosmic::iced::keyboard::Event::KeyReleased {
                    key: Key::Named(Named::Escape),
                    modifiers: _mods,
                    ..
                }) => Some(Message::EscapePressed),
                cosmic::iced::Event::Mouse(iced::mouse::Event::ButtonPressed(_))
                    if id == SurfaceId::RESERVED =>
                {
                    Some(Message::CloseContextMenu)
                }
                cosmic::iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
                    key,
                    text: _,
                    modifiers,
                    ..
                }) => match key {
                    Key::Character(c) if modifiers.control() && (c == "p" || c == "k") => {
                        Some(Message::PrevRow)
                    }
                    Key::Character(c) if modifiers.control() && (c == "n" || c == "j") => {
                        Some(Message::NextRow)
                    }
                    Key::Character(c) if modifiers.control() && (c == "f" || c == "l") => {
                        Some(Message::KeyboardNav(keyboard_nav::Action::FocusNext))
                    }
                    Key::Character(c) if modifiers.control() && (c == "b" || c == "h") => {
                        Some(Message::KeyboardNav(keyboard_nav::Action::FocusPrevious))
                    }
                    Key::Named(Named::ArrowUp)
                        if matches!(status, iced::event::Status::Ignored) =>
                    {
                        Some(Message::PrevRow)
                    }
                    Key::Named(Named::ArrowDown)
                        if matches!(status, iced::event::Status::Ignored) =>
                    {
                        Some(Message::NextRow)
                    }
                    Key::Named(Named::ArrowLeft)
                        if matches!(status, iced::event::Status::Ignored) =>
                    {
                        Some(Message::KeyboardNav(keyboard_nav::Action::FocusPrevious))
                    }
                    Key::Named(Named::ArrowRight)
                        if matches!(status, iced::event::Status::Ignored) =>
                    {
                        Some(Message::KeyboardNav(keyboard_nav::Action::FocusNext))
                    }
                    _ => None,
                },
                cosmic::iced::Event::Window(WindowEvent::Opened { position: _, size }) => {
                    Some(Message::Opened(size, id))
                }
                cosmic::iced::Event::PlatformSpecific(PlatformSpecific::Wayland(
                    wayland::Event::Output(event, _),
                )) => Some(Message::Output(event)),
                _ => None,
            }),
            keyboard_nav::subscription().map(Message::KeyboardNav),
            self.core
                .watch_config::<cosmic_app_list_config::AppListConfig>(
                    cosmic_app_list_config::APP_ID,
                )
                .map(|config| Message::AppListConfig(config.config)),
        ])
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(mut core: Core, flags: Args) -> (Self, iced::Task<cosmic::Action<Self::Message>>) {
        core.set_keyboard_nav(false);
        core.set_app_type(cosmic::core::AppType::System);

        let helper = AppLibraryConfig::helper();

        let config: AppLibraryConfig = helper
            .as_ref()
            .map(|helper| {
                AppLibraryConfig::get_entry(helper).unwrap_or_else(|(errors, config)| {
                    for err in errors {
                        error!("{:?}", err);
                    }
                    config
                })
            })
            .unwrap_or_default();
        let scrollable_id = Id::new("group-home");
        let group_count = config.groups.len() as u64;
        let group_keys: Vec<u64> = (0..group_count).collect();
        let mut self_ = Self {
            locale: std::env::var("LANG")
                .ok()
                .and_then(|l| l.split(".").next().map(str::to_string)),
            config,
            core,
            helper,
            last_hide: None,
            margin: 0.,
            overlap: HashMap::new(),
            size: Size::new(1920., 1080.),
            scrollable_id,
            group_keys,
            next_group_key: group_count,
            ..Default::default()
        };

        // Auto-activate when running in standalone mode
        let task = if matches!(flags.subcommand, Some(ApplicationsTasks::Run)) {
            Task::done(cosmic::Action::App(Message::Activate))
        } else {
            Task::none()
        };
        let dummy_task = self_.create_dummy_layer_surface();
        (self_, Task::batch([dummy_task, task]))
    }
}
