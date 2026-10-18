//! Reusable writing panel, concrete editor/view state and presentation helpers.
//! Native modules require `native-ui`; the default graph remains toolkit-free.

pub mod diff;
pub mod formatting;

#[cfg(feature = "native-ui")]
pub mod native;

#[cfg(feature = "native-ui")]
pub mod writing;

#[cfg(feature = "native-ui")]
pub mod document;

#[cfg(feature = "native-ui")]
pub mod workbench;

pub mod layout;

#[cfg(feature = "native-ui")]
pub mod markdown;

#[cfg(feature = "native-ui")]
pub mod components;

#[cfg(feature = "native-ui")]
pub mod theme;
