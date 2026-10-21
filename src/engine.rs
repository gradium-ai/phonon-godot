//! The Godot-free part: loading a model and running generations on worker threads.
//!
//! Nothing here touches the Godot API, so all of it can run off the main thread. Results go
//! back over channels that `PhononSpeaker` polls from its `_process`.

use ptts::checkpoint::{Checkpoint, ResolveOptions};
use ptts::preprocess::Normalize;
use ptts::synth::{DeviceKind, Quant, SpeechOptions, Synth};
use ptts::voice_catalog::VoiceCatalog;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Sender, SyncSender};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant};

/// What a load needs, copied from the node's properties on the main thread.
pub struct Settings {
    pub model: String,
    pub language: String,
    pub voice: String,
    pub quant: String,
}

/// A loaded model, shared by the node and its worker threads.
pub struct Model {
    synth: Mutex<Synth>,
    catalog: VoiceCatalog,
    /// The model's own voices, then the catalog's voices in the model's language.
    pub voices: Vec<String>,
    pub sample_rate: u32,
    /// Where it runs and in which weight format, e.g. `cpu, q8_0`.
    pub description: String,
    /// Problems worth a warning that did not stop the load.
    pub warnings: Vec<String>,
}

impl Model {
    /// Load the checkpoint and its voices. Blocks: downloads on first use, then reads the weights.
    pub fn load(settings: &Settings) -> Result<Self, String> {
        let normalize = settings.language.parse::<Normalize>().map_err(message)?;
        let quant = settings.quant.parse::<Quant>().map_err(message)?;
        // Fails fast, before anything is downloaded, for a combination this build cannot run.
        quant.check_device(DeviceKind::Auto).map_err(message)?;
        let options = ResolveOptions { quant, weights: None };
        let checkpoint = Checkpoint::locate(&settings.model, options).map_err(message)?;
        let mut synth = checkpoint.builder(normalize).build().map_err(message)?;
        checkpoint.register_voices(&mut synth);

        let mut warnings = vec![];
        if let Some(warning) = normalize.language_warning(synth.languages()) {
            warnings.push(warning);
        }
        let catalog =
            VoiceCatalog::new(ptts::voice_catalog::DEFAULT_VOICE_CATALOG).map_err(message)?;
        let mut voices = synth.voices();
        match catalog.voices() {
            Ok(listed) => {
                let lang = (normalize != Normalize::OFF).then(|| normalize.as_str());
                for voice in listed {
                    let same_lang = lang.is_none() || voice.language.as_deref() == lang;
                    if same_lang && !voices.contains(&voice.name) {
                        voices.push(voice.name.clone());
                    }
                }
            }
            Err(e) => warnings.push(format!("cannot read the voice catalog: {e}")),
        }
        let sample_rate = synth.sample_rate();
        let description = format!("{}, {}", synth.device_name(), synth.quant().as_str());
        let model =
            Self { synth: Mutex::new(synth), catalog, voices, sample_rate, description, warnings };
        // A voice that does not exist fails the load rather than the first line spoken.
        model.options(&settings.voice)?;
        Ok(model)
    }

    fn synth(&self) -> MutexGuard<'_, Synth> {
        // A panic while holding the lock leaves nothing half-written that later calls depend on.
        self.synth.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Options for one request in `voice`, downloading it from the catalog on first use. An
    /// empty name keeps the model's default voice.
    fn options(&self, voice: &str) -> Result<SpeechOptions, String> {
        if voice.is_empty() {
            return Ok(SpeechOptions::default());
        }
        let mut synth = self.synth();
        let name = self.catalog.register(&mut synth, voice).map_err(message)?;
        if !synth.voices().contains(&name) {
            return Err(format!(
                "no voice '{voice}'; voices() lists the voices of this model and language"
            ));
        }
        Ok(SpeechOptions::default().voice(name))
    }
}

/// Messages from a speech worker to the node.
pub enum SpeechEvent {
    Audio(Vec<f32>),
    Done,
    Failed(String),
}

/// Stream `text` to `tx` chunk by chunk until it ends, `cancel` is set, or the receiver goes
/// away. The channel is bounded, so generation pauses while the node is not consuming.
pub fn speak(
    model: &Model,
    text: &str,
    voice: &str,
    cancel: &AtomicBool,
    tx: &SyncSender<SpeechEvent>,
) {
    let result = (|| {
        let options = model.options(voice)?;
        let stream = model.synth().stream_with(text, &options).map_err(message)?;
        for chunk in stream {
            if cancel.load(Ordering::Acquire) {
                return Ok(());
            }
            if tx.send(SpeechEvent::Audio(chunk.map_err(message)?)).is_err() {
                return Ok(());
            }
        }
        Ok(())
    })();
    // Dropping the stream above stopped and joined the generation threads, here rather than
    // on the main thread. Nobody is listening any more if this send fails.
    let _ = tx.send(match result {
        Ok(()) => SpeechEvent::Done,
        Err(e) => SpeechEvent::Failed(e),
    });
}

/// A finished `synthesize` request: its id and its audio or error.
pub type Synthesized = (i64, Result<Vec<f32>, String>);

/// Generate all of `text` and send it to `tx` as request `id`, unless `cancel` is set first.
pub fn synthesize(
    model: &Model,
    id: i64,
    text: &str,
    voice: &str,
    cancel: &AtomicBool,
    tx: &Sender<Synthesized>,
) {
    let result = (|| {
        let options = model.options(voice)?;
        let stream = model.synth().stream_with(text, &options).map_err(message)?;
        let mut pcm = vec![];
        for chunk in stream {
            if cancel.load(Ordering::Acquire) {
                return Ok(None);
            }
            pcm.extend_from_slice(&chunk.map_err(message)?);
        }
        Ok(Some(pcm))
    })();
    if let Some(result) = result.transpose() {
        let _ = tx.send((id, result));
    }
}

fn message(e: ptts::Error) -> String {
    e.to_string()
}

/// Worker threads still running, so the library is not unloaded under one at exit.
static LIVE: (Mutex<usize>, Condvar) = (Mutex::new(0), Condvar::new());

struct Live;

impl Drop for Live {
    fn drop(&mut self) {
        let mut live = LIVE.0.lock().unwrap_or_else(|e| e.into_inner());
        *live -= 1;
        LIVE.1.notify_all();
    }
}

/// Start a named worker thread that counts as live until it returns.
pub fn spawn(name: &str, work: impl FnOnce() + Send + 'static) -> Result<(), String> {
    *LIVE.0.lock().unwrap_or_else(|e| e.into_inner()) += 1;
    let guard = Live;
    std::thread::Builder::new()
        .name(name.to_string())
        .spawn(move || {
            let _guard = guard;
            work();
        })
        .map(drop)
        .map_err(|e| format!("cannot start a thread: {e}"))
}

/// Wait up to `timeout` for every worker thread to return. Cancelled generations stop within
/// a frame; a download in progress may not, which is what the timeout is for.
pub fn wait_for_workers(timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    let mut live = LIVE.0.lock().unwrap_or_else(|e| e.into_inner());
    while *live > 0 {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return false;
        }
        live = LIVE.1.wait_timeout(live, left).unwrap_or_else(|e| e.into_inner()).0;
    }
    true
}

/// A flag the node sets to stop a worker, shared with that worker.
pub type Cancel = Arc<AtomicBool>;
