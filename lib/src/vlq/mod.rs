/// [`VlqData`], used to represent a value as a VLQ byte string.
mod data;
mod reader;

pub use data::VlqData;
pub use reader::{VlqDecodeError, VlqReader};
