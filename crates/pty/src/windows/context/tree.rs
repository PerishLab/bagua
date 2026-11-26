use super::{super::handle::Handle, proc::Target};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    io,
    mem::{size_of, zeroed},
};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
};

const CAP: usize = 256;

struct Node {
    pid: u32,
    parent: u32,
    ignored: bool,
}

pub(super) fn select(root: u32, baseline: u64) -> io::Result<Target> {
    let nodes = snapshot()?;
    let candidates = descendants(&nodes, root);
    let mut best: Option<(usize, u64, Target)> = None;
    for (pid, depth) in candidates {
        let Ok(candidate) = Target::open(pid) else {
            continue;
        };
        let Ok(stamp) = candidate.stamp() else {
            continue;
        };
        if stamp.started < baseline {
            continue;
        }
        let rank = (depth, stamp.started);
        if best
            .as_ref()
            .is_none_or(|(height, started, _)| rank > (*height, *started))
        {
            best = Some((depth, stamp.started, candidate));
        }
    }
    match best {
        Some((_, _, target)) => Ok(target),
        None => Target::open(root),
    }
}

fn snapshot() -> io::Result<Vec<Node>> {
    let handle = Handle::new(unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) })?;
    let mut entry: PROCESSENTRY32W = unsafe { zeroed() };
    entry.dwSize = size_of::<PROCESSENTRY32W>() as u32;
    let mut items = Vec::new();
    if unsafe { Process32FirstW(handle.raw(), &mut entry) } == 0 {
        return Err(io::Error::last_os_error());
    }
    loop {
        items.push(Node {
            pid: entry.th32ProcessID,
            parent: entry.th32ParentProcessID,
            ignored: ignored(&entry.szExeFile),
        });
        if unsafe { Process32NextW(handle.raw(), &mut entry) } == 0 {
            break;
        }
    }
    Ok(items)
}

fn ignored(wide: &[u16]) -> bool {
    let end = wide
        .iter()
        .position(|code| *code == 0)
        .unwrap_or(wide.len());
    let Ok(name) = String::from_utf16(&wide[..end]) else {
        return false;
    };
    name.eq_ignore_ascii_case("conhost.exe") || name.eq_ignore_ascii_case("openconsole.exe")
}

fn descendants(nodes: &[Node], root: u32) -> Vec<(u32, usize)> {
    let mut children: HashMap<u32, Vec<u32>> = HashMap::new();
    for node in nodes {
        if node.ignored || node.pid == 0 || node.pid == node.parent {
            continue;
        }
        children.entry(node.parent).or_default().push(node.pid);
    }
    let mut seen = HashSet::from([root]);
    let mut queue = VecDeque::from([(root, 0usize)]);
    let mut found = Vec::new();
    while let Some((current, depth)) = queue.pop_front() {
        let Some(next) = children.get(&current) else {
            continue;
        };
        for &child in next {
            if seen.len() >= CAP {
                return found;
            }
            if seen.insert(child) {
                found.push((child, depth + 1));
                queue.push_back((child, depth + 1));
            }
        }
    }
    found
}
