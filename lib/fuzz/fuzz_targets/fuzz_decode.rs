#![no_main]

use libfuzzer_sys::fuzz_target;
use libtechmino_replay_fuzz::DecodeStream;

fuzz_target!(|stream: DecodeStream| {
    stream.test();
});
