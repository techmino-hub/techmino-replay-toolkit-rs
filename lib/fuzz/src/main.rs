use std::io::prelude::Write as _;
use strum::{FromRepr, IntoStaticStr, VariantArray};

#[derive(Clone, Copy, Debug, PartialEq, Eq, VariantArray, IntoStaticStr, FromRepr)]
enum Action {
    Run,
}

impl Action {
    /// Ask the user for an action
    fn get() -> Self {
        println!("actions available:");

        for (index, variant) in Self::VARIANTS.iter().enumerate() {
            println!("\t[{}]: {}", index, <&str>::from(variant));
        }
        println!();

        loop {
            print!("choose one: ");
            std::io::stdout()
                .lock()
                .flush()
                .expect("flushing stdout should work");

            let mut buf = String::new();

            std::io::stdin()
                .read_line(&mut buf)
                .expect("reading from stdin should work");

            let Ok(number): Result<usize, _> = buf.trim().parse() else {
                println!("not a number!");
                continue;
            };

            let Some(&variant) = Self::VARIANTS.get(number) else {
                println!("out of range!");
                continue;
            };

            return variant;
        }
    }

    /// Act according to the action.
    fn act(self) {
        match self {
            Self::Run => runner::run(),
        }
    }
}

fn main() {
    println!("welcome to the libtechmino-replay fuzzer helper executable");
    Action::get().act();
}

mod runner {
    use arbitrary::{Arbitrary, Unstructured};
    use libtechmino_replay_fuzz::EncodeStream;
    use std::{ffi::OsString, fs, io::prelude::Write, path::PathBuf};
    use strum::{FromRepr, IntoStaticStr, VariantArray};

    pub fn run() {
        let target = Target::get();
        let data = data_chooser(target);

        core::hint::black_box(run_target)(target, &data);
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq, VariantArray, IntoStaticStr, FromRepr)]
    pub enum Target {
        FuzzEncode,
    }

    impl Target {
        /// Gets the specific subdirectory name for this target.
        pub const fn subdir_name(self) -> &'static str {
            match self {
                Self::FuzzEncode => "fuzz_encode",
            }
        }

        /// Ask the user for a target
        fn get() -> Self {
            println!("choose which fuzz target to test:\n");

            for (index, variant) in Self::VARIANTS.iter().enumerate() {
                println!("\t[{}]: {}", index, <&str>::from(variant));
            }
            println!();

            loop {
                print!("choose one: ");
                std::io::stdout()
                    .lock()
                    .flush()
                    .expect("flushing stdout should work");

                let mut buf = String::new();

                std::io::stdin()
                    .read_line(&mut buf)
                    .expect("reading from stdin should work");

                let Ok(number): Result<usize, _> = buf.trim().parse() else {
                    println!("not a number!");
                    continue;
                };

                let Some(&variant) = Self::VARIANTS.get(number) else {
                    println!("out of range!");
                    continue;
                };

                return variant;
            }
        }
    }

    fn data_chooser(target: Target) -> Box<[u8]> {
        let rel_dir = get_rel_dir(target);

        println!(
            "choose a testcase data file. relative to '{rel_dir:?}' (or you can use an absolute path)",
        );
        loop {
            print!("file name/path: ");
            std::io::stdout()
                .lock()
                .flush()
                .expect("flushing stdout should work");

            let mut buf = String::new();

            std::io::stdin()
                .read_line(&mut buf)
                .expect("reading from stdin should work");

            let file_path = rel_dir.join(buf.trim());

            let contents = match fs::read(&file_path) {
                Ok(contents) => contents,
                Err(e) => {
                    eprintln!("error accessing '{file_path:?}': {e}");
                    continue;
                }
            };

            return contents.into_boxed_slice();
        }
    }

    fn get_rel_dir(target: Target) -> PathBuf {
        if let Some(artifact_dir) = try_find_relevant_artifact_dir(target) {
            return artifact_dir;
        }

        if let Ok(cwd) = std::env::current_dir() {
            return cwd;
        }

        PathBuf::new()
    }

    fn try_find_relevant_artifact_dir(target: Target) -> Option<PathBuf> {
        /// The fuzzer artifacts/ directory relative to the repo's root.
        static ARTIFACTS_DIR: &str = "lib/fuzz/artifacts/";

        let exe = std::env::current_exe().ok()?;
        let mut target_dir = None;
        let target_str = OsString::from("target");

        for ancestor in exe.ancestors() {
            let Some(filename) = ancestor.file_name() else {
                continue;
            };

            if filename == target_str {
                target_dir = Some(ancestor);
                break;
            }
        }

        let target_dir = target_dir?;
        let repo_dir = target_dir.parent()?;
        let artifacts_dir = repo_dir.join(ARTIFACTS_DIR);
        let artifact_dir = artifacts_dir.join(target.subdir_name());
        Some(artifact_dir)
    }

    fn run_target(target: Target, data: &[u8]) {
        println!("running test!");

        match target {
            Target::FuzzEncode => run_encode_case(data),
        }

        println!("test succeeded");
    }

    fn run_encode_case(data: &[u8]) {
        let mut unstructured = Unstructured::new(data);

        let stream = EncodeStream::arbitrary(&mut unstructured)
            .expect("arbitrary data should be valid encode stream");

        stream.test().expect("stream test should succeed");
    }
}
