//! Fuzzing focused on the encoder, with a single decode step at the end to make
//! sure it roundtrips correctly.
#![no_main]

use libfuzzer_sys::fuzz_target;
use libtechmino_replay_fuzz::EncodeStream;

fuzz_target!(|data: EncodeStream| {
    let _result = data.test();
});
