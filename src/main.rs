use std::io;

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
            techmino_replay_toolkit::GameReplayData::try_from_base64(&input.trim(), None)
        );
    }
}
