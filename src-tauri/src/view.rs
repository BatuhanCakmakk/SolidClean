//! Tarama ağacının arayüze gönderilen, sınırlandırılmış görünümleri.
//!
//! Ağacın tamamı Rust tarafında kalır. Arayüz yalnızca o an gösterdiği klasörü,
//! birkaç seviye derinlikte ve en büyük N öğeyle ister.

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};

use serde::Serialize;

use crate::category::{self, Category};
use crate::cleanup::Safety;
use crate::scanner::{Kind, Node, ScanResult};

/// Temizlik önerilerinin işaretlediği yollar (küçük harfli tam yol → seviye).
pub type Marks = HashMap<String, Safety>;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeView {
    pub name: String,
    /// "dir" | "file" | "small" | "rest"
    pub kind: &'static str,
    pub category: Category,
    pub flags: u8,
    pub size: u64,
    pub alloc: u64,
    pub files: u64,
    pub dirs: u64,
    /// Unix zamanı, milisaniye.
    pub modified: i64,
    /// Üst klasördeki sırası; "rest" için -1.
    pub index: i64,
    pub child_count: usize,
    /// Bir temizlik önerisinin kapsamındaysa seviyesi.
    pub cleanup: Option<Safety>,
    pub children: Vec<NodeView>,
}

/// `node_view` özyinelemesinde değişmeyen parametreler.
struct ViewCtx<'a> {
    limit: usize,
    marks: &'a Marks,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub alloc: u64,
    pub category: Category,
    pub modified: i64,
}

pub fn child_category(node: &Node, parent: Category, level: usize) -> Category {
    match node.kind {
        Kind::Dir => category::classify_dir(&node.name, parent, level),
        Kind::File => category::classify_file(&node.name, parent, level),
        Kind::SmallFiles => parent,
    }
}

fn unix_ms(filetime: u64) -> i64 {
    if filetime == 0 {
        return 0;
    }
    (filetime / 10_000) as i64 - 11_644_473_600_000
}

#[allow(clippy::too_many_arguments)]
fn node_view(
    ctx: &ViewCtx,
    node: &Node,
    index: i64,
    category: Category,
    level: usize,
    depth: u8,
    path: &mut String,
    cleanup: Option<Safety>,
) -> NodeView {
    let limit = ctx.limit;
    let mut children = Vec::new();
    if depth > 0 {
        let mut rest = NodeView {
            name: String::new(),
            kind: "rest",
            category: Category::Other,
            flags: 0,
            size: 0,
            alloc: 0,
            files: 0,
            dirs: 0,
            modified: 0,
            index: -1,
            child_count: 0,
            cleanup: None,
            children: Vec::new(),
        };
        for (i, child) in node.children.iter().enumerate() {
            if child.size == 0 && child.alloc == 0 && child.flags == 0 {
                continue;
            }
            if children.len() < limit {
                let child_cat = child_category(child, category, level + 1);
                let len = path.len();
                path.push('\\');
                path.push_str(&child.name);
                let child_cleanup = mark_for(ctx.marks, path).or(cleanup);
                children.push(node_view(ctx, child, i as i64, child_cat, level + 1, depth - 1, path, child_cleanup));
                path.truncate(len);
            } else {
                rest.size += child.size;
                rest.alloc += child.alloc;
                rest.files += child.files;
                rest.dirs += child.dirs;
                rest.child_count += 1;
            }
        }
        if rest.child_count > 0 {
            children.push(rest);
        }
    }
    NodeView {
        name: node.name.to_string(),
        kind: match node.kind {
            Kind::Dir => "dir",
            Kind::File => "file",
            Kind::SmallFiles => "small",
        },
        category,
        flags: node.flags,
        size: node.size,
        alloc: node.alloc,
        files: node.files,
        dirs: node.dirs,
        modified: unix_ms(node.modified),
        index,
        child_count: node.children.len(),
        cleanup,
        children,
    }
}

fn mark_for(marks: &Marks, path: &str) -> Option<Safety> {
    if marks.is_empty() {
        return None;
    }
    marks.get(&path.to_lowercase()).copied()
}

/// `path`: kökten itibaren çocuk sıra numaraları. Tam yolu ve görünümü döndürür.
pub fn subtree(result: &ScanResult, path: &[u32], depth: u8, limit: usize, marks: &Marks) -> Option<(String, NodeView)> {
    let (mut category, mut level) = category::for_path(&result.root_path);
    let mut node = &result.root;
    let mut full = result.root_path.trim_end_matches('\\').to_string();
    let mut cleanup = mark_for(marks, &full);
    let mut index = 0i64;
    for &i in path {
        node = node.children.get(i as usize)?;
        level += 1;
        category = child_category(node, category, level);
        full.push('\\');
        full.push_str(&node.name);
        cleanup = mark_for(marks, &full).or(cleanup);
        index = i as i64;
    }
    let ctx = ViewCtx { limit, marks };
    let mut walk = full.clone();
    let view = node_view(&ctx, node, index, category, level, depth, &mut walk, cleanup);
    if full.ends_with(':') {
        full.push('\\');
    }
    Some((full, view))
}

pub fn largest_files(result: &ScanResult, limit: usize) -> Vec<FileEntry> {
    let (category, level) = category::for_path(&result.root_path);
    let mut path = result.root_path.trim_end_matches('\\').to_string();
    let mut heap = BinaryHeap::new();
    let mut found = Vec::new();
    collect_files(&result.root, category, level, &mut path, limit, &mut heap, &mut found);
    let mut out: Vec<FileEntry> = heap.into_iter().filter_map(|Reverse((_, i))| found[i].take()).collect();
    out.sort_unstable_by(|a, b| b.alloc.cmp(&a.alloc));
    out
}

fn collect_files(
    node: &Node,
    category: Category,
    level: usize,
    path: &mut String,
    limit: usize,
    heap: &mut BinaryHeap<Reverse<(u64, usize)>>,
    found: &mut Vec<Option<FileEntry>>,
) {
    for child in &node.children {
        let child_cat = child_category(child, category, level + 1);
        // Windows'un kendi dosyaları (hiberfil.sys, pagefile.sys, C:\Windows...) haritada kalır ama
        // listede yer almaz: kullanıcının silebileceği ya da taşıyabileceği şeyler değil.
        if child_cat == Category::System {
            continue;
        }
        match child.kind {
            Kind::File => {
                let threshold = if heap.len() < limit { 0 } else { heap.peek().map_or(0, |r| r.0 .0) };
                if child.alloc > threshold {
                    found.push(Some(FileEntry {
                        name: child.name.to_string(),
                        path: format!("{path}\\{}", child.name),
                        size: child.size,
                        alloc: child.alloc,
                        category: child_cat,
                        modified: unix_ms(child.modified),
                    }));
                    heap.push(Reverse((child.alloc, found.len() - 1)));
                    if heap.len() > limit {
                        heap.pop();
                    }
                }
            }
            Kind::Dir if !child.children.is_empty() => {
                let len = path.len();
                path.push('\\');
                path.push_str(&child.name);
                collect_files(child, child_cat, level + 1, path, limit, heap, found);
                path.truncate(len);
            }
            _ => {}
        }
    }
}
