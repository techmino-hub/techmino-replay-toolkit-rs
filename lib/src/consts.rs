//! Module for any constants related to Techmino, especially its replays

/// The total amount of pieces in the current game.
///
/// There are currently 29 elements:
/// - 1 monomino
/// - 1 domino
/// - 2 trominoes
/// - 7 tetrominoes
/// - 18 pentominoes
pub const TOTAL_PIECE_COUNT: usize = 29;

/// Zlib always begins with 0x78 (`x`). \
/// <https://en.wikipedia.org/wiki/List_of_file_signatures>
pub const ZLIB_HEADER_FIRST_BYTE: u8 = b'x';
/// 0x7800 until 0x78FF always starts with an `e` in base64
pub const BASE64_ZLIB_FIRST_BYTE: u8 = b'e';
/// Raw uncompressed game data begins with a JSON object, which begins with a `{`
pub const UNCOMPRESSED_FIRST_BYTE: u8 = b'{';

/// The separator between the metadata and input event data sections of the raw
/// (uncompressed) versions of the replay.
pub const METADATA_EVENTDATA_SEPARATOR: u8 = b'\n';

/// The format string the game uses to format the date for the replay's metadata.
pub static METADATA_DATE_FORMAT: &str = "%Y/%m/%d %H:%M:%S";
