//! Phonon text-to-speech for Godot.
//!
//! Registers the `PhononSpeaker` node. See the README for its API.

mod engine;
mod speaker;

use godot::init::{ExtensionLibrary, InitStage, gdextension};
use std::time::Duration;

struct Phonon;

#[gdextension]
unsafe impl ExtensionLibrary for Phonon {
    fn on_stage_deinit(stage: InitStage) {
        // Freed speakers have cancelled their work by now. Wait for the threads to return, so
        // none is still running this library's code when Godot unloads it.
        if stage == InitStage::Scene && !engine::wait_for_workers(Duration::from_secs(5)) {
            godot::global::godot_warn!("Phonon: a worker thread was still running at exit");
        }
    }
}
