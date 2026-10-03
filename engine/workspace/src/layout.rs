//! Typed authoritative layout transactions; no disk or draft ownership.
use crate::model::*;
use std::collections::{HashMap, HashSet};

pub fn valid_path(p: &str) -> bool {
    !p.is_empty()
        && !p.starts_with('/')
        && !p.contains('\0')
        && !p.contains('\\')
        && p.split('/').all(|s| !s.is_empty() && s != "." && s != "..")
        && p != ".workspace"
        && (!p.starts_with(".workspace/")
            || matches!(p, ".workspace/config.json" | ".workspace/env.json")
            || service_path(p))
}
fn service_path(path: &str) -> bool {
    path.strip_prefix(".workspace/services/")
        .is_some_and(|tail| {
            tail.split('/').count() >= 2
                && tail.split('/').all(|p| {
                    !p.is_empty()
                        && p != "."
                        && p != ".."
                        && p.len() <= 160
                        && p.bytes()
                            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
                })
        })
}
pub fn valid_target(t: &Target) -> bool {
    match t.kind.as_str() {
        "file" | "image" | "diff" => t.path.as_deref().is_some_and(valid_path),
        "terminal" | "conversation" => t.id.as_deref().is_some_and(|id| !id.trim().is_empty()),
        "proxy" => matches!(t.page.as_deref(), Some("overview" | "logs" | "connections")),
        "settings" => true,
        _ => false,
    }
}
pub fn key(t: &Target) -> String {
    match t.kind.as_str() {
        "file" | "image" => format!("file:{}", t.path.as_deref().unwrap_or("")),
        "diff" => format!("diff:{}", t.path.as_deref().unwrap_or("")),
        "terminal" | "conversation" => format!("{}:{}", t.kind, t.id.as_deref().unwrap_or("")),
        "proxy" => format!("proxy:{}", t.page.as_deref().unwrap_or("")),
        _ => "settings".into(),
    }
}
pub fn resources(w: &Workbench) -> Vec<ResourceRef> {
    let mut out = vec![];
    for p in &w.panels {
        let r = match p.target.kind.as_str() {
            "file" | "image" | "diff" => ResourceRef::File {
                path: p.target.path.clone().unwrap_or_default(),
            },
            "conversation" => ResourceRef::Conversation {
                id: p.target.id.clone().unwrap_or_default(),
            },
            _ => continue,
        };
        if !out.contains(&r) {
            out.push(r)
        }
    }
    out
}
pub fn leaves(n: &Node) -> Vec<String> {
    match n {
        Node::Leaf(l) => vec![l.stack.clone()],
        Node::Split(s) => s.children.iter().flat_map(leaves).collect(),
    }
}
fn panel(w: &Workbench, id: &str) -> Option<usize> {
    w.panels.iter().position(|p| p.id == id)
}
fn stack(w: &Workbench, id: &str) -> Option<usize> {
    w.stacks.iter().position(|p| p.id == id)
}
fn home(w: &Workbench, id: &str) -> Option<String> {
    w.stacks
        .iter()
        .find(|s| s.panels.iter().any(|p| p == id))
        .map(|s| s.id.clone())
}
fn editor_stack(w: &Workbench, id: &str) -> bool {
    id != "bottom" && id != "aux" && stack(w, id).is_some()
}
fn new_id(w: &mut Workbench, prefix: &str) -> String {
    let mut seq = w.next_seq.max(1);
    // Preserve the established allocation rule, including references in unknown extension fields.
    let serialized = serde_json::to_string(w).expect("typed workbench serializes");
    loop {
        let id = format!("{prefix}{seq}");
        seq = seq.wrapping_add(1).max(1);
        if !serialized.contains(&format!("\"{id}\"")) {
            w.next_seq = seq;
            return id;
        }
    }
}
fn weights(ws: Vec<f64>) -> Vec<f64> {
    let n = ws.len();
    if n == 0 {
        return ws;
    }
    let floor = 0.05f64.min(1.0 / n as f64);
    if ws.iter().any(|x| !x.is_finite() || *x <= 0.0) {
        return vec![1.0 / n as f64; n];
    }
    let sum: f64 = ws.iter().sum();
    if (sum - 1.0).abs() < 1e-9 && ws.iter().all(|x| *x >= floor - 1e-9) {
        return ws;
    }
    let mut out: Vec<f64> = ws.iter().map(|x| x / sum).collect();
    for _ in 0..n {
        if out.iter().all(|x| *x >= floor) {
            break;
        }
        let free: Vec<usize> = (0..n).filter(|i| out[*i] > floor).collect();
        let free_sum: f64 = free.iter().map(|i| out[*i]).sum();
        let left = 1.0 - (n - free.len()) as f64 * floor;
        for (i, v) in out.iter_mut().enumerate() {
            *v = if free.contains(&i) && free_sum > 0.0 {
                *v * left / free_sum
            } else {
                floor
            }
        }
    }
    out
}
fn prune(
    n: &Node,
    keep: &HashSet<String>,
    seen: &mut HashSet<String>,
    ids: &mut HashSet<String>,
    seq: &mut u64,
) -> Option<Node> {
    let split = match n {
        Node::Leaf(l) => {
            return (keep.contains(&l.stack) && seen.insert(l.stack.clone())).then(|| n.clone());
        }
        Node::Split(s) => s,
    };
    let mut ws = split.weights.clone();
    if ws.len() != split.children.len() || ws.iter().any(|x| *x <= 0.0) {
        ws = vec![1.0; split.children.len()]
    }
    let axis = if split.axis == "column" {
        "column"
    } else {
        "row"
    };
    let mut cs = vec![];
    let mut out_ws = vec![];
    for (c, w) in split.children.iter().zip(ws) {
        if let Some(c) = prune(c, keep, seen, ids, seq) {
            match c {
                Node::Split(child) if child.axis == axis => {
                    ids.remove(&child.split);
                    for (gc, gw) in child.children.into_iter().zip(child.weights) {
                        cs.push(gc);
                        out_ws.push(w * gw)
                    }
                }
                other => {
                    cs.push(other);
                    out_ws.push(w)
                }
            }
        }
    }
    match cs.len() {
        0 => None,
        1 => cs.pop(),
        _ => {
            let mut id = split.split.clone();
            if id.is_empty() || !ids.insert(id.clone()) {
                loop {
                    id = format!("x{}", *seq);
                    *seq = seq.wrapping_add(1).max(1);
                    if ids.insert(id.clone()) {
                        break;
                    }
                }
            }
            Some(Node::Split(Split {
                split: id,
                axis: axis.into(),
                children: cs,
                weights: weights(out_ws),
                extra: split.extra.clone(),
            }))
        }
    }
}
pub fn normalize(w: &mut Workbench) {
    let input_panels = w.panels.clone();
    let all_stacks = w.stacks.clone();
    let mut els = leaves(&w.editor);
    let mut seen = HashSet::new();
    els.retain(|id| {
        id != "bottom"
            && id != "aux"
            && all_stacks.iter().any(|s| s.id == *id)
            && seen.insert(id.clone())
    });
    let mut order = els.clone();
    order.extend(["bottom".into(), "aux".into()]);
    let mut homes = HashMap::new();
    let mut in_order = vec![];
    for st in &order {
        if let Some(stk) = all_stacks.iter().find(|a| a.id == *st) {
            for id in &stk.panels {
                if input_panels
                    .iter()
                    .any(|p| p.id == *id && valid_target(&p.target))
                    && !homes.contains_key(id)
                {
                    homes.insert(id.clone(), st.clone());
                    in_order.push(id.clone())
                }
            }
        }
    }
    let mut rank = w.mru.clone();
    rank.extend(in_order.iter().filter(|id| !w.mru.contains(id)).cloned());
    let mut keys = HashSet::new();
    let mut kept = HashSet::new();
    for id in rank {
        if homes.contains_key(&id) {
            if let Some(p) = input_panels.iter().find(|p| p.id == id) {
                if keys.insert(key(&p.target)) {
                    kept.insert(id);
                }
            }
        }
    }
    w.panels = input_panels
        .into_iter()
        .filter(|p| kept.contains(&p.id))
        .collect();
    let mut seen = HashSet::new();
    w.mru
        .retain(|id| kept.contains(id) && seen.insert(id.clone()));
    let mru = w.mru.clone();
    let mut stacks = vec![];
    for st in &order {
        let old = all_stacks.iter().find(|a| a.id == *st);
        let mut seen = HashSet::new();
        let ps: Vec<String> = old
            .map(|o| o.panels.clone())
            .unwrap_or_default()
            .into_iter()
            .filter(|id| kept.contains(id) && homes.get(id) == Some(st) && seen.insert(id.clone()))
            .collect();
        let active = old
            .and_then(|o| o.active.as_ref())
            .filter(|a| ps.contains(a))
            .cloned()
            .or_else(|| {
                mru.iter()
                    .find(|p| ps.contains(p))
                    .cloned()
                    .or_else(|| ps.first().cloned())
            });
        stacks.push(Stack {
            id: st.clone(),
            panels: ps,
            active,
            extra: old.map(|o| o.extra.clone()).unwrap_or_default(),
        })
    }
    let mut promotion = w.chat.promoted_from.clone();
    if w.paradigm == "files"
        || promotion.as_ref().is_some_and(|p| {
            homes.get(&p.panel_id).map(String::as_str) != Some("aux") || p.stack_id == "aux"
        })
    {
        promotion = None
    }
    let mut keep: HashSet<String> = els
        .iter()
        .filter(|id| {
            stacks.iter().any(|a| a.id == **id && !a.panels.is_empty())
                || promotion.as_ref().is_some_and(|p| p.stack_id == **id)
        })
        .cloned()
        .collect();
    if keep.is_empty() {
        if els.contains(&w.last_editor_stack) {
            keep.insert(w.last_editor_stack.clone());
        } else if let Some(id) = els.first() {
            keep.insert(id.clone());
        }
    }
    let mut seq = w.next_seq.max(1);
    w.editor = prune(
        &w.editor,
        &keep,
        &mut HashSet::new(),
        &mut HashSet::new(),
        &mut seq,
    )
    .unwrap_or_else(|| {
        let id = loop {
            let id = format!("s{seq}");
            seq = seq.wrapping_add(1).max(1);
            if !all_stacks.iter().any(|x| x.id == id) {
                break id;
            }
        };
        stacks.push(Stack::empty(&id));
        Node::leaf(id)
    });
    els = leaves(&w.editor);
    stacks.retain(|st| els.contains(&st.id) || matches!(st.id.as_str(), "bottom" | "aux"));
    w.stacks = stacks;
    w.next_seq = seq;
    let recent = |allowed: &dyn Fn(&str) -> bool| {
        mru.iter()
            .filter_map(|id| home(w, id))
            .find(|st| allowed(st))
    };
    let last = if els.contains(&w.last_editor_stack) {
        w.last_editor_stack.clone()
    } else {
        recent(&|id| els.iter().any(|e| e == id)).unwrap_or_else(|| els[0].clone())
    };
    let focus = if stack(w, &w.files.focus).is_some() {
        w.files.focus.clone()
    } else {
        recent(&|_| true).unwrap_or(last.clone())
    };
    let side = if els.contains(&w.chat.side_stack) || w.chat.side_stack == "bottom" {
        w.chat.side_stack.clone()
    } else {
        last.clone()
    };
    let cf = if w.chat.focus == "aux" || w.chat.focus == side {
        w.chat.focus.clone()
    } else if stack(w, &w.chat.focus).is_some() {
        side.clone()
    } else {
        "aux".into()
    };
    w.last_editor_stack = last;
    w.files.focus = focus;
    w.chat.side_stack = side;
    w.chat.focus = cf;
    if w.files
        .maximized
        .as_deref()
        .is_none_or(|id| stack(w, id).is_none())
    {
        w.files.maximized = None
    }
    if promotion
        .as_ref()
        .is_some_and(|p| stack(w, &p.stack_id).is_none())
    {
        promotion = None
    }
    w.chat.promoted_from = promotion;
    if !matches!(w.paradigm.as_str(), "files" | "chat" | "solo") {
        w.paradigm = "files".into()
    }
    if w.solo
        .as_ref()
        .is_none_or(|s| panel(w, &s.panel_id).is_none())
    {
        if w.paradigm == "solo" {
            w.paradigm = if w.solo.as_ref().is_some_and(|s| s.return_to == "chat") {
                "chat"
            } else {
                "files"
            }
            .into()
        }
        w.solo = None
    }
    if w.paradigm != "solo" {
        w.solo = None
    } else if let Some(s) = &mut w.solo {
        if !matches!(s.return_to.as_str(), "files" | "chat") {
            s.return_to = "files".into()
        }
    }
    for (r, default, min, max) in [
        (&mut w.files.explorer, 264.0, 200.0, 1600.0),
        (&mut w.files.aux, 360.0, 300.0, 1600.0),
        (&mut w.files.bottom, 0.35, 0.1, 0.9),
        (&mut w.chat.rail, 264.0, 200.0, 1600.0),
        (&mut w.chat.side, 400.0, 300.0, 1600.0),
        (&mut w.chat.tree, 200.0, 160.0, 1600.0),
    ] {
        r.size = if r.size.is_finite() {
            r.size.clamp(min, max)
        } else {
            default
        }
    }
    let mut seen = HashSet::new();
    w.explorer
        .expanded
        .retain(|p| valid_path(p) && seen.insert(p.clone()));
    if w.explorer
        .selected
        .as_deref()
        .is_none_or(|p| !valid_path(p))
    {
        w.explorer.selected = None
    }
}
fn remove(w: &mut Workbench, id: &str) {
    if let Some(st) = home(w, id) {
        let i = stack(w, &st).unwrap();
        let at = w.stacks[i].panels.iter().position(|p| p == id).unwrap();
        w.stacks[i].panels.retain(|p| p != id);
        if w.stacks[i].active.as_deref() == Some(id) {
            w.stacks[i].active = w
                .mru
                .iter()
                .find(|p| w.stacks[i].panels.contains(p))
                .cloned()
                .or_else(|| {
                    w.stacks[i]
                        .panels
                        .get(at.min(w.stacks[i].panels.len().saturating_sub(1)))
                        .cloned()
                })
        }
    }
}
fn insert(w: &mut Workbench, st: &str, id: &str, index: Option<usize>) {
    if let Some(i) = stack(w, st) {
        let s = &mut w.stacks[i];
        let pos = index
            .unwrap_or_else(|| {
                s.panels
                    .iter()
                    .position(|p| Some(p) == s.active.as_ref())
                    .map(|i| i + 1)
                    .unwrap_or(s.panels.len())
            })
            .min(s.panels.len());
        s.panels.insert(pos, id.into());
        s.active = Some(id.into())
    }
}
fn restore_promotion(w: &mut Workbench) {
    if let Some(o) = w.chat.promoted_from.take() {
        if home(w, &o.panel_id).as_deref() == Some("aux")
            && o.stack_id != "aux"
            && stack(w, &o.stack_id).is_some()
        {
            remove(w, &o.panel_id);
            if let Some(i) = stack(w, "aux") {
                if o.aux_active_before
                    .as_ref()
                    .is_some_and(|id| w.stacks[i].panels.contains(id))
                {
                    w.stacks[i].active = o.aux_active_before
                }
            }
            insert(w, &o.stack_id, &o.panel_id, Some(o.index))
        }
    }
}
fn enter_chat(w: &mut Workbench) {
    let last = w.last_editor_stack.clone();
    let has = stack(w, &last).is_some_and(|i| !w.stacks[i].panels.is_empty());
    let side = if has {
        last
    } else {
        w.mru
            .iter()
            .filter_map(|id| home(w, id))
            .find(|st| editor_stack(w, st))
            .unwrap_or(last)
    };
    w.paradigm = "chat".into();
    w.chat.side_stack = side;
    w.chat.focus = "aux".into()
}
fn leave_solo(w: &mut Workbench, to: Option<&str>) {
    let dest = to.unwrap_or(if w.solo.as_ref().is_some_and(|s| s.return_to == "chat") {
        "chat"
    } else {
        "files"
    });
    w.paradigm = dest.into();
    w.solo = None;
    if dest == "chat" {
        enter_chat(w)
    } else {
        restore_promotion(w)
    }
}
fn focus_stack(w: &mut Workbench, st: &str) {
    if stack(w, st).is_none() {
        return;
    }
    if editor_stack(w, st) {
        w.last_editor_stack = st.into()
    }
    if w.paradigm == "chat" {
        w.chat.focus = st.into();
        if st != "aux" {
            w.chat.side_stack = st.into();
            w.chat.side.collapsed = false
        }
    } else {
        w.files.focus = st.into();
        match st {
            "bottom" => w.files.bottom.collapsed = false,
            "aux" => w.files.aux.collapsed = false,
            _ => {}
        }
        if w.files.maximized.as_deref() != Some(st) {
            w.files.maximized = None
        }
    }
}
fn reveal(w: &mut Workbench, id: &str) {
    if w.paradigm == "solo" && w.solo.as_ref().is_none_or(|s| s.panel_id != id) {
        leave_solo(w, None)
    }
    if let Some(st) = home(w, id) {
        let i = stack(w, &st).unwrap();
        w.stacks[i].active = Some(id.into());
        w.mru.retain(|p| p != id);
        w.mru.insert(0, id.into());
        focus_stack(w, &st)
    }
}
fn edge_info(edge: &str) -> (&str, bool) {
    (
        if matches!(edge, "top" | "bottom") {
            "column"
        } else {
            "row"
        },
        matches!(edge, "left" | "top"),
    )
}
fn at_edge(n: &mut Node, target: Option<&str>, new: &str, edge: &str, split: &str) {
    let (axis, before) = edge_info(edge);
    let fresh = Node::leaf(new);
    if target.is_none() {
        if let Node::Split(s) = n {
            if s.axis == axis {
                let count = s.children.len();
                for w in &mut s.weights {
                    *w *= count as f64 / (count + 1) as f64
                }
                let pos = if before { 0 } else { count };
                s.children.insert(pos, fresh);
                s.weights.insert(pos, 1.0 / (count + 1) as f64);
                return;
            }
        }
    }
    let is_target = match n {
        Node::Leaf(l) => Some(l.stack.as_str()) == target,
        _ => false,
    };
    if target.is_none() || is_target {
        let old = n.clone();
        *n = Node::Split(Split {
            split: split.into(),
            axis: axis.into(),
            children: if before {
                vec![fresh, old]
            } else {
                vec![old, fresh]
            },
            weights: vec![0.5, 0.5],
            extra: Default::default(),
        });
        return;
    }
    if let Node::Split(s) = n {
        if s.axis == axis {
            if let Some(i) = s
                .children
                .iter()
                .position(|c| matches!(c,Node::Leaf(l)if Some(l.stack.as_str())==target))
            {
                let half = s.weights[i] / 2.0;
                s.weights[i] = half;
                let pos = if before { i } else { i + 1 };
                s.children.insert(pos, fresh);
                s.weights.insert(pos, half);
                return;
            }
        }
        for c in &mut s.children {
            at_edge(c, target, new, edge, split)
        }
    }
}
fn open(w: &mut Workbench, t: &Target, p: &Placement, focus: bool) {
    if !valid_target(t) {
        return;
    }
    if w.paradigm == "solo" {
        let same = w
            .solo
            .as_ref()
            .and_then(|s| panel(w, &s.panel_id))
            .is_some_and(|i| key(&w.panels[i].target) == key(t));
        if !same {
            leave_solo(w, None)
        }
    }
    if let Some(id) = w
        .panels
        .iter()
        .find(|a| key(&a.target) == key(t))
        .map(|a| a.id.clone())
    {
        if let Placement::InStack {
            stack_id,
            index,
            move_existing: true,
            ..
        } = p
        {
            if stack(w, stack_id).is_some() && home(w, &id).as_deref() != Some(stack_id) {
                remove(w, &id);
                insert(w, stack_id, &id, *index)
            }
        }
        if focus {
            reveal(w, &id)
        }
        return;
    }
    let requested = match p {
        Placement::Auto => None,
        Placement::InStack { stack_id, .. } | Placement::SplitEdge { stack_id, .. } => {
            Some(stack_id.as_str())
        }
    };
    let special = requested.is_some_and(|st| stack(w, st).is_some());
    let st = if special {
        requested.unwrap().to_owned()
    } else {
        match t.kind.as_str() {
            "terminal" => "bottom".into(),
            "conversation" => "aux".into(),
            _ => {
                let candidate = if w.paradigm == "chat" {
                    &w.chat.side_stack
                } else {
                    &w.files.focus
                };
                if editor_stack(w, candidate) {
                    candidate.clone()
                } else {
                    w.last_editor_stack.clone()
                }
            }
        }
    };
    if special
        && matches!(
            p,
            Placement::InStack {
                replace_active: true,
                ..
            }
        )
    {
        let i = stack(w, &st).unwrap();
        if let Some(active) = w.stacks[i].active.clone() {
            if let Some(i) = panel(w, &active) {
                if w.panels[i].target.kind == t.kind {
                    w.panels[i].target = t.clone();
                    w.panels[i].view = PanelView::default();
                    if focus {
                        reveal(w, &active)
                    }
                    return;
                }
            }
        }
    }
    let id = new_id(w, "p");
    w.panels.push(Panel {
        id: id.clone(),
        target: t.clone(),
        view: Default::default(),
        extra: Default::default(),
    });
    if let Placement::SplitEdge { edge, .. } = p
        && special
        && editor_stack(w, &st)
    {
        let fresh = new_id(w, "s");
        let split = new_id(w, "x");
        at_edge(&mut w.editor, Some(&st), &fresh, edge, &split);
        w.stacks.push(Stack::with_panel(fresh, id.clone()))
    } else {
        let i = stack(w, &st).unwrap();
        let old = w.stacks[i].active.clone();
        insert(
            w,
            &st,
            &id,
            if special {
                match p {
                    Placement::InStack { index, .. } => *index,
                    _ => None,
                }
            } else {
                None
            },
        );
        if !focus && old.is_some() {
            w.stacks[i].active = old
        }
    }
    if focus {
        reveal(w, &id)
    }
}
fn after_bottom(w: &mut Workbench, had: bool) {
    if had && stack(w, "bottom").is_some_and(|i| w.stacks[i].panels.is_empty()) {
        w.files.bottom.collapsed = true;
        if w.files.focus == "bottom" {
            w.files.focus = w.last_editor_stack.clone()
        }
        if w.files.maximized.as_deref() == Some("bottom") {
            w.files.maximized = None
        }
        if w.chat.side_stack == "bottom" {
            w.chat.side_stack = w.last_editor_stack.clone();
            if w.chat.focus == "bottom" {
                w.chat.focus = w.last_editor_stack.clone()
            }
        }
    }
}
fn move_panel(w: &mut Workbench, id: &str, to: &DropTarget) {
    let Some(source) = home(w, id) else { return };
    let st = match to {
        DropTarget::Center { stack_id }
        | DropTarget::Tab { stack_id, .. }
        | DropTarget::Edge { stack_id, .. } => Some(stack_id.as_str()),
        DropTarget::EditorEdge { .. } => None,
    };
    if st.is_some_and(|st| stack(w, st).is_none()) {
        return;
    }
    let fallback = st
        .filter(|st| matches!(to, DropTarget::Edge { .. }) && !editor_stack(w, st))
        .map(|st| DropTarget::Center {
            stack_id: st.into(),
        });
    let to = fallback.as_ref().unwrap_or(to);
    let had = stack(w, "bottom").is_some_and(|i| !w.stacks[i].panels.is_empty());
    match to {
        DropTarget::Center { stack_id } => {
            if source == *stack_id {
                return;
            }
            remove(w, id);
            insert(w, stack_id, id, Some(usize::MAX))
        }
        DropTarget::Tab { stack_id, index } => {
            let i = stack(w, stack_id).unwrap();
            let mut pos = (*index).min(w.stacks[i].panels.len());
            if source == *stack_id {
                let from = w.stacks[i].panels.iter().position(|p| p == id).unwrap();
                if pos > from {
                    pos -= 1
                }
            }
            remove(w, id);
            insert(w, stack_id, id, Some(pos))
        }
        DropTarget::Edge { edge, .. } | DropTarget::EditorEdge { edge } => {
            let i = stack(w, &source).unwrap();
            let only = w.stacks[i].panels.len() == 1;
            if only
                && ((st == Some(&source))
                    || (st.is_none() && leaves(&w.editor) == vec![source.clone()]))
            {
                return;
            }
            let fresh = new_id(w, "s");
            let split = new_id(w, "x");
            remove(w, id);
            w.stacks.push(Stack::with_panel(fresh.clone(), id.into()));
            at_edge(&mut w.editor, st, &fresh, edge, &split)
        }
    }
    reveal(w, id);
    after_bottom(w, had)
}
fn resize(n: &mut Node, id: &str, ws: Option<&[f64]>) {
    if let Node::Split(s) = n {
        if s.split == id {
            let next = ws
                .map(<[f64]>::to_vec)
                .unwrap_or(vec![1.0; s.children.len()]);
            if next.len() == s.children.len() && next.iter().all(|x| *x > 0.0 && x.is_finite()) {
                s.weights = weights(next)
            }
        } else {
            for c in &mut s.children {
                resize(c, id, ws)
            }
        }
    }
}
pub fn rebase(p: &str, from: &str, to: &str) -> String {
    if p == from {
        to.into()
    } else if p.starts_with(&format!("{from}/")) {
        format!("{to}{}", &p[from.len()..])
    } else {
        p.into()
    }
}
fn region<'a>(w: &'a mut Workbench, name: &str) -> Option<&'a mut RegionState> {
    match name {
        "explorer" => Some(&mut w.files.explorer),
        "aux" => Some(&mut w.files.aux),
        "bottom" => Some(&mut w.files.bottom),
        "chat_rail" => Some(&mut w.chat.rail),
        "chat_side" => Some(&mut w.chat.side),
        "chat_tree" => Some(&mut w.chat.tree),
        _ => None,
    }
}
pub fn apply(input: &Workbench, op: &LayoutAction) -> Workbench {
    let mut w = input.clone();
    normalize(&mut w);
    match op {
        LayoutAction::Open {
            target,
            placement,
            focus,
        } => open(&mut w, target, placement, *focus),
        LayoutAction::Focus { panel_id } => {
            if panel(&w, panel_id).is_some() {
                reveal(&mut w, panel_id)
            }
        }
        LayoutAction::FocusStack { stack_id } => {
            if let Some(i) = stack(&w, stack_id) {
                if let Some(active) = w.stacks[i].active.clone() {
                    reveal(&mut w, &active)
                } else {
                    if w.paradigm == "solo" {
                        leave_solo(&mut w, None)
                    }
                    focus_stack(&mut w, stack_id)
                }
            }
        }
        LayoutAction::Close { panel_ids } => {
            let had = stack(&w, "bottom").is_some_and(|i| !w.stacks[i].panels.is_empty());
            for id in panel_ids {
                remove(&mut w, id)
            }
            w.panels.retain(|p| !panel_ids.contains(&p.id));
            w.mru.retain(|p| !panel_ids.contains(p));
            if w.solo
                .as_ref()
                .is_some_and(|s| panel_ids.contains(&s.panel_id))
            {
                leave_solo(&mut w, None)
            }
            after_bottom(&mut w, had)
        }
        LayoutAction::Move { panel_id, to } => move_panel(&mut w, panel_id, to),
        LayoutAction::SplitStack { stack_id, edge } => {
            if editor_stack(&w, stack_id) {
                let i = stack(&w, stack_id).unwrap();
                if w.stacks[i].panels.len() > 1 {
                    if let Some(id) = w.stacks[i].active.clone() {
                        move_panel(
                            &mut w,
                            &id,
                            &DropTarget::Edge {
                                stack_id: stack_id.clone(),
                                edge: edge.clone(),
                            },
                        )
                    }
                }
            }
        }
        LayoutAction::ResizeSplit { split_id, weights } => {
            resize(&mut w.editor, split_id, Some(weights))
        }
        LayoutAction::ResetSplit { split_id } => resize(&mut w.editor, split_id, None),
        LayoutAction::ResizeRegion { region: name, size } => {
            if let Some(r) = region(&mut w, name) {
                r.size = *size
            }
        }
        LayoutAction::SetRegionCollapsed {
            region: name,
            collapsed,
        } => {
            if let Some(r) = region(&mut w, name) {
                r.collapsed = *collapsed
            }
        }
        LayoutAction::ToggleRegion { region: name } => {
            if let Some(r) = region(&mut w, name) {
                r.collapsed = !r.collapsed
            }
        }
        LayoutAction::SetMaximized { stack_id } => {
            if let Some(st) = stack_id {
                if let Some(i) = stack(&w, st) {
                    if w.paradigm == "files" {
                        if let Some(active) = w.stacks[i].active.clone() {
                            reveal(&mut w, &active)
                        } else {
                            focus_stack(&mut w, st)
                        }
                    }
                    w.files.maximized = Some(st.clone())
                }
            } else {
                w.files.maximized = None
            }
        }
        LayoutAction::SwitchParadigm { paradigm } => match paradigm.as_str() {
            "files" => {
                if w.paradigm == "solo" {
                    leave_solo(&mut w, Some("files"))
                } else {
                    w.paradigm = "files".into();
                    restore_promotion(&mut w)
                }
            }
            "chat" => {
                if w.paradigm != "chat" {
                    if w.paradigm == "solo" {
                        leave_solo(&mut w, Some("files"))
                    }
                    enter_chat(&mut w)
                }
            }
            _ => {}
        },
        LayoutAction::PromoteConversation { panel_id: id } => {
            if panel(&w, id).is_some_and(|i| w.panels[i].target.kind == "conversation") {
                if w.paradigm == "solo" {
                    leave_solo(&mut w, None)
                }
                if let Some(source) = home(&w, id) {
                    if source != "aux" {
                        let i = stack(&w, &source).unwrap();
                        let idx = w.stacks[i].panels.iter().position(|p| p == id).unwrap();
                        let aux = stack(&w, "aux").unwrap();
                        let origin = PanelOrigin {
                            panel_id: id.clone(),
                            stack_id: source,
                            index: idx,
                            aux_active_before: w.stacks[aux].active.clone(),
                            extra: Default::default(),
                        };
                        remove(&mut w, id);
                        insert(&mut w, "aux", id, None);
                        w.chat.promoted_from = Some(origin)
                    }
                    if w.paradigm != "chat" {
                        enter_chat(&mut w)
                    }
                    reveal(&mut w, id)
                }
            }
        }
        LayoutAction::ReturnToFiles => {
            if w.paradigm == "solo" {
                leave_solo(&mut w, Some("files"))
            } else {
                w.paradigm = "files".into();
                restore_promotion(&mut w)
            }
        }
        LayoutAction::EnterSolo { target } => {
            if valid_target(target) {
                let ret = if w.paradigm == "solo" {
                    w.solo
                        .as_ref()
                        .map(|s| s.return_to.clone())
                        .unwrap_or_else(|| "files".into())
                } else {
                    w.paradigm.clone()
                };
                w.paradigm = ret.clone();
                w.solo = None;
                open(&mut w, target, &Placement::Auto, true);
                let id = w
                    .panels
                    .iter()
                    .find(|p| key(&p.target) == key(target))
                    .unwrap()
                    .id
                    .clone();
                w.paradigm = "solo".into();
                w.solo = Some(SoloArrangement {
                    panel_id: id,
                    return_to: ret,
                    extra: Default::default(),
                })
            }
        }
        LayoutAction::SetChatSideStack { stack_id: st } => {
            if editor_stack(&w, st) || st == "bottom" {
                if editor_stack(&w, st) {
                    w.last_editor_stack = st.clone()
                }
                w.chat.side_stack = st.clone();
                if w.paradigm == "chat" {
                    w.chat.focus = st.clone()
                }
                w.chat.side.collapsed = false
            }
        }
        LayoutAction::Retarget {
            panel_id: id,
            target,
        } => {
            if let Some(i) = panel(&w, id) {
                if valid_target(target) {
                    if let Some(other) = w
                        .panels
                        .iter()
                        .find(|p| key(&p.target) == key(target))
                        .map(|p| p.id.clone())
                    {
                        if other == *id {
                            w.panels[i].target = target.clone()
                        }
                        reveal(&mut w, &other)
                    } else {
                        w.panels[i].target = target.clone();
                        w.panels[i].view = Default::default();
                        reveal(&mut w, id)
                    }
                }
            }
        }
        LayoutAction::UpdateView { panel_id, view } => {
            if let Some(i) = panel(&w, panel_id) {
                w.panels[i].view = view.clone()
            }
        }
        LayoutAction::UpdateExplorer { explorer } => w.explorer = explorer.clone(),
        LayoutAction::RenamePath { from, to } => {
            if valid_path(from) && valid_path(to) {
                let mut renamed = vec![];
                for p in &mut w.panels {
                    if matches!(p.target.kind.as_str(), "file" | "image" | "diff") {
                        if let Some(path) = &mut p.target.path {
                            let new = rebase(path, from, to);
                            if new != *path {
                                renamed.push(p.id.clone());
                                *path = new
                            }
                        }
                    }
                }
                let unique: HashSet<_> = w.panels.iter().map(|p| key(&p.target)).collect();
                if unique.len() < w.panels.len() {
                    for id in &w.mru {
                        if !renamed.contains(id) {
                            renamed.push(id.clone())
                        }
                    }
                    w.mru = renamed
                }
                w.explorer.expanded = w
                    .explorer
                    .expanded
                    .iter()
                    .map(|p| rebase(p, from, to))
                    .collect();
                if let Some(p) = &mut w.explorer.selected {
                    *p = rebase(p, from, to)
                }
            }
        }
    }
    normalize(&mut w);
    w
}
