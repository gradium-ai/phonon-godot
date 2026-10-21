//! The `PhononSpeaker` node.

// The `init` that `#[class(init)]` generates writes `base: base`, which clippy flags from 1.99.
#![allow(clippy::redundant_field_names)]

use crate::engine::{self, Cancel, Model, Settings, SpeechEvent, Synthesized};
use godot::classes::audio_stream_generator::AudioStreamGeneratorMixRate;
use godot::classes::audio_stream_wav::Format;
use godot::classes::node::InternalMode;
use godot::classes::{
    AudioStreamGenerator, AudioStreamGeneratorPlayback, AudioStreamPlayer, AudioStreamPlayer2D,
    AudioStreamPlayer3D, AudioStreamWav, INode, Os, ProjectSettings,
};
use godot::prelude::*;
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::sync::{Arc, mpsc};

/// Chunks a speech worker may queue ahead of playback before generation pauses.
const SPEECH_QUEUE: usize = 8;

/// Seconds of audio the playback buffer holds, enough to ride out a slow frame.
const PLAYBACK_BUFFER_SECONDS: f32 = 1.0;

/// Speaks text with Phonon, Gradium's on-device text-to-speech model.
///
/// Call [method load_model] once, wait for [signal loaded], then call [method speak]. Audio
/// starts while the rest of the line is still being generated. Loading and generation run on
/// worker threads, so no method blocks the main thread.
///
/// The model is downloaded from Hugging Face on first use. Exported games keep it under
/// [code]user://[/code]; the editor uses the shared Hugging Face cache.
#[derive(GodotClass)]
#[class(base = Node, init)]
pub struct PhononSpeaker {
    /// The model: a Hugging Face repo as [code]hf://owner/name[/code] or
    /// [code]hf://owner/name@revision[/code], or a local folder. The default is Phonon 31m,
    /// English, under Apache 2.0. Takes effect at the next [method load_model].
    #[export]
    #[init(val = GString::from(ptts::checkpoint::DEFAULT_MODEL))]
    model: GString,

    /// Language the text is normalized as before it is spoken, so that numbers, dates and
    /// symbols are read correctly. [code]none[/code] turns normalization off. Takes effect at
    /// the next [method load_model].
    #[export(enum = (en, fr, de, es, pt, none))]
    #[init(val = GString::from("en"))]
    language: GString,

    /// Voice to speak in: one of [method voices]. Empty uses the model's default voice. Read at
    /// each [method speak] and [method synthesize], so it can change between lines.
    #[export]
    voice: GString,

    /// Weight format: [code]q8[/code] is smaller and runs on the CPU, [code]f32[/code] uses the
    /// GPU where the build supports it. Takes effect at the next [method load_model].
    #[export(enum = (q8, f32))]
    #[init(val = GString::from("q8"))]
    quant: GString,

    /// Player for the speech: an [AudioStreamPlayer], [AudioStreamPlayer2D] or
    /// [AudioStreamPlayer3D]. Empty plays through an internal [AudioStreamPlayer].
    #[export(node_path = ("AudioStreamPlayer", "AudioStreamPlayer2D", "AudioStreamPlayer3D"))]
    player: NodePath,

    loaded: Option<Arc<Model>>,
    loading: Option<Receiver<Result<Model, String>>>,
    speech: Option<Speech>,
    requests: Requests,
    /// Errors from calls that failed at once, emitted from the next frame.
    deferred_errors: Vec<String>,
    own_player: Option<Gd<AudioStreamPlayer>>,

    base: Base<Node>,
}

#[godot_api]
impl PhononSpeaker {
    /// The model loaded. [method speak] and [method synthesize] can now be called.
    #[signal]
    fn loaded();

    /// The model did not load, for the reason given.
    #[signal]
    fn load_failed(message: GString);

    /// The first audio of a [method speak] call started playing.
    #[signal]
    fn started();

    /// A [method speak] call finished playing. Not emitted after [method stop].
    #[signal]
    fn finished();

    /// A [method speak] or [method synthesize] call failed, for the reason given.
    #[signal]
    fn error(message: GString);

    /// A [method synthesize] request finished. [param audio] is null if it failed, in which case
    /// [signal error] is emitted too.
    #[signal]
    fn synthesized(id: i64, audio: Option<Gd<AudioStreamWav>>);

