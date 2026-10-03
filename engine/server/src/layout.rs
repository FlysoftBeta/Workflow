//! Authoritative layout transactions. Wire objects match StateCodec's current format.
use serde_json::{Value as V, json};
use std::collections::{HashMap, HashSet};
pub fn s(v: &V) -> &str {
    v.as_str().unwrap_or("")
}
fn strings(v: &V) -> Vec<String> {
    v.as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}
fn arr(v: &V) -> Vec<V> {
    v.as_array().cloned().unwrap_or_default()
}
pub fn empty() -> V {
    json!({"paradigm":"files","panels":[],"stacks":[{"id":"s0","panels":[],"active":null},{"id":"bottom","panels":[],"active":null},{"id":"aux","panels":[],"active":null}],"editor":{"stack":"s0"},"files":{"explorer":{"size":264.0,"collapsed":false},"aux":{"size":360.0,"collapsed":false},"bottom":{"size":0.35,"collapsed":true},"focus":"s0","maximized":null},"chat":{"rail":{"size":264.0,"collapsed":false},"side":{"size":400.0,"collapsed":false},"tree":{"size":200.0,"collapsed":false},"sideStack":"s0","focus":"aux","promotedFrom":null},"solo":null,"lastEditorStack":"s0","explorer":{"expanded":[],"selected":null,"showHidden":false},"mru":[],"nextSeq":1})
}
pub fn valid_path(p: &str) -> bool {
    !p.is_empty()
        && !p.starts_with('/')
        && !p.contains('\0')
        && !p.contains('\\')
        && p.split('/').all(|s| !s.is_empty() && s != "." && s != "..")
        && (!p.starts_with(".workspace/")
            || p == ".workspace/config.json"
            || p == ".workspace/env.json"
            || service_path(p))
        && p != ".workspace"
}
fn service_path(path: &str) -> bool {
    let Some(tail) = path.strip_prefix(".workspace/services/") else {
        return false;
    };
    let parts: Vec<_> = tail.split('/').collect();
    parts.len() >= 2 && parts.iter().all(|p| crate::storage::identifier(p).is_ok())
}
pub fn valid_target(t: &V) -> bool {
    match s(&t["kind"]) {
        "file" | "image" | "diff" => valid_path(s(&t["path"])),
        "terminal" | "conversation" => !s(&t["id"]).trim().is_empty(),
        "proxy" => matches!(s(&t["page"]), "overview" | "logs" | "connections"),
        "settings" => true,
        _ => false,
    }
}
pub fn key(t: &V) -> String {
    match s(&t["kind"]) {
        "file" | "image" => format!("file:{}", s(&t["path"])),
        "diff" => format!("diff:{}", s(&t["path"])),
        "terminal" | "conversation" => format!("{}:{}", s(&t["kind"]), s(&t["id"])),
        "proxy" => format!("proxy:{}", s(&t["page"])),
        _ => "settings".into(),
    }
}
pub fn resources(w: &V) -> Vec<V> {
    let mut out = vec![];
    for p in arr(&w["panels"]) {
        let t = &p["target"];
        let r = match s(&t["kind"]) {
            "file" | "image" | "diff" => json!({"kind":"file","path":t["path"]}),
            "conversation" => json!({"kind":"conversation","id":t["id"]}),
            _ => continue,
        };
        if !out.contains(&r) {
            out.push(r);
        }
    }
    out
}
pub fn leaves(n: &V) -> Vec<String> {
    if let Some(id) = n["stack"].as_str() {
        vec![id.to_owned()]
    } else {
        arr(&n["children"]).iter().flat_map(leaves).collect()
    }
}
fn panel(w: &V, id: &str) -> Option<usize> {
    w["panels"]
        .as_array()?
        .iter()
        .position(|p| s(&p["id"]) == id)
}
fn stack(w: &V, id: &str) -> Option<usize> {
    w["stacks"]
        .as_array()?
        .iter()
        .position(|p| s(&p["id"]) == id)
}
fn home(w: &V, id: &str) -> Option<String> {
    arr(&w["stacks"])
        .iter()
        .find(|p| strings(&p["panels"]).contains(&id.to_owned()))
        .map(|p| s(&p["id"]).to_owned())
}
fn editor_stack(w: &V, id: &str) -> bool {
    id != "bottom" && id != "aux" && stack(w, id).is_some()
}
fn new_id(w: &mut V, prefix: &str) -> String {
    let mut seq = w["nextSeq"].as_u64().unwrap_or(1).max(1);
    let serialized = w.to_string();
    loop {
        let id = format!("{prefix}{seq}");
        seq += 1;
        if !serialized.contains(&format!("\"{id}\"")) {
            w["nextSeq"] = json!(seq);
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
        for i in 0..n {
            out[i] = if free.contains(&i) && free_sum > 0.0 {
                out[i] * left / free_sum
            } else {
                floor
            };
        }
    }
    out
}
fn prune(
    n: &V,
    keep: &HashSet<String>,
    seen: &mut HashSet<String>,
    ids: &mut HashSet<String>,
    seq: &mut u64,
) -> Option<V> {
    if let Some(id) = n["stack"].as_str() {
        return (keep.contains(id) && seen.insert(id.to_owned())).then(|| n.clone());
    }
    let children = arr(&n["children"]);
    let mut ws: Vec<f64> = arr(&n["weights"])
        .iter()
        .map(|x| x.as_f64().unwrap_or(0.0))
        .collect();
    if ws.len() != children.len() || ws.iter().any(|x| *x <= 0.0) {
        ws = vec![1.0; children.len()];
    }
    let mut cs = vec![];
    let mut out_ws = vec![];
    let axis = if s(&n["axis"]) == "column" {
        "column"
    } else {
        "row"
    };
    for (c, w) in children.iter().zip(ws) {
        if let Some(c) = prune(c, keep, seen, ids, seq) {
            if s(&c["axis"]) == axis {
                ids.remove(s(&c["split"]));
                for (gc, gw) in arr(&c["children"]).into_iter().zip(arr(&c["weights"])) {
                    cs.push(gc);
                    out_ws.push(w * gw.as_f64().unwrap_or(1.0));
                }
            } else {
                cs.push(c);
                out_ws.push(w);
            }
        }
    }
    match cs.len() {
        0 => None,
        1 => cs.pop(),
        _ => {
            let mut id = s(&n["split"]).to_owned();
            if id.is_empty() || !ids.insert(id.clone()) {
                loop {
                    id = format!("x{}", *seq);
                    *seq += 1;
                    if ids.insert(id.clone()) {
                        break;
                    }
                }
            }
            Some(json!({"split":id,"axis":axis,"children":cs,"weights":weights(out_ws)}))
        }
    }
}
pub fn normalize(w: &mut V) {
    if !w.is_object() {
        *w = empty();
        return;
    }
    let defaults = empty();
    for (k, v) in defaults.as_object().unwrap() {
        if w.get(k).is_none() {
            w[k] = v.clone();
        }
    }
    if !w["panels"].is_array()
        || !w["stacks"].is_array()
        || !w["files"].is_object()
        || !w["chat"].is_object()
        || !w["explorer"].is_object()
        || !w["editor"].is_object()
    {
        *w = empty();
        return;
    }
    for section in ["files", "chat", "explorer"] {
        for (k, v) in defaults[section].as_object().unwrap() {
            if w[section].get(k).is_none() {
                w[section][k] = v.clone();
            }
        }
    }
    let input_panels = arr(&w["panels"]);
    let all_stacks = arr(&w["stacks"]);
    let mut els = leaves(&w["editor"]);
    let mut seen = HashSet::new();
    els.retain(|id| {
        id != "bottom"
            && id != "aux"
            && all_stacks.iter().any(|s| s["id"] == *id)
            && seen.insert(id.clone())
    });
    let mut order = els.clone();
    order.extend(["bottom".into(), "aux".into()]);
    let mut homes = HashMap::new();
    let mut in_order = vec![];
    for st in &order {
        if let Some(stk) = all_stacks.iter().find(|a| s(&a["id"]) == st) {
            for id in strings(&stk["panels"]) {
                if input_panels
                    .iter()
                    .any(|p| s(&p["id"]) == id && valid_target(&p["target"]))
                    && !homes.contains_key(&id)
                {
                    homes.insert(id.clone(), st.clone());
                    in_order.push(id);
                }
            }
        }
    }
    let mru = strings(&w["mru"]);
    let mut rank = mru.clone();
    rank.extend(in_order.iter().filter(|id| !mru.contains(id)).cloned());
    let mut keys = HashSet::new();
    let mut kept = HashSet::new();
    for id in rank {
        if homes.contains_key(&id) {
            if let Some(p) = input_panels.iter().find(|p| s(&p["id"]) == id) {
                if keys.insert(key(&p["target"])) {
                    kept.insert(id);
                }
            }
        }
    }
    w["panels"] = json!(
        input_panels
            .into_iter()
            .filter(|p| kept.contains(s(&p["id"])))
            .collect::<Vec<_>>()
    );
    let mut seen = HashSet::new();
    let mru: Vec<_> = mru
        .into_iter()
        .filter(|id| kept.contains(id) && seen.insert(id.clone()))
        .collect();
    w["mru"] = json!(mru);
    let mut stacks = vec![];
    for st in &order {
        let old = all_stacks.iter().find(|a| s(&a["id"]) == st);
        let mut seen = HashSet::new();
        let ps: Vec<String> = old
            .map(|o| strings(&o["panels"]))
            .unwrap_or_default()
            .into_iter()
            .filter(|id| kept.contains(id) && homes.get(id) == Some(st) && seen.insert(id.clone()))
            .collect();
        let previous = old.map(|o| s(&o["active"])).unwrap_or("");
        let active = if ps.iter().any(|p| p == previous) {
            Some(previous.to_owned())
        } else {
            mru.iter()
                .find(|p| ps.contains(p))
                .cloned()
                .or_else(|| ps.first().cloned())
        };
        stacks.push(json!({"id":st,"panels":ps,"active":active}));
    }
    let mut promotion = w["chat"]["promotedFrom"].clone();
    if s(&w["paradigm"]) == "files"
        || homes.get(s(&promotion["panelId"])).map(String::as_str) != Some("aux")
        || s(&promotion["stackId"]) == "aux"
    {
        promotion = V::Null;
    }
    let mut keep: HashSet<String> = els
        .iter()
        .filter(|id| {
            stacks
                .iter()
                .any(|a| s(&a["id"]) == *id && !arr(&a["panels"]).is_empty())
                || s(&promotion["stackId"]) == *id
        })
        .cloned()
        .collect();
    if keep.is_empty() {
        if els.iter().any(|id| id == s(&w["lastEditorStack"])) {
            keep.insert(s(&w["lastEditorStack"]).into());
        } else if let Some(id) = els.first() {
            keep.insert(id.clone());
        }
    }
    let mut seq = w["nextSeq"].as_u64().unwrap_or(1).max(1);
    let editor = prune(
        &w["editor"],
        &keep,
        &mut HashSet::new(),
        &mut HashSet::new(),
        &mut seq,
    )
    .unwrap_or_else(|| {
        let id = loop {
            let id = format!("s{seq}");
            seq += 1;
            if !all_stacks.iter().any(|x| x["id"] == id) {
                break id;
            }
        };
        stacks.push(json!({"id":id,"panels":[],"active":null}));
        json!({"stack":id})
    });
    w["editor"] = editor;
    els = leaves(&w["editor"]);
    stacks.retain(|st| {
        els.contains(&s(&st["id"]).to_owned()) || matches!(s(&st["id"]), "bottom" | "aux")
    });
    w["stacks"] = json!(stacks);
    w["nextSeq"] = json!(seq);
    let recent = |allowed: &dyn Fn(&str) -> bool| {
        mru.iter()
            .filter_map(|id| home(w, id))
            .find(|st| allowed(st))
    };
    let last = if els.contains(&s(&w["lastEditorStack"]).to_owned()) {
        s(&w["lastEditorStack"]).to_owned()
    } else {
        recent(&|id| els.contains(&id.to_owned())).unwrap_or_else(|| els[0].clone())
    };
    let focus = if stack(w, s(&w["files"]["focus"])).is_some() {
        s(&w["files"]["focus"]).to_owned()
    } else {
        recent(&|_| true).unwrap_or(last.clone())
    };
    let side = if els.contains(&s(&w["chat"]["sideStack"]).to_owned())
        || s(&w["chat"]["sideStack"]) == "bottom"
    {
        s(&w["chat"]["sideStack"]).to_owned()
    } else {
        last.clone()
    };
    let cf = s(&w["chat"]["focus"]);
    let cf = if cf == "aux" || cf == side {
        cf.to_owned()
    } else if stack(w, cf).is_some() {
        side.clone()
    } else {
        "aux".into()
    };
    w["lastEditorStack"] = json!(last);
    w["files"]["focus"] = json!(focus);
    w["chat"]["sideStack"] = json!(side);
    w["chat"]["focus"] = json!(cf);
    if stack(w, s(&w["files"]["maximized"])).is_none() {
        w["files"]["maximized"] = V::Null;
    }
    if stack(w, s(&promotion["stackId"])).is_none() {
        promotion = V::Null;
    }
    w["chat"]["promotedFrom"] = promotion;
    if !matches!(s(&w["paradigm"]), "files" | "chat" | "solo") {
        w["paradigm"] = json!("files");
    }
    if panel(w, s(&w["solo"]["panelId"])).is_none() {
        if s(&w["paradigm"]) == "solo" {
            w["paradigm"] = json!(if s(&w["solo"]["returnTo"]) == "chat" {
                "chat"
            } else {
                "files"
            });
        }
        w["solo"] = V::Null;
    }
    if s(&w["paradigm"]) != "solo" {
        w["solo"] = V::Null;
    } else if !matches!(s(&w["solo"]["returnTo"]), "files" | "chat") {
        w["solo"]["returnTo"] = json!("files");
    }
    for (a, b, def, min, max) in regions() {
        let size = w[a][b]["size"].as_f64().unwrap_or(def).clamp(min, max);
        let collapsed = w[a][b]["collapsed"].as_bool().unwrap_or(false);
        w[a][b] = json!({"size":size,"collapsed":collapsed});
    }
    let mut seen = HashSet::new();
    w["explorer"]["expanded"] = json!(
        strings(&w["explorer"]["expanded"])
            .into_iter()
            .filter(|p| valid_path(p) && seen.insert(p.clone()))
            .collect::<Vec<_>>()
    );
    if !valid_path(s(&w["explorer"]["selected"])) {
        w["explorer"]["selected"] = V::Null;
    }
}
fn regions() -> [(&'static str, &'static str, f64, f64, f64); 6] {
    [
        ("files", "explorer", 264.0, 200.0, 1600.0),
        ("files", "aux", 360.0, 300.0, 1600.0),
        ("files", "bottom", 0.35, 0.1, 0.9),
        ("chat", "rail", 264.0, 200.0, 1600.0),
        ("chat", "side", 400.0, 300.0, 1600.0),
        ("chat", "tree", 200.0, 160.0, 1600.0),
    ]
}
fn remove(w: &mut V, id: &str) {
    if let Some(st) = home(w, id) {
        let i = stack(w, &st).unwrap();
        let ps = strings(&w["stacks"][i]["panels"]);
        let at = ps.iter().position(|p| p == id).unwrap();
        let ps: Vec<_> = ps.into_iter().filter(|p| p != id).collect();
        if s(&w["stacks"][i]["active"]) == id {
            let active = strings(&w["mru"])
                .into_iter()
                .find(|p| ps.contains(p))
                .or_else(|| ps.get(at.min(ps.len().saturating_sub(1))).cloned());
            w["stacks"][i]["active"] = json!(active);
        }
        w["stacks"][i]["panels"] = json!(ps);
    }
}
fn insert(w: &mut V, st: &str, id: &str, index: Option<usize>) {
    if let Some(i) = stack(w, st) {
        let mut ps = strings(&w["stacks"][i]["panels"]);
        let pos = index
            .unwrap_or_else(|| {
                ps.iter()
                    .position(|p| p == s(&w["stacks"][i]["active"]))
                    .map(|i| i + 1)
                    .unwrap_or(ps.len())
            })
            .min(ps.len());
        ps.insert(pos, id.into());
        w["stacks"][i]["panels"] = json!(ps);
        w["stacks"][i]["active"] = json!(id);
    }
}
fn restore_promotion(w: &mut V) {
    let o = w["chat"]["promotedFrom"].clone();
    w["chat"]["promotedFrom"] = V::Null;
    let id = s(&o["panelId"]);
    let st = s(&o["stackId"]);
    if home(w, id).as_deref() == Some("aux") && st != "aux" && stack(w, st).is_some() {
        remove(w, id);
        if let Some(i) = stack(w, "aux") {
            if strings(&w["stacks"][i]["panels"]).contains(&s(&o["auxActiveBefore"]).to_owned()) {
                w["stacks"][i]["active"] = o["auxActiveBefore"].clone();
            }
        }
        insert(w, st, id, o["index"].as_u64().map(|v| v as usize));
    }
}
fn enter_chat(w: &mut V) {
    let last = s(&w["lastEditorStack"]).to_owned();
    let has = stack(w, &last).is_some_and(|i| !arr(&w["stacks"][i]["panels"]).is_empty());
    let side = if has {
        last
    } else {
        strings(&w["mru"])
            .iter()
            .filter_map(|id| home(w, id))
            .find(|st| editor_stack(w, st))
            .unwrap_or(last)
    };
    w["paradigm"] = json!("chat");
    w["chat"]["sideStack"] = json!(side);
    w["chat"]["focus"] = json!("aux");
}
fn leave_solo(w: &mut V, to: Option<&str>) {
    let dest = to.unwrap_or(if s(&w["solo"]["returnTo"]) == "chat" {
        "chat"
    } else {
        "files"
    });
    w["paradigm"] = json!(dest);
    w["solo"] = V::Null;
    if dest == "chat" {
        enter_chat(w);
    } else {
        restore_promotion(w);
    }
}
fn focus_stack(w: &mut V, st: &str) {
    if stack(w, st).is_none() {
        return;
    }
    if editor_stack(w, st) {
        w["lastEditorStack"] = json!(st);
    }
    if s(&w["paradigm"]) == "chat" {
        w["chat"]["focus"] = json!(st);
        if st != "aux" {
            w["chat"]["sideStack"] = json!(st);
            w["chat"]["side"]["collapsed"] = json!(false);
        }
    } else {
        w["files"]["focus"] = json!(st);
        if matches!(st, "bottom" | "aux") {
            w["files"][st]["collapsed"] = json!(false);
        }
        if s(&w["files"]["maximized"]) != st {
            w["files"]["maximized"] = V::Null;
        }
    }
}
fn reveal(w: &mut V, id: &str) {
    if s(&w["paradigm"]) == "solo" && s(&w["solo"]["panelId"]) != id {
        leave_solo(w, None);
    }
    if let Some(st) = home(w, id) {
        let i = stack(w, &st).unwrap();
        w["stacks"][i]["active"] = json!(id);
        let mut mru = strings(&w["mru"]);
        mru.retain(|p| p != id);
        mru.insert(0, id.into());
        w["mru"] = json!(mru);
        focus_stack(w, &st);
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
fn at_edge(n: &mut V, target: Option<&str>, new: &str, edge: &str, split: &str) {
    let (axis, before) = edge_info(edge);
    let fresh = json!({"stack":new});
    if target.is_none() {
        if s(&n["axis"]) == axis {
            let mut cs = arr(&n["children"]);
            let count = cs.len();
            let mut ws: Vec<f64> = arr(&n["weights"])
                .iter()
                .map(|w| w.as_f64().unwrap_or(1.0) * count as f64 / (count + 1) as f64)
                .collect();
            let pos = if before { 0 } else { count };
            cs.insert(pos, fresh);
            ws.insert(pos, 1.0 / (count + 1) as f64);
            n["children"] = json!(cs);
            n["weights"] = json!(ws);
            return;
        }
        let old = n.clone();
        *n = json!({"split":split,"axis":axis,"children":if before{vec![fresh,old]}else{vec![old,fresh]},"weights":[0.5,0.5]});
        return;
    }
    let target = target.unwrap();
    if s(&n["stack"]) == target {
        let old = n.clone();
        *n = json!({"split":split,"axis":axis,"children":if before{vec![fresh,old]}else{vec![old,fresh]},"weights":[0.5,0.5]});
    } else if n["children"].is_array() {
        let mut cs = arr(&n["children"]);
        if s(&n["axis"]) == axis {
            if let Some(i) = cs.iter().position(|c| s(&c["stack"]) == target) {
                let mut ws = arr(&n["weights"]);
                let half = ws[i].as_f64().unwrap_or(1.0) / 2.0;
                ws[i] = json!(half);
                let pos = if before { i } else { i + 1 };
                cs.insert(pos, fresh);
                ws.insert(pos, json!(half));
                n["children"] = json!(cs);
                n["weights"] = json!(ws);
                return;
            }
        }
        for c in &mut cs {
            at_edge(c, Some(target), new, edge, split);
        }
        n["children"] = json!(cs);
    }
}
fn open(w: &mut V, t: &V, p: &V, focus: bool) {
    if !valid_target(t) {
        return;
    }
    if s(&w["paradigm"]) == "solo" {
        let same = panel(w, s(&w["solo"]["panelId"]))
            .is_some_and(|i| key(&w["panels"][i]["target"]) == key(t));
        if !same {
            leave_solo(w, None);
        }
    }
    if let Some(id) = arr(&w["panels"])
        .iter()
        .find(|a| key(&a["target"]) == key(t))
        .map(|a| s(&a["id"]).to_owned())
    {
        let st = s(&p["stackId"]);
        if s(&p["type"]) == "inStack"
            && p["moveExisting"] == true
            && stack(w, st).is_some()
            && home(w, &id).as_deref() != Some(st)
        {
            remove(w, &id);
            insert(w, st, &id, p["index"].as_u64().map(|n| n as usize));
        }
        if focus {
            reveal(w, &id);
        }
        return;
    }
    let mut st = s(&p["stackId"]).to_owned();
    let special = matches!(s(&p["type"]), "inStack" | "splitEdge") && stack(w, &st).is_some();
    if !special {
        st = match s(&t["kind"]) {
            "terminal" => "bottom".into(),
            "conversation" => "aux".into(),
            _ => {
                let candidate = if s(&w["paradigm"]) == "chat" {
                    s(&w["chat"]["sideStack"])
                } else {
                    s(&w["files"]["focus"])
                };
                if editor_stack(w, candidate) {
                    candidate.into()
                } else {
                    s(&w["lastEditorStack"]).into()
                }
            }
        };
    }
    if special && s(&p["type"]) == "inStack" && p["replaceActive"] == true {
        let i = stack(w, &st).unwrap();
        let active = s(&w["stacks"][i]["active"]).to_owned();
        if let Some(i) = panel(w, &active) {
            if w["panels"][i]["target"]["kind"] == t["kind"] {
                w["panels"][i] = json!({"id":active,"target":t,"view":{}});
                if focus {
                    reveal(w, &active);
                }
                return;
            }
        }
    }
    let id = new_id(w, "p");
    w["panels"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":id,"target":t,"view":{}}));
    if special && s(&p["type"]) == "splitEdge" && editor_stack(w, &st) {
        let fresh = new_id(w, "s");
        let split = new_id(w, "x");
        at_edge(&mut w["editor"], Some(&st), &fresh, s(&p["edge"]), &split);
        w["stacks"]
            .as_array_mut()
            .unwrap()
            .push(json!({"id":fresh,"panels":[id],"active":id}));
    } else {
        let i = stack(w, &st).unwrap();
        let old = w["stacks"][i]["active"].clone();
        insert(
            w,
            &st,
            &id,
            if special && s(&p["type"]) == "inStack" {
                p["index"].as_u64().map(|n| n as usize)
            } else {
                None
            },
        );
        if !focus && !old.is_null() {
            w["stacks"][i]["active"] = old;
        }
    }
    if focus {
        reveal(w, &id);
    }
}
fn after_bottom(w: &mut V, had: bool) {
    if had && stack(w, "bottom").is_some_and(|i| arr(&w["stacks"][i]["panels"]).is_empty()) {
        w["files"]["bottom"]["collapsed"] = json!(true);
        if s(&w["files"]["focus"]) == "bottom" {
            w["files"]["focus"] = w["lastEditorStack"].clone();
        }
        if s(&w["files"]["maximized"]) == "bottom" {
            w["files"]["maximized"] = V::Null;
        }
        if s(&w["chat"]["sideStack"]) == "bottom" {
            w["chat"]["sideStack"] = w["lastEditorStack"].clone();
            if s(&w["chat"]["focus"]) == "bottom" {
                w["chat"]["focus"] = w["lastEditorStack"].clone();
            }
        }
    }
}
fn move_panel(w: &mut V, id: &str, to: &V) {
    let Some(source) = home(w, id) else {
        return;
    };
    let mut kind = s(&to["type"]);
    let st = s(&to["stackId"]);
    if kind != "editorEdge" && stack(w, st).is_none() {
        return;
    }
    if kind == "edge" && !editor_stack(w, st) {
        kind = "center";
    }
    let had = stack(w, "bottom").is_some_and(|i| !arr(&w["stacks"][i]["panels"]).is_empty());
    match kind {
        "center" => {
            if source == st {
                return;
            }
            remove(w, id);
            insert(w, st, id, Some(usize::MAX));
        }
        "tab" => {
            let i = stack(w, st).unwrap();
            let ps = strings(&w["stacks"][i]["panels"]);
            let mut pos = to["index"].as_u64().unwrap_or(0) as usize;
            pos = pos.min(ps.len());
            if source == st {
                let from = ps.iter().position(|p| p == id).unwrap();
                if pos > from {
                    pos -= 1;
                }
            }
            remove(w, id);
            insert(w, st, id, Some(pos));
        }
        "edge" | "editorEdge" => {
            let i = stack(w, &source).unwrap();
            let only = arr(&w["stacks"][i]["panels"]).len() == 1;
            if only
                && ((kind == "edge" && st == source)
                    || (kind == "editorEdge" && leaves(&w["editor"]) == vec![source.clone()]))
            {
                return;
            }
            let fresh = new_id(w, "s");
            let split = new_id(w, "x");
            remove(w, id);
            w["stacks"]
                .as_array_mut()
                .unwrap()
                .push(json!({"id":fresh,"panels":[id],"active":id}));
            at_edge(
                &mut w["editor"],
                if kind == "edge" { Some(st) } else { None },
                &fresh,
                s(&to["edge"]),
                &split,
            );
        }
        _ => return,
    }
    reveal(w, id);
    after_bottom(w, had);
}
fn resize(n: &mut V, id: &str, ws: Option<&V>) {
    if s(&n["split"]) == id {
        let count = arr(&n["children"]).len();
        let next = ws
            .map(|v| arr(v).iter().filter_map(V::as_f64).collect::<Vec<_>>())
            .unwrap_or(vec![1.0; count]);
        if next.len() == count && next.iter().all(|x| *x > 0.0) {
            n["weights"] = json!(weights(next));
        }
    } else if let Some(cs) = n.get_mut("children").and_then(V::as_array_mut) {
        for c in cs {
            resize(c, id, ws);
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
pub fn apply(input: &V, op: &V) -> V {
    let mut w = input.clone();
    normalize(&mut w);
    let id = s(&op["panelId"]);
    let st = s(&op["stackId"]);
    match s(&op["type"]) {
        "open" => open(
            &mut w,
            &op["target"],
            &op["placement"],
            op["focus"].as_bool().unwrap_or(true),
        ),
        "focus" => {
            if panel(&w, id).is_some() {
                reveal(&mut w, id)
            }
        }
        "focusStack" => {
            if let Some(i) = stack(&w, st) {
                let active = s(&w["stacks"][i]["active"]).to_owned();
                if !active.is_empty() {
                    reveal(&mut w, &active);
                } else {
                    if s(&w["paradigm"]) == "solo" {
                        leave_solo(&mut w, None);
                    }
                    focus_stack(&mut w, st);
                }
            }
        }
        "close" => {
            let ids = strings(&op["panelIds"]);
            let had =
                stack(&w, "bottom").is_some_and(|i| !arr(&w["stacks"][i]["panels"]).is_empty());
            for id in &ids {
                remove(&mut w, id);
            }
            w["panels"] = json!(
                arr(&w["panels"])
                    .into_iter()
                    .filter(|p| !ids.contains(&s(&p["id"]).to_owned()))
                    .collect::<Vec<_>>()
            );
            w["mru"] = json!(
                strings(&w["mru"])
                    .into_iter()
                    .filter(|p| !ids.contains(p))
                    .collect::<Vec<_>>()
            );
            if ids.contains(&s(&w["solo"]["panelId"]).to_owned()) {
                leave_solo(&mut w, None);
            }
            after_bottom(&mut w, had);
        }
        "move" => move_panel(&mut w, id, &op["to"]),
        "splitStack" => {
            if editor_stack(&w, st) {
                let i = stack(&w, st).unwrap();
                if arr(&w["stacks"][i]["panels"]).len() > 1 {
                    let id = s(&w["stacks"][i]["active"]).to_owned();
                    move_panel(
                        &mut w,
                        &id,
                        &json!({"type":"edge","stackId":st,"edge":op["edge"]}),
                    );
                }
            }
        }
        "resizeSplit" | "resetSplit" => resize(
            &mut w["editor"],
            s(&op["splitId"]),
            if s(&op["type"]) == "resetSplit" {
                None
            } else {
                Some(&op["weights"])
            },
        ),
        "resizeRegion" | "setRegionCollapsed" | "toggleRegion" => {
            let region = s(&op["region"]);
            let pair = match region {
                "explorer" => Some(("files", "explorer")),
                "aux" => Some(("files", "aux")),
                "bottom" => Some(("files", "bottom")),
                "chat_rail" => Some(("chat", "rail")),
                "chat_side" => Some(("chat", "side")),
                "chat_tree" => Some(("chat", "tree")),
                _ => None,
            };
            if let Some((a, b)) = pair {
                match s(&op["type"]) {
                    "resizeRegion" => {
                        if op["size"].is_number() {
                            w[a][b]["size"] = op["size"].clone();
                        }
                    }
                    "toggleRegion" => w[a][b]["collapsed"] = json!(w[a][b]["collapsed"] != true),
                    _ => w[a][b]["collapsed"] = json!(op["collapsed"] == true),
                }
            }
        }
        "setMaximized" => {
            if op["stackId"].is_null() {
                w["files"]["maximized"] = V::Null;
            } else if stack(&w, st).is_some() {
                if s(&w["paradigm"]) == "files" {
                    let i = stack(&w, st).unwrap();
                    let active = s(&w["stacks"][i]["active"]).to_owned();
                    if !active.is_empty() {
                        reveal(&mut w, &active);
                    } else {
                        focus_stack(&mut w, st);
                    }
                }
                w["files"]["maximized"] = json!(st);
            }
        }
        "switchParadigm" => match s(&op["paradigm"]) {
            "files" => {
                if s(&w["paradigm"]) == "solo" {
                    leave_solo(&mut w, Some("files"));
                } else {
                    w["paradigm"] = json!("files");
                    restore_promotion(&mut w);
                }
            }
            "chat" => {
                if s(&w["paradigm"]) != "chat" {
                    if s(&w["paradigm"]) == "solo" {
                        leave_solo(&mut w, Some("files"));
                    }
                    enter_chat(&mut w);
                }
            }
            _ => {}
        },
        "promoteConversation" => {
            if panel(&w, id).is_some_and(|i| s(&w["panels"][i]["target"]["kind"]) == "conversation")
            {
                if s(&w["paradigm"]) == "solo" {
                    leave_solo(&mut w, None);
                }
                if let Some(source) = home(&w, id) {
                    if source != "aux" {
                        let i = stack(&w, &source).unwrap();
                        let idx = strings(&w["stacks"][i]["panels"])
                            .iter()
                            .position(|p| p == id)
                            .unwrap();
                        let aux = stack(&w, "aux").unwrap();
                        let origin = json!({"panelId":id,"stackId":source,"index":idx,"auxActiveBefore":w["stacks"][aux]["active"]});
                        remove(&mut w, id);
                        insert(&mut w, "aux", id, None);
                        w["chat"]["promotedFrom"] = origin;
                    }
                    if s(&w["paradigm"]) != "chat" {
                        enter_chat(&mut w);
                    }
                    reveal(&mut w, id);
                }
            }
        }
        "returnToFiles" => {
            if s(&w["paradigm"]) == "solo" {
                leave_solo(&mut w, Some("files"));
            } else {
                w["paradigm"] = json!("files");
                restore_promotion(&mut w);
            }
        }
        "enterSolo" => {
            if valid_target(&op["target"]) {
                let ret = if s(&w["paradigm"]) == "solo" {
                    s(&w["solo"]["returnTo"])
                } else {
                    s(&w["paradigm"])
                }
                .to_owned();
                w["paradigm"] = json!(ret);
                w["solo"] = V::Null;
                open(&mut w, &op["target"], &V::Null, true);
                let id = arr(&w["panels"])
                    .iter()
                    .find(|p| key(&p["target"]) == key(&op["target"]))
                    .map(|p| s(&p["id"]).to_owned())
                    .unwrap();
                w["paradigm"] = json!("solo");
                w["solo"] = json!({"panelId":id,"returnTo":ret});
            }
        }
        "setChatSideStack" => {
            if editor_stack(&w, st) || st == "bottom" {
                if editor_stack(&w, st) {
                    w["lastEditorStack"] = json!(st);
                }
                w["chat"]["sideStack"] = json!(st);
                if s(&w["paradigm"]) == "chat" {
                    w["chat"]["focus"] = json!(st);
                }
                w["chat"]["side"]["collapsed"] = json!(false);
            }
        }
        "retarget" => {
            if let Some(i) = panel(&w, id) {
                if valid_target(&op["target"]) {
                    if let Some(other) = arr(&w["panels"])
                        .iter()
                        .find(|p| key(&p["target"]) == key(&op["target"]))
                        .map(|p| s(&p["id"]).to_owned())
                    {
                        if other == id {
                            w["panels"][i]["target"] = op["target"].clone();
                        }
                        reveal(&mut w, &other);
                    } else {
                        w["panels"][i] = json!({"id":id,"target":op["target"],"view":{}});
                        reveal(&mut w, id);
                    }
                }
            }
        }
        "updateView" => {
            if let Some(i) = panel(&w, id) {
                if op["view"].is_object() {
                    w["panels"][i]["view"] = op["view"].clone();
                }
            }
        }
        "updateExplorer" => {
            if op["explorer"].is_object() {
                w["explorer"] = op["explorer"].clone();
            }
        }
        "renamePath" => {
            let from = s(&op["from"]);
            let to = s(&op["to"]);
            if valid_path(from) && valid_path(to) {
                let mut renamed = vec![];
                for p in w["panels"].as_array_mut().unwrap() {
                    if matches!(s(&p["target"]["kind"]), "file" | "image" | "diff") {
                        let new = rebase(s(&p["target"]["path"]), from, to);
                        if new != s(&p["target"]["path"]) {
                            renamed.push(s(&p["id"]).to_owned());
                            p["target"]["path"] = json!(new);
                        }
                    }
                }
                let ps = arr(&w["panels"]);
                let unique: HashSet<_> = ps.iter().map(|p| key(&p["target"])).collect();
                if unique.len() < ps.len() {
                    let mru = strings(&w["mru"]);
                    renamed.extend(
                        mru.into_iter()
                            .filter(|id| !renamed.clone().contains(id))
                            .collect::<Vec<_>>(),
                    );
                    w["mru"] = json!(renamed);
                }
                w["explorer"]["expanded"] = json!(
                    strings(&w["explorer"]["expanded"])
                        .iter()
                        .map(|p| rebase(p, from, to))
                        .collect::<Vec<_>>()
                );
                if let Some(p) = w["explorer"]["selected"].as_str() {
                    w["explorer"]["selected"] = json!(rebase(p, from, to));
                }
            }
        }
        _ => {}
    }
    normalize(&mut w);
    w
}
