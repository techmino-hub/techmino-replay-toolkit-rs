//! Handling and parsing CLI arguments.

use clap::{Args, Parser, Subcommand, ValueEnum};
use core::{fmt::Display, num::IntErrorKind};
use libtechmino_replay::{InputParseMode, ReplayBufferKind};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    version,
    about,
    long_about = "
╭~~~~~~~~~~~~~╮
┊ ▀▀█▀▀    █  ┊  Techmino Replay Toolkit
┊   █  █▀█ █▀ ┊  We don't condone misusing this for nefarious purposes!
┊   █  █   █▄ ┊  https://github.com/techmino-hub/techmino-replay-toolkit-rs
╰~~~~~~~~~~~~~╯
This program and library is licensed under the GNU General Public License version 3.
For more information, see <https://www.gnu.org/licenses/>.
"
)]
pub struct CliParser {
    #[command(subcommand)]
    pub command: CliCommand,
}

/// The specific command to run.
#[derive(Clone, Debug, Subcommand)]
pub enum CliCommand {
    /// Do a one-off operation in the CLI.
    Cli {
        /// The operation to do.
        #[command(subcommand)]
        operation: CliOperation,
    },
    /// Start the TRT terminal user interface.
    #[cfg(feature = "tui")]
    Tui,
    /// Start the TRT graphical user interface.
    #[cfg(feature = "gui")]
    Gui,
}

#[derive(Clone, Debug, Subcommand)]
pub enum CliOperation {
    /// Extract information about a replay into JSON form.
    ///
    /// Format is meant to be backwards compatible with
    /// `techmino-replay-parser@v4`'s output.
    /// (`techmino-replay-parser`: <https://www.npmjs.com/package/techmino-replay-parser>)
    Extract(ExtractArguments),
    /// Create a replay from a JSON input.
    Create(CreateArguments),
    /// Turn a `.rep` file into a base64 pasteable text.
    Base64ify(Base64ifyArguments),
    /// Turn base64 pasteable text into a `.rep` file.
    Binaryify(BinaryifyArguments),
    /// Recompress a replay to slightly reduce its size.
    Shrink(ShrinkArguments),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Args)]
pub struct RetryArguments {
    /// How many times to retry in case of a "resumable" I/O error.
    ///
    /// Negative values for infinite retries.
    ///
    /// Defaults to never retrying.
    #[arg(short = 'r', long = "retries", value_parser = MaxRetryCount::try_from_str, default_value_t = MaxRetryCount::NEVER)]
    pub max_retries: MaxRetryCount,
    /// Retry ALL I/O errors, not just "resumable" ones.
    #[arg(long)]
    pub retry_all_io: bool,
    /// How long to wait after an I/O error to retry the I/O operation, in milliseconds.
    #[arg(long = "retry-delay", default_value_t = 5000)]
    pub retry_delay_ms: u64,
}

/// How many times to retry an operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MaxRetryCount {
    /// Retries none or a few times.
    Finite(u32),
    /// Retries infinite times.
    Infinite,
}

impl MaxRetryCount {
    /// Never retry.
    pub const NEVER: Self = Self::Finite(0);

    /// Tries to convert from a specific terminal string slice into a valid
    /// `RetryCount`.
    ///
    /// All valid nonnegative values map into `Self::Finite(u32)`.
    /// Overflowing values or strings that begin with `-` map into `Self::Infinite`.
    pub fn try_from_str(s: &str) -> Result<Self, String> {
        if s.starts_with('-') {
            return Ok(Self::Infinite);
        };

        match s.parse::<u32>() {
            Ok(num) => Ok(Self::Finite(num)),
            Err(e) if *e.kind() == IntErrorKind::PosOverflow => Ok(Self::Infinite),
            Err(e) => Err(e.to_string()),
        }
    }

    /// Returns whether or not a certain number of retries is at the limit.
    pub fn is_at_limit(self, retry_counter: u32) -> bool {
        match self {
            Self::Finite(num) => retry_counter >= num,
            Self::Infinite => false,
        }
    }
}

impl Default for MaxRetryCount {
    fn default() -> Self {
        Self::NEVER
    }
}

impl Display for MaxRetryCount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MaxRetryCount::Finite(num) => Display::fmt(&num, f),
            MaxRetryCount::Infinite => f.write_str("-1"),
        }
    }
}

/// What format of replay to treat the input as.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, ValueEnum)]
pub enum CliReplayFormat {
    /// Treat the input as the binary representation of the replay.
    ///
    /// The binary replay format is the format used by the `.rep` files in the game's
    /// save directory.
    Binary,
    /// Treat the input as the base64/text representation of the replay.
    ///
    /// The base64 form of the replay is achieved by copying using the in-game
    /// Replays menu.
    Base64,
}

impl From<CliReplayFormat> for ReplayBufferKind {
    fn from(value: CliReplayFormat) -> Self {
        match value {
            CliReplayFormat::Binary => Self::Compressed,
            CliReplayFormat::Base64 => Self::Base64,
        }
    }
}

/// How the inputs of the replay are to be parsed and created.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, ValueEnum)]
pub enum CliInputMode {
    /// Uses relative timing for input data (0.17.22+).
    ///
    /// Replays made before version 0.17.22 of the game (i.e., 0.17.21 and before it)
    /// use relative timing for its inputs. That is, the time in each key-time
    /// pair are relative to the frame of the previous input.
    ///
    /// For example, if you press two keys at the exact same frame, the first input
    /// has a stored time of the number of frames since the previous input,
    /// while the second input has a time of 0.
    Relative,
    /// Uses absolute timing for input data (before 0.17.22)
    ///
    /// Replays made after version 0.17.21 of the game (i.e., 0.17.22 and onwards)
    /// use absolute timing for its inputs. That is, the time in each key-time
    /// pair are relative to the beginning of the replay (i.e., frame zero).
    ///
    /// For example, if you press two keys at the exact same frame, the first input
    /// has a time of the current frame number, as well as the second input.
    Absolute,
}