    /// Load the model given by the properties, on a worker thread. Emits [signal loaded] or
    /// [signal load_failed]. The first load downloads the model. Calling it again reloads with
    /// the current properties; the current model stays in use until the new one is ready.
    #[func]
    fn load_model(&mut self) {
        use_user_cache_in_exported_games();
        let settings = Settings {
            model: local_path(&self.model),
            language: self.language.to_string(),
            voice: self.voice.to_string(),
            quant: self.quant.to_string(),
        };
        let (tx, rx) = mpsc::channel();
        let failed = tx.clone();
        if let Err(e) = engine::spawn("phonon-load", move || {
            let _ = tx.send(Model::load(&settings));
        }) {
            let _ = failed.send(Err(e));
        }
        self.loading = Some(rx);
    }

    /// True once a model is loaded.
    #[func]
    fn is_loaded(&self) -> bool {
        self.loaded.is_some()
    }

    /// True from [method speak] until the line has played out or [method stop] is called.
    #[func]
    fn is_speaking(&self) -> bool {
        self.speech.is_some()
    }

    /// The voices [member voice] can name: the model's own, then those of the voice catalog in
    /// [member language]. Catalog voices download on first use. Empty until the model is loaded.
    #[func]
    fn voices(&self) -> PackedStringArray {
        self.loaded.iter().flat_map(|model| model.voices.iter()).map(GString::from).collect()
    }

    /// Speak [param text] in [member voice]. Playback starts as soon as the first audio is
    /// generated, and [signal finished] follows once all of it has played. A line already being
    /// spoken is stopped first.
    #[func]
    fn speak(&mut self, text: GString) {
        self.stop();
        if let Err(e) = self.start_speech(text.to_string()) {
            self.fail(e);
        }
    }

    /// Stop speaking at once. Generation stops within a frame of audio.
    #[func]
    fn stop(&mut self) {
        if let Some(mut speech) = self.speech.take() {
            speech.cancel.store(true, Ordering::Release);
            speech.player.stop();
        }
    }

    /// Generate all of [param text] in [member voice] without playing it, for lines to have ready
    /// ahead of time. Returns a request id at once; [signal synthesized] follows with that id and
    /// an [AudioStreamWAV] at the model's sample rate.
    #[func]
    fn synthesize(&mut self, text: GString) -> i64 {
        let id = self.requests.next_id;
        self.requests.next_id += 1;
        let Some(model) = self.loaded.clone() else {
            let _ = self.requests.tx.send((id, Err(NOT_LOADED.into())));
            return id;
        };
        let cancel = Cancel::default();
        let tx = self.requests.tx.clone();
        let (text, voice) = (text.to_string(), self.voice.to_string());
        let job = cancel.clone();
        let spawned = engine::spawn("phonon-synthesize", move || {
            engine::synthesize(&model, id, &text, &voice, &job, &tx);
        });
        match spawned {
            Ok(()) => drop(self.requests.cancels.insert(id, cancel)),
            Err(e) => drop(self.requests.tx.send((id, Err(e)))),
        }
        id
    }
}

#[godot_api]
impl INode for PhononSpeaker {
    fn process(&mut self, _delta: f64) {
        for message in std::mem::take(&mut self.deferred_errors) {
            self.signals().error().emit(&GString::from(&message));
        }
        self.poll_load();
        self.poll_requests();
        self.feed_speech();
    }
}

const NOT_LOADED: &str = "the model is not loaded: call load_model() and wait for `loaded`";

impl PhononSpeaker {
    fn fail(&mut self, message: String) {
        godot_error!("Phonon: {message}");
        self.deferred_errors.push(message);
    }

    fn poll_load(&mut self) {
        let Some(rx) = &self.loading else { return };
        let result = match rx.try_recv() {
            Ok(result) => result,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => Err("loading stopped unexpectedly".to_string()),
        };
        self.loading = None;
        match result {
            Ok(model) => {
                for warning in &model.warnings {
                    godot_warn!("Phonon: {warning}");
                }
                let line = format!("Phonon: loaded {} ({})", self.model, model.description);
                godot::global::print_verbose(&[line.to_variant()]);
                self.loaded = Some(Arc::new(model));
                self.signals().loaded().emit();
            }
            Err(e) => {
                godot_error!("Phonon: cannot load {}: {e}", self.model);
                self.signals().load_failed().emit(&GString::from(&e));
            }
        }
    }

