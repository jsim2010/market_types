//! Implements common instances of a [`market`].
#![cfg_attr(feature = "unstable-doc-cfg", feature(doc_cfg))]
#![no_std]

extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

pub mod compose;
pub mod convert;

#[cfg(feature = "crossbeam-channel")]
#[cfg_attr(feature = "unstable-doc-cfg", doc(cfg(feature = "crossbeam-channel")))]
pub mod channel_crossbeam;
#[cfg(feature = "std")]
#[cfg_attr(feature = "unstable-doc-cfg", doc(cfg(feature = "std")))]
pub mod channel_std;
#[cfg(feature = "std")]
#[cfg_attr(feature = "unstable-doc-cfg", doc(cfg(feature = "std")))]
pub mod collections;
#[cfg(feature = "std")]
#[cfg_attr(feature = "unstable-doc-cfg", doc(cfg(feature = "std")))]
pub mod io;
#[cfg(feature = "std")]
#[cfg_attr(feature = "unstable-doc-cfg", doc(cfg(feature = "std")))]
pub mod process;
#[cfg(feature = "crossbeam-queue")]
#[cfg_attr(feature = "unstable-doc-cfg", doc(cfg(feature = "crossbeam-queue")))]
pub mod queue_crossbeam;
#[cfg(feature = "thread")]
#[cfg_attr(feature = "unstable-doc-cfg", doc(cfg(feature = "thread")))]
pub mod thread;