impl From<CliInputMode> for InputParseMode {
    fn from(value: CliInputMode) -> Self {
        match value {
            CliInputMode::Absolute => Self::Absolute,
            CliInputMode::Relative => Self::Relative,
        }
    }
}

/// General I/O-related arguments.
#[derive(Args, Debug, Clone)]
pub(super) struct IoArguments {
    /// The input file to read from. If omitted, reads from stdin.
    #[arg(short = 'i', long)]
    pub(super) input_file: Option<PathBuf>,
    /// The output file to write into. If omitted, writes into stdout.
    #[arg(short = 'o', long)]
    pub(super) output_file: Option<PathBuf>,
    #[command(flatten)]
    pub(super) retry_args: RetryArguments,
}

/// Arguments for the `extract` command.
#[derive(Args, Debug, Clone)]
pub struct ExtractArguments {
    /// What to extract from the replay.
    #[command(subcommand)]
    pub(super) extract_mode: ExtractMode,
    /// The replay format to interpret the input data as.
    ///
    /// If not provided, the replay format is inferred from the input stream.
    /// This is usually pretty accurate.
    #[arg(short = 'f', long)]
    pub(super) replay_format: Option<CliReplayFormat>,
    /// An override for the input mode.
    ///
    /// If omitted, the input mode is inferred from the replay's
    /// metadata's version string.
    ///
    /// Inference is usually pretty accurate, but this
    /// override is useful if inference failed and you KNOW which mode you
    /// want to use. An example usecase is if you're using a mod of the game
    /// that changes up the version string and this library fails to parse it.
    ///
    /// Note that misuse may cause malformed replays that fail to play back
    /// properly. Handle with care!
    #[arg(long)]
    pub(super) override_input_mode: Option<CliInputMode>,
    #[command(flatten)]
    pub(super) io_args: IoArguments,
}

/// Arguments for the `create` command.
#[derive(Args, Debug, Clone)]
pub struct CreateArguments {
    /// What format to create the replay in.
    #[arg(short = 'f', long)]
    pub(super) replay_format: CliReplayFormat,
    /// How much to compress the replay.
    ///
    /// This is passed into zlib; higher values generally result in
    /// smaller replays.
    ///
    /// Valid values are between 0 and 10, inclusively.
    #[arg(short = 'c', long, value_parser = clap::value_parser!(u8).range(0..=10), default_value_t = 7)]
    pub(super) compression_level: u8,
    /// An override for the input mode.
    ///
    /// If omitted, the input mode is inferred from the replay's
    /// metadata's version string.
    ///
    /// Inference is usually pretty accurate, but this
    /// override is useful if inference failed and you KNOW which mode you
    /// want to use. An example usecase is if you're using a mod of the game
    /// that changes up the version string and this library fails to parse it.
    ///
    /// Note that misuse may cause malformed replays that fail to play back
    /// properly. Handle with care!
    #[arg(long)]
    pub(super) override_input_mode: Option<CliInputMode>,
    #[command(flatten)]
    pub(super) io_args: IoArguments,
}

/// Arguments for the `base64ify` command.
#[derive(Args, Clone, Debug)]
pub struct Base64ifyArguments {
    #[command(flatten)]
    pub(super) io_args: IoArguments,
}

/// Arguments for the `binaryify` command.
#[derive(Args, Clone, Debug)]
pub struct BinaryifyArguments {
    #[command(flatten)]
    pub(super) io_args: IoArguments,
    /// Allow writing binary to the console.
    #[arg(long)]
    pub(super) skip_console_check: bool,
}

/// Arguments for the `shrink` command.
#[derive(Args, Clone, Debug)]
pub struct ShrinkArguments {
    #[command(flatten)]
    pub(super) io_args: IoArguments,
}

/// Extract something from the replay.
#[derive(Clone, Debug, Subcommand)]
pub enum ExtractMode {
    /// Extract everything from the replay; the metadata and the input data.
    ///
    /// Example: {"metadata":{...},"inputs":[...]}
    All,
    /// Extract just the metadata from the replay.
    ///
    /// Example: {"mode": "sprint_10l"}
    Metadata,
    /// Extract just the player input data from the replay.
    ///
    /// Example: [{"frame":0,"type":1,"key":16},{"frame":182,"type":1,"key":1}]
    Inputs,
}

impl ExtractMode {
    /// Returns a `(keep_metadata, keep_inputs)` tuple based on the mode.
    pub(crate) const fn to_keeps(&self) -> (bool, bool) {
        match self {
            Self::All => (true, true),
            Self::Metadata => (true, false),
            Self::Inputs => (false, true),
        }
    }

    /// Returns the starting header for the mode.
    pub(crate) const fn header(&self) -> Option<&'static [u8]> {
        match self {
            Self::All => Some(br#"{"metadata":"#),
            Self::Metadata => None,
            Self::Inputs => Some(b"["),
        }
    }

    /// Returns the ending footer for the mode.
    pub(crate) const fn footer(&self) -> Option<&'static [u8]> {
        match self {
            Self::All => Some(b"]}"),
            Self::Metadata => None,
            Self::Inputs => Some(b"]"),
        }
    }
}