    fn poll_requests(&mut self) {
        while let Ok((id, result)) = self.requests.rx.try_recv() {
            self.requests.cancels.remove(&id);
            let sample_rate = self.loaded.as_ref().map_or(24_000, |model| model.sample_rate);
            match result {
                Ok(pcm) => {
                    let audio = wav(&pcm, sample_rate);
                    self.signals().synthesized().emit(id, Some(&audio));
                }
                Err(e) => {
                    godot_error!("Phonon: {e}");
                    self.signals().error().emit(&GString::from(&e));
                    self.signals().synthesized().emit(id, None::<&Gd<AudioStreamWav>>);
                }
            }
        }
    }

    fn start_speech(&mut self, text: String) -> Result<(), String> {
        let model = self.loaded.clone().ok_or(NOT_LOADED)?;
        if !self.base().is_inside_tree() {
            return Err("speak() needs the speaker to be inside the scene tree".into());
        }
        let mut player = self.resolve_player()?;
        let mut generator = AudioStreamGenerator::new_gd();
        generator.set_mix_rate_mode(AudioStreamGeneratorMixRate::CUSTOM);
        generator.set_mix_rate(model.sample_rate as f32);
        generator.set_buffer_length(PLAYBACK_BUFFER_SECONDS);
        let playback = player.start(generator).ok_or("the player did not start playback")?;
        let empty = playback.get_frames_available();

        let (tx, rx) = mpsc::sync_channel(SPEECH_QUEUE);
        let cancel = Cancel::default();
        let job = cancel.clone();
        let voice = self.voice.to_string();
        let spawned = engine::spawn("phonon-speak", move || {
            engine::speak(&model, &text, &voice, &job, &tx);
        });
        if let Err(e) = spawned {
            player.stop();
            return Err(e);
        }
        self.speech = Some(Speech {
            rx,
            cancel,
            player,
            playback,
            empty,
            pending: vec![],
            offset: 0,
            started: false,
            generated: false,
        });
        Ok(())
    }

    /// The player named by [member player], or the internal one.
    fn resolve_player(&mut self) -> Result<Player, String> {
        if !self.player.is_empty() {
            let node = self
                .base()
                .get_node_or_null(&self.player)
                .ok_or_else(|| format!("no player at {}", self.player))?;
            return Player::from_node(node).ok_or_else(|| {
                format!(
                    "{} is not an AudioStreamPlayer, AudioStreamPlayer2D or AudioStreamPlayer3D",
                    self.player
                )
            });
        }
        if let Some(player) = self.own_player.as_ref().filter(|p| p.is_instance_valid()) {
            return Ok(Player::Flat(player.clone()));
        }
        let mut player = AudioStreamPlayer::new_alloc();
        player.set_name("PhononPlayer");
        self.base_mut().add_child_ex(&player).internal(InternalMode::FRONT).done();
        self.own_player = Some(player.clone());
        Ok(Player::Flat(player))
    }

    /// Move generated audio into the playback buffer, as much as fits.
    fn feed_speech(&mut self) {
        let Some(speech) = self.speech.as_mut() else { return };
        let mut room = speech.playback.get_frames_available().max(0) as usize;
        let mut started = false;
        let mut failure = None;
        loop {
            if speech.offset < speech.pending.len() {
                let n = room.min(speech.pending.len() - speech.offset);
                if n == 0 {
                    break;
                }
                let frames: PackedVector2Array = speech.pending[speech.offset..speech.offset + n]
                    .iter()
                    .map(|&s| Vector2::new(s, s))
                    .collect();
                speech.playback.push_buffer(&frames);
                speech.offset += n;
                room -= n;
                if !speech.started {
                    speech.started = true;
                    started = true;
                }
                continue;
            }
            if speech.generated {
                break;
            }
            match speech.rx.try_recv() {
                Ok(SpeechEvent::Audio(pcm)) => {
                    speech.pending = pcm;
                    speech.offset = 0;
                }
                Ok(SpeechEvent::Done) => speech.generated = true,
                Ok(SpeechEvent::Failed(e)) => {
                    failure = Some(e);
                    break;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    failure = Some("speech generation stopped unexpectedly".into());
                    break;
                }
            }
        }
        let played_out = speech.generated
            && speech.offset >= speech.pending.len()
            && speech.playback.get_frames_available() >= speech.empty;

        // Signals go last: a handler may call speak() or stop() and replace the speech.
        if started {
            self.signals().started().emit();
        }
        if let Some(e) = failure {
            self.stop();
            godot_error!("Phonon: {e}");
            self.signals().error().emit(&GString::from(&e));
        } else if played_out {
            self.stop();
            self.signals().finished().emit();
        }
    }
}

