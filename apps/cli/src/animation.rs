//! The logo animation frames, generated from the ascii-motion frames export.
//!
//! piramid serve prints them before it starts, and the console plays them as its splash screen.

include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/ascii_motion_frames.rs_inc"
));
