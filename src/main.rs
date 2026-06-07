use std::io;

use techmino_replay_toolkit::{format::ReplayBufferKind, GameReplayData};

fn main() {
    println!(
        "\
        ╭~~~~~~~~~~~~~╮  \n\
        ┊ ▀▀█▀▀    █  ┊  Techmino Replay Toolkit\n\
        ┊   █  █▀█ █▀ ┊  v{version}\n\
        ┊   █  █   █▄ ┊  https://github.com/techmino-hub/techmino-replay-toolkit-rs\n\
        ╰~~~~~~~~~~~~~╯  \n\
        This program and library is licensed under the GNU General Public License version 3.\n\
        For more information, see <https://www.gnu.org/licenses/>.\n",
        version = env!("CARGO_PKG_VERSION")
    );

    loop {
        eprintln!("Paste the game replay string below:");

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .expect("Failed to read from stdin");

        println!(
            "{:?}",
            GameReplayData::parse_replay(input.trim().as_bytes(), ReplayBufferKind::Base64, None),
        );
    }
}