impl Drop for PhononSpeaker {
    fn drop(&mut self) {
        if let Some(speech) = self.speech.take() {
            speech.cancel.store(true, Ordering::Release);
        }
        for cancel in self.requests.cancels.values() {
            cancel.store(true, Ordering::Release);
        }
    }
}

/// A line being spoken.
struct Speech {
    rx: Receiver<SpeechEvent>,
    cancel: Cancel,
    player: Player,
    playback: Gd<AudioStreamGeneratorPlayback>,
    /// Frames available in the playback buffer when it is empty.
    empty: i32,
    /// The chunk being pushed, and how much of it has been.
    pending: Vec<f32>,
    offset: usize,
    started: bool,
    /// The worker has sent all of its audio.
    generated: bool,
}

/// `synthesize` requests in flight.
struct Requests {
    tx: Sender<Synthesized>,
    rx: Receiver<Synthesized>,
    cancels: HashMap<i64, Cancel>,
    next_id: i64,
}

impl Default for Requests {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel();
        Self { tx, rx, cancels: HashMap::new(), next_id: 1 }
    }
}

/// The three players speech can go through. They share no base class with these methods.
enum Player {
    Flat(Gd<AudioStreamPlayer>),
    Planar(Gd<AudioStreamPlayer2D>),
    Spatial(Gd<AudioStreamPlayer3D>),
}

impl Player {
    fn from_node(node: Gd<Node>) -> Option<Self> {
        let node = match node.try_cast::<AudioStreamPlayer>() {
            Ok(player) => return Some(Self::Flat(player)),
            Err(node) => node,
        };
        let node = match node.try_cast::<AudioStreamPlayer2D>() {
            Ok(player) => return Some(Self::Planar(player)),
            Err(node) => node,
        };
        node.try_cast::<AudioStreamPlayer3D>().ok().map(Self::Spatial)
    }

    /// Play `stream` and return its playback.
    fn start(
        &mut self,
        stream: Gd<AudioStreamGenerator>,
    ) -> Option<Gd<AudioStreamGeneratorPlayback>> {
        let playback = match self {
            Self::Flat(p) => {
                p.set_stream(&stream);
                p.play();
                p.get_stream_playback()
            }
            Self::Planar(p) => {
                p.set_stream(&stream);
                p.play();
                p.get_stream_playback()
            }
            Self::Spatial(p) => {
                p.set_stream(&stream);
                p.play();
                p.get_stream_playback()
            }
        };
        playback?.try_cast().ok()
    }

    fn stop(&mut self) {
        match self {
            Self::Flat(p) if p.is_instance_valid() => p.stop(),
            Self::Planar(p) if p.is_instance_valid() => p.stop(),
            Self::Spatial(p) if p.is_instance_valid() => p.stop(),
            _ => {}
        }
    }
}

/// `pcm` as a 16-bit mono [AudioStreamWAV].
fn wav(pcm: &[f32], sample_rate: u32) -> Gd<AudioStreamWav> {
    let bytes: PackedByteArray = pcm
        .iter()
        .flat_map(|&s| ((s.clamp(-1.0, 1.0) * i16::MAX as f32).round() as i16).to_le_bytes())
        .collect();
    let mut audio = AudioStreamWav::new_gd();
    audio.set_format(Format::FORMAT_16_BITS);
    audio.set_mix_rate(sample_rate as i32);
    audio.set_stereo(false);
    audio.set_data(&bytes);
    audio
}

/// A `res://` or `user://` model folder as a filesystem path; anything else unchanged.
fn local_path(model: &GString) -> String {
    let text = model.to_string();
    if text.starts_with("res://") || text.starts_with("user://") {
        ProjectSettings::singleton().globalize_path(model).to_string()
    } else {
        text
    }
}

/// Point the Hugging Face cache at `user://` in exported games, unless `HF_HUB_CACHE` says
/// otherwise. The editor keeps the shared cache, so models download once per machine there.
fn use_user_cache_in_exported_games() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        if std::env::var_os("HF_HUB_CACHE").is_some() || Os::singleton().has_feature("editor") {
            return;
        }
        let dir = ProjectSettings::singleton().globalize_path("user://phonon/huggingface");
        // SAFETY: this runs once, on the main thread, before the first worker thread of this
        // extension starts, and the variable is only read by the downloads those threads make.
        unsafe { std::env::set_var("HF_HUB_CACHE", dir.to_string()) };
    });
}
