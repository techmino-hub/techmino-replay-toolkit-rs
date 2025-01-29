use base64::{engine::general_purpose::STANDARD as b64, Engine};
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

        println!("{:?}", techmino_replay_toolkit::parse_base64(&input.trim(), None));

        // let res = b64
        //     .decode(input.trim())
        //     .and_then(|it| Ok(miniz_oxide::inflate::decompress_to_vec_zlib(&it)));

        // match res {
        //     Ok(Ok(d)) => println!("{d:?}"),
        //     Ok(Err(e)) => println!("Decompression error! {e:?}"),
        //     Err(e) => println!("Base64 error! {e:?}"),
        // }
    }
}
