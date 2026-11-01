pub mod api;
pub mod cache;
pub mod crypto;
pub mod download;
pub mod error;
pub mod jobs;
pub mod link;
pub mod lrc;
pub mod model;
pub mod providers;
pub mod source;
pub mod store;
pub mod tag;

pub use api::Netease;
pub use error::{Error, Result};
pub use model::*;
pub use cache::DataCache;
pub use source::{DynSource, Source, Sources};
