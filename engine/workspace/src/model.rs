use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use workflow_environment::json::OpaqueObject;

/// A panel addresses an Engine resource. Unknown fields are retained on every persisted object.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Target {
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<String>,
    #[serde(default, flatten)]
    pub extra: OpaqueObject,
}
impl Target {
    pub fn file(path: impl Into<String>) -> Self {
        Self {
            kind: "file".into(),
            path: Some(path.into()),
            id: None,
            page: None,
            extra: Default::default(),
        }
    }
    pub fn conversation(id: impl Into<String>) -> Self {
        Self {
            kind: "conversation".into(),
            path: None,
            id: Some(id.into()),
            page: None,
            extra: Default::default(),
        }
    }
    pub fn terminal(id: impl Into<String>) -> Self {
        Self {
            kind: "terminal".into(),
            path: None,
            id: Some(id.into()),
            page: None,
            extra: Default::default(),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum ResourceRef {
    File { path: String },
    Conversation { id: String },
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PanelView {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scroll_anchor: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scroll_offset: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<[i64; 4]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extras: Option<BTreeMap<String, String>>,
    #[serde(default, flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Panel {
    pub id: String,
    pub target: Target,
    #[serde(default)]
    pub view: PanelView,
    #[serde(default, flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Stack {
    pub id: String,
    #[serde(default)]
    pub panels: Vec<String>,
    #[serde(default)]
    pub active: Option<String>,
    #[serde(default, flatten)]
    pub extra: OpaqueObject,
}
impl Stack {
    pub fn empty(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            panels: vec![],
            active: None,
            extra: Default::default(),
        }
    }
    pub fn with_panel(id: String, panel: String) -> Self {
        Self {
            id,
            panels: vec![panel.clone()],
            active: Some(panel),
            extra: Default::default(),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, JsonSchema)]
#[serde(untagged)]
pub enum Node {
    Leaf(Leaf),
    Split(Split),
}
impl<'de> Deserialize<'de> for Node {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Shape {
            stack: Option<String>,
            split: Option<String>,
            axis: Option<String>,
            children: Option<Vec<Node>>,
            weights: Option<Vec<f64>>,
            #[serde(flatten)]
            extra: OpaqueObject,
        }
        let s = Shape::deserialize(d)?;
        if let Some(stack) = s.stack {
            Ok(Self::Leaf(Leaf {
                stack,
                extra: s.extra,
            }))
        } else {
            Ok(Self::Split(Split {
                split: s.split.unwrap_or_default(),
                axis: s.axis.unwrap_or_default(),
                children: s.children.unwrap_or_default(),
                weights: s.weights.unwrap_or_default(),
                extra: s.extra,
            }))
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Leaf {
    pub stack: String,
    #[serde(default, flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Split {
    #[serde(default)]
    pub split: String,
    #[serde(default)]
    pub axis: String,
    #[serde(default)]
    pub children: Vec<Node>,
    #[serde(default)]
    pub weights: Vec<f64>,
    #[serde(default, flatten)]
    pub extra: OpaqueObject,
}
impl Node {
    pub fn leaf(stack: impl Into<String>) -> Self {
        Self::Leaf(Leaf {
            stack: stack.into(),
            extra: Default::default(),
        })
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct RegionState {
    pub size: f64,
    #[serde(default)]
    pub collapsed: bool,
    #[serde(default, flatten)]
    pub extra: OpaqueObject,
}
impl RegionState {
    fn new(size: f64, collapsed: bool) -> Self {
        Self {
            size,
            collapsed,
            extra: Default::default(),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct FilesArrangement {
    pub explorer: RegionState,
    pub aux: RegionState,
    pub bottom: RegionState,
    pub focus: String,
    pub maximized: Option<String>,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
impl Default for FilesArrangement {
    fn default() -> Self {
        Self {
            explorer: RegionState::new(264.0, false),
            aux: RegionState::new(360.0, false),
            bottom: RegionState::new(0.35, true),
            focus: "s0".into(),
            maximized: None,
            extra: Default::default(),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct ChatArrangement {
    pub rail: RegionState,
    pub side: RegionState,
    pub tree: RegionState,
    pub side_stack: String,
    pub focus: String,
    pub promoted_from: Option<PanelOrigin>,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
impl Default for ChatArrangement {
    fn default() -> Self {
        Self {
            rail: RegionState::new(264.0, false),
            side: RegionState::new(400.0, false),
            tree: RegionState::new(200.0, false),
            side_stack: "s0".into(),
            focus: "aux".into(),
            promoted_from: None,
            extra: Default::default(),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PanelOrigin {
    pub panel_id: String,
    pub stack_id: String,
    pub index: usize,
    pub aux_active_before: Option<String>,
    #[serde(default, flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SoloArrangement {
    pub panel_id: String,
    pub return_to: String,
    #[serde(default, flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct ExplorerView {
    pub expanded: Vec<String>,
    pub selected: Option<String>,
    pub show_hidden: bool,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct Workbench {
    pub paradigm: String,
    pub panels: Vec<Panel>,
    pub stacks: Vec<Stack>,
    pub editor: Node,
    pub files: FilesArrangement,
    pub chat: ChatArrangement,
    pub solo: Option<SoloArrangement>,
    pub last_editor_stack: String,
    pub explorer: ExplorerView,
    pub mru: Vec<String>,
    pub next_seq: u64,
    #[serde(flatten)]
    pub extra: OpaqueObject,
}
impl Default for Workbench {
    fn default() -> Self {
        Self {
            paradigm: "files".into(),
            panels: vec![],
            stacks: vec![
                Stack::empty("s0"),
                Stack::empty("bottom"),
                Stack::empty("aux"),
            ],
            editor: Node::leaf("s0"),
            files: Default::default(),
            chat: Default::default(),
            solo: None,
            last_editor_stack: "s0".into(),
            explorer: Default::default(),
            mru: vec![],
            next_seq: 1,
            extra: Default::default(),
        }
    }
}
fn yes() -> bool {
    true
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Placement {
    #[default]
    Auto,
    InStack {
        stack_id: String,
        #[serde(default)]
        index: Option<usize>,
        #[serde(default)]
        move_existing: bool,
        #[serde(default)]
        replace_active: bool,
    },
    SplitEdge {
        stack_id: String,
        edge: String,
    },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum DropTarget {
    Center { stack_id: String },
    Tab { stack_id: String, index: usize },
    Edge { stack_id: String, edge: String },
    EditorEdge { edge: String },
}
#[derive(Clone, Debug, PartialEq, Serialize, JsonSchema)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum LayoutAction {
    Open {
        target: Target,
        #[serde(default)]
        placement: Placement,
        #[serde(default = "yes")]
        focus: bool,
    },
    Focus {
        panel_id: String,
    },
    FocusStack {
        stack_id: String,
    },
    Close {
        panel_ids: Vec<String>,
    },
    Move {
        panel_id: String,
        to: DropTarget,
    },
    SplitStack {
        stack_id: String,
        edge: String,
    },
    ResizeSplit {
        split_id: String,
        weights: Vec<f64>,
    },
    ResetSplit {
        split_id: String,
    },
    ResizeRegion {
        region: String,
        size: f64,
    },
    SetRegionCollapsed {
        region: String,
        collapsed: bool,
    },
    ToggleRegion {
        region: String,
    },
    SetMaximized {
        stack_id: Option<String>,
    },
    SwitchParadigm {
        paradigm: String,
    },
    PromoteConversation {
        panel_id: String,
    },
    ReturnToFiles,
    EnterSolo {
        target: Target,
    },
    SetChatSideStack {
        stack_id: String,
    },
    Retarget {
        panel_id: String,
        target: Target,
    },
    UpdateView {
        panel_id: String,
        view: PanelView,
    },
    UpdateExplorer {
        explorer: ExplorerView,
    },
    RenamePath {
        from: String,
        to: String,
    },
}

impl<'de> Deserialize<'de> for LayoutAction {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Header {
            #[serde(rename = "type")]
            kind: String,
            #[serde(flatten)]
            fields: OpaqueObject,
        }
        let header = Header::deserialize(d)?;
        let bytes = serde_json::to_vec(&header.fields).map_err(serde::de::Error::custom)?;
        macro_rules! action {($variant:ident{$($(#[$meta:meta])* $field:ident:$kind:ty),*$(,)?})=>{{
            #[derive(Deserialize)]#[serde(rename_all="camelCase")]struct Args{$($(#[$meta])* $field:$kind),*}
            let a:Args=workflow_environment::json::strict_json(&bytes).map_err(serde::de::Error::custom)?;
            Self::$variant{$($field:a.$field),*}
        }}}
        Ok(match header.kind.as_str() {
            "open" => action!(Open {
                target: Target,
                #[serde(default)]
                placement: Placement,
                #[serde(default = "yes")]
                focus: bool
            }),
            "focus" => action!(Focus { panel_id: String }),
            "focusStack" => action!(FocusStack { stack_id: String }),
            "close" => action!(Close{panel_ids:Vec<String>}),
            "move" => action!(Move {
                panel_id: String,
                to: DropTarget
            }),
            "splitStack" => action!(SplitStack {
                stack_id: String,
                edge: String
            }),
            "resizeSplit" => action!(ResizeSplit{split_id:String,weights:Vec<f64>}),
            "resetSplit" => action!(ResetSplit { split_id: String }),
            "resizeRegion" => action!(ResizeRegion {
                region: String,
                size: f64
            }),
            "setRegionCollapsed" => action!(SetRegionCollapsed {
                region: String,
                collapsed: bool
            }),
            "toggleRegion" => action!(ToggleRegion { region: String }),
            "setMaximized" => action!(SetMaximized{stack_id:Option<String>}),
            "switchParadigm" => action!(SwitchParadigm { paradigm: String }),
            "promoteConversation" => action!(PromoteConversation { panel_id: String }),
            "returnToFiles" => Self::ReturnToFiles,
            "enterSolo" => action!(EnterSolo { target: Target }),
            "setChatSideStack" => action!(SetChatSideStack { stack_id: String }),
            "retarget" => action!(Retarget {
                panel_id: String,
                target: Target
            }),
            "updateView" => action!(UpdateView {
                panel_id: String,
                view: PanelView
            }),
            "updateExplorer" => action!(UpdateExplorer {
                explorer: ExplorerView
            }),
            "renamePath" => action!(RenamePath {
                from: String,
                to: String
            }),
            _ => return Err(serde::de::Error::custom("unknown layout operation")),
        })
    }
}
