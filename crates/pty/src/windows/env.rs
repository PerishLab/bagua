use radicula_process::Environment;
use std::{
    ffi::{OsStr, OsString},
    os::windows::ffi::OsStrExt,
    path::Path,
};

pub(super) fn wide(value: &OsStr) -> Vec<u16> {
    let mut units: Vec<u16> = value.encode_wide().collect();
    units.push(0);
    units
}

pub(super) fn line(program: &Path, args: &[OsString]) -> Vec<u16> {
    let mut units = Vec::new();
    quote(program.as_os_str(), &mut units);
    for argument in args {
        units.push(b' ' as u16);
        quote(argument.as_os_str(), &mut units);
    }
    units.push(0);
    units
}

pub(super) fn block(environment: &Environment) -> Vec<u16> {
    let mut entries: Vec<(Vec<u16>, Vec<u16>)> = environment
        .values()
        .iter()
        .map(|(key, value)| {
            let key: Vec<u16> = key.encode_wide().collect();
            let value: Vec<u16> = value.encode_wide().collect();
            (key, value)
        })
        .collect();
    entries.sort_by(|left, right| compare(&left.0, &right.0));
    let mut units = Vec::new();
    for (key, value) in entries {
        units.extend_from_slice(&key);
        units.push(b'=' as u16);
        units.extend_from_slice(&value);
        units.push(0);
    }
    if units.is_empty() {
        units.push(0);
    }
    units.push(0);
    units
}

fn quote(argument: &OsStr, target: &mut Vec<u16>) {
    let units: Vec<u16> = argument.encode_wide().collect();
    let needs = units.is_empty()
        || units
            .iter()
            .any(|&unit| unit == b' ' as u16 || unit == b'\t' as u16 || unit == b'"' as u16);
    if !needs {
        target.extend_from_slice(&units);
        return;
    }
    target.push(b'"' as u16);
    let mut slashes = 0usize;
    for unit in units {
        if unit == b'\\' as u16 {
            slashes += 1;
            continue;
        }
        if unit == b'"' as u16 {
            for _ in 0..(slashes * 2 + 1) {
                target.push(b'\\' as u16);
            }
            target.push(b'"' as u16);
            slashes = 0;
            continue;
        }
        for _ in 0..slashes {
            target.push(b'\\' as u16);
        }
        slashes = 0;
        target.push(unit);
    }
    for _ in 0..(slashes * 2) {
        target.push(b'\\' as u16);
    }
    target.push(b'"' as u16);
}

fn compare(left: &[u16], right: &[u16]) -> std::cmp::Ordering {
    for (&a, &b) in left.iter().zip(right.iter()) {
        let a = upper(a);
        let b = upper(b);
        if a != b {
            return a.cmp(&b);
        }
    }
    left.len().cmp(&right.len())
}

fn upper(unit: u16) -> u16 {
    if (b'a' as u16..=b'z' as u16).contains(&unit) {
        unit - 32
    } else {
        unit
    }
}
