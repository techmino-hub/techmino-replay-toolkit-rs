/// [`VlqData`], used to represent a value as a VLQ byte string.
mod data;
mod reader;

pub(crate) use data::VlqData;
pub(crate) use reader::{VlqDecodeError, VlqReader};
