/// [`VlqData`], used to represent a value as a VLQ byte string.
mod data;
mod reader;

pub(crate) use data::{VlqData, VlqEncodeError};
pub(crate) use reader::{VlqDecodeError, VlqReaderSM};
