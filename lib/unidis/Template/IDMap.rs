#![allow(dead_code)]

use libc::{getegid, geteuid, gid_t, uid_t};
use std::{clone::Clone, fmt::Display, iter::DoubleEndedIterator, marker::Copy, option::Option};

/* IDMap is a simple mapping structure from SOURCE -> TARGET. */
#[derive(Copy, Clone)]
struct IDMap<T> {
    id: Option<T>,
    lowerid: Option<T>,
    count: T,
}

/* UIDMAP is the UID mapping to map from outside to inside the namespace. */
const UIDMAP: [IDMap<uid_t>; 1] = [
    // Map current user to `root`
    IDMap {
        id: Some(0),
        lowerid: None,
        count: 1,
    },
];

/* GIDMAP is the GID mapping to map from outside to inside the namespace. */
const GIDMAP: [IDMap<gid_t>; 1] = [
    // Map current group to `root`
    IDMap {
        id: Some(0),
        lowerid: None,
        count: 1,
    },
];

/* idmap() creates an IDMap'ing from the provided IDMap replacing `None`
with the DEFAULT value. */
fn idmap<I, U>(idmap_iter: I, default: U) -> impl DoubleEndedIterator<Item = IDMap<U>>
where
    I: DoubleEndedIterator<Item = IDMap<U>>,
    U: Copy,
{
    idmap_iter.map(move |idmap| IDMap {
        id: Some(idmap.id.unwrap_or(default)),
        lowerid: Some(idmap.lowerid.unwrap_or(default)),
        count: idmap.count,
    })
}

/* idmap_rev() reverse's an IDMap'ing from outside to inside to inside to
outside the namespace, effectively resulting in the same IDMap as before
the IDMap was originally applied. */
fn idmap_rev<I, U>(idmap_iter: I) -> impl DoubleEndedIterator<Item = IDMap<U>>
where
    I: DoubleEndedIterator<Item = IDMap<U>>,
    U: Copy,
{
    idmap_iter.rev().map(|idmap| IDMap {
        id: idmap.lowerid,
        lowerid: idmap.id,
        count: idmap.count,
    })
}

/* idmap_str() converts an IDMap'ing into a form suitable for writing to
`uid_map` or `gid_map`. */
fn idmap_str<I, U>(idmap_iter: I) -> String
where
    I: Iterator<Item = IDMap<U>>,
    U: Display,
{
    idmap_iter
        .map(|idmap| {
            format!(
                "{} {} {}",
                idmap.id.unwrap(),
                idmap.lowerid.unwrap(),
                idmap.count
            )
        })
        // .chain(std::iter::once(String::new())) // Lines are terminated by newlines
        .reduce(|a, b| format!("{}\n{}", a, b))
        .unwrap()
}

/* newmap() returns the mapping as dictated from IDMAP. */
fn newmap<I, U>(idmap: I) -> String
where
    I: Iterator<Item = IDMap<U>>,
    U: Display,
{
    idmap_str(idmap)
}

/* revmap() returns a mapping that would effectively revert the application
of IDMAP. */
fn revmap<I, U>(idmap: I) -> String
where
    I: DoubleEndedIterator<Item = IDMap<U>>,
    U: Copy + Display,
{
    idmap_str(idmap_rev(idmap))
}

/* newuidmap() returns the UID mapping from outside to inside the namespace. */
pub fn newuidmap() -> String {
    let euid = unsafe { geteuid() };
    newmap(idmap(UIDMAP.iter().copied(), euid))
}

/* revuidmap() returns the UID mapping that reverts the mapping from
newuidmap(). */
pub fn revuidmap() -> String {
    let euid = unsafe { geteuid() };
    revmap(idmap(UIDMAP.iter().copied(), euid))
}

/* newgidmap() returns the GID mapping from outside to inside the namespace. */
pub fn newgidmap() -> String {
    let egid = unsafe { getegid() };
    newmap(idmap(GIDMAP.iter().copied(), egid))
}

/* revgidmap() returns the GID mapping that reverts the mapping from
newuidmap(). */
pub fn revgidmap() -> String {
    let egid = unsafe { getegid() };
    revmap(idmap(GIDMAP.iter().copied(), egid))
}
