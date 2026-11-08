use crate::Failure;
use std::collections::BTreeSet;

pub fn valid(name: &str) -> bool {
    if name.is_empty() || name.len() > 64 {
        return false;
    }
    if name.trim() != name {
        return false;
    }
    !name.chars().any(char::is_control)
}

pub(crate) fn available(
    prefix: &str,
    names: impl Iterator<Item = String>,
) -> Result<String, Failure> {
    let names: BTreeSet<_> = names.collect();
    (1..=33)
        .map(|number| format!("{prefix}-{number}"))
        .find(|name| !names.contains(name))
        .ok_or(Failure::Capacity)
}
