# Phonon for Godot

<img src="store/phonon-monogram.png" alt="Phonon" width="96">

Speak text in your Godot game with Phonon, Gradium's on-device text-to-speech model. The speech is generated on the player's machine. There is no server and no API key.

The plugin adds one node, `PhononSpeaker`. It is a GDExtension written in Rust, on top of the [`ptts`](https://crates.io/crates/ptts) runtime.

This is a first version. It runs on desktop: macOS, Linux and Windows. See [Platforms](#platforms).

## Install

Once the addon is on the [Godot Asset Store](https://store.godotengine.org/), install it from the editor's Asset Store tab. Until then:

1. Download `phonon-godot-<version>.zip` from a [release](../../releases), or the `phonon-godot-addon` artifact of the latest successful run of the [Build workflow](../../actions/workflows/build.yml) on `main`.
2. Extract it into your project folder, so that `addons/phonon/phonon.gdextension` is inside it.
3. Restart the editor. `PhononSpeaker` now appears in the Create New Node dialog.

On macOS, a downloaded zip marks the library as quarantined, and Godot cannot load it. Clear the flag once:

```sh
xattr -dr com.apple.quarantine addons/phonon/bin/macos
```

The plugin needs Godot 4.7 or later.

## Quick start

Add a `PhononSpeaker` node to your scene. Then, in a script:

```gdscript
extends Node

@onready var speaker: PhononSpeaker = $PhononSpeaker


func _ready() -> void:
	speaker.loaded.connect(func(): speaker.speak("Welcome back, traveler."))
	speaker.load_failed.connect(func(message: String): push_error(message))
	speaker.load_model()
```

The first `load_model()` downloads the default model, Phonon 31m. Later runs load it from the cache.

## PhononSpeaker

`PhononSpeaker` extends `Node`. None of its methods block the main thread. Loading and generation run on worker threads, and the results arrive as signals. The editor's help (F1, then search for `PhononSpeaker`) shows the same reference.

### Properties

| Property | Default | Description |
|---|---|---|
| `model` | `hf://gradium/phonon-31m-en@<commit>` | The model: a Hugging Face repo as `hf://owner/name` or `hf://owner/name@revision`, or a local folder. The default is Phonon 31m, pinned to one commit. |
| `language` | `en` | The language the text is normalized as, so that numbers, dates and symbols are read correctly: `en`, `fr`, `de`, `es`, `pt`, or `none` to turn normalization off. Use the language of your text. |
| `voice` | empty | The voice to speak in, one of `voices()`. Empty uses the model's default voice. |
| `quant` | `q8` | The weight format. `q8` is smaller and runs on the CPU. `f32` runs on the GPU on Apple silicon, and on the CPU elsewhere. |
| `player` | empty | An `AudioStreamPlayer`, `AudioStreamPlayer2D` or `AudioStreamPlayer3D` to play the speech through. Empty uses an internal `AudioStreamPlayer`. |

`model`, `language` and `quant` take effect at the next `load_model()`. `voice` is read at each `speak()` and `synthesize()`, so it can change from one line to the next.

### Methods

| Method | Description |
|---|---|
| `load_model()` | Loads the model on a worker thread. Emits `loaded` or `load_failed`. Call it again after changing `model`, `language` or `quant`. The current model stays in use until the new one is ready. |
| `speak(text: String)` | Speaks `text` in `voice`. Playback starts as soon as the first audio is ready, while the rest is still being generated. A line that is already playing is stopped first. |
| `stop()` | Stops speaking at once. |
| `synthesize(text: String) -> int` | Generates all of `text` without playing it, for lines you want ready ahead of time. Returns a request id at once. `synthesized` follows with the same id. |
| `voices() -> PackedStringArray` | The voices `voice` can name: the model's own voices, then the voices of the voice catalog in `language`. Empty until the model is loaded. |
| `is_loaded() -> bool` | True once a model is loaded. |
| `is_speaking() -> bool` | True from `speak()` until the line has played out or `stop()` is called. |

### Signals

| Signal | Description |
|---|---|
| `loaded()` | The model is ready. |
| `load_failed(message: String)` | The model did not load. |
| `started()` | The first audio of a `speak()` line is playing. |
| `finished()` | A `speak()` line has played out. It is not emitted after `stop()`. |
| `error(message: String)` | A `speak()` or `synthesize()` call failed. |
| `synthesized(id: int, audio: AudioStreamWAV)` | A `synthesize()` request is done. `audio` is null if it failed, and `error` is emitted too. |

Failures are also printed to the Output panel.

### Lines ready ahead of time

`synthesize()` returns an `AudioStreamWAV` at the model's sample rate. Play it with any player, or save it with `save_to_wav()`.

```gdscript
var lines := {}  # line name -> AudioStreamWAV
var pending := {}  # request id -> line name


func _ready() -> void:
	speaker.synthesized.connect(_on_synthesized)


func prepare(line: String, text: String) -> void:
	pending[speaker.synthesize(text)] = line


func _on_synthesized(id: int, audio: AudioStreamWAV) -> void:
	var line: String = pending.get(id, "")
	pending.erase(id)
	if audio:
		lines[line] = audio
```

### Speech in 2D or 3D

Set `player` to an `AudioStreamPlayer2D` or `AudioStreamPlayer3D` on the character, and the speech comes from where the character is. The speaker sets that player's `stream`, so give each speaker its own player.

## Models and licenses

| Model | `model` | Languages | License |
|---|---|---|---|
| Phonon 31m (the default) | `hf://gradium/phonon-31m-en` | English | Apache 2.0 |
| Phonon 40m | `hf://gradium/phonon-40m-en`, `-fr`, `-de`, `-es`, `-pt` | One language each | CC BY-NC 4.0 |
| Phonon 40m multilingual | `hf://gradium/phonon-40m-multilingual` | English, French, German, Spanish, Portuguese | CC BY-NC 4.0 |

Phonon 31m is the default. It is under Apache 2.0, so you can ship it in a commercial game.

The Phonon 40m models are under CC BY-NC 4.0, which does not allow commercial use. You can try them with the plugin, but a commercial game needs a license from Gradium to use them. To get one, contact Gradium. <!-- TODO: commercial contact --> The [model table on the Phonon 31m page](https://huggingface.co/gradium/phonon-31m-en#the-phonon-family) lists the license of each model.

The Phonon 40m models are also gated on Hugging Face. Accept the terms on the model's page, then set `HF_TOKEN` or run `hf auth login` before you load one. Players have no token, so a game cannot download a gated model for them.

Set `language` to the language of the model and of your text. Phonon 31m speaks English.

The default voice comes with the model. The other voices come from the voice catalog [`gradium/phonon-voices`](https://huggingface.co/gradium/phonon-voices). Each one downloads the first time it is used. The voice catalog is under Apache 2.0, as its Hugging Face page states.

To get the same files on every machine, pin a commit: `hf://gradium/phonon-31m-en@<commit>`. The default model is pinned.

## Model download and cache

The first `load_model()` downloads the model from Hugging Face. Only the files it uses are downloaded. Later loads find them in the cache.

- In the editor, and when you run the game from the editor, files go to the shared Hugging Face cache (`~/.cache/huggingface/hub`), as with the `ptts` command and Python package. A model downloads once per machine.
- In an exported game, files go to `user://phonon/huggingface`, in the game's own data folder. Nothing is written to the player's shared Hugging Face cache, and deleting the game's data deletes the model too.
- The `HF_HUB_CACHE` environment variable overrides both.

A game can also ship the model so that it needs no download. Put the files of the model's Hugging Face repo in a folder beside the game, and set `model` to that folder's absolute path, for example `OS.get_executable_path().get_base_dir().path_join("phonon-31m-en")`. A `user://` folder works too. A `res://` folder works only in the editor, because an exported game packs `res://` into one file that the runtime cannot read models from. Voices from the catalog still download the first time they are used.

## Platforms

| Platform | Library |
|---|---|
| macOS 15 or later, Apple silicon and Intel | `bin/macos/libphonon_godot.dylib`, one universal library |
| Linux x86_64 | `bin/linux/libphonon_godot.x86_64.so` |
| Windows x86_64 | `bin/windows/phonon_godot.x86_64.dll` |

The x86_64 libraries need a CPU with AVX2 (x86-64-v3). The Linux library is built on Ubuntu 22.04, so it needs glibc 2.35 or later.

Android, iOS and web exports are not supported in this version. The extension has no library for them.

## Demo

`demo/` is a Godot project with a text box, a voice list, and Speak and Stop buttons. It finds the addon through `demo/addons/phonon`, a symbolic link to `addons/phonon`. On Windows, enable symbolic links in Git, or copy the folder there.

Build the library (see below), then open `demo/project.godot` in Godot and run it.

## Building from source

You need a recent stable Rust. Build the library and copy it where the `.gdextension` file looks for it:

```sh
cargo build --release
mkdir -p addons/phonon/bin/macos
cp target/release/libphonon_godot.dylib addons/phonon/bin/macos/
```

On Linux, copy `target/release/libphonon_godot.so` to `addons/phonon/bin/linux/libphonon_godot.x86_64.so`. On Windows, copy `target\release\phonon_godot.dll` to `addons\phonon\bin\windows\phonon_godot.x86_64.dll`.

The build uses the same settings as the `ptts` release builds. `.cargo/config.toml` sets fixed CPU baselines, never `target-cpu=native`, because the libraries run on players' machines. On macOS, `Cargo.toml` turns on Accelerate, and on Apple silicon also Metal and Arm's KleidiAI kernels.

### Smoke test

`demo/tests/smoke_test.gd` loads the default model, writes WAV files of synthesized and spoken lines, and checks the signals of `speak()` and `stop()`. It runs headless:

```sh
godot --headless --path demo --import
godot --headless --path demo --script res://tests/smoke_test.gd -- /tmp/phonon-test
```

The first import of a fresh checkout can crash while Godot exits. The import itself has finished by then, and later runs work.

## CI

`.github/workflows/build.yml` checks formatting and runs clippy. It builds the library for macOS (one universal library), Linux x86_64 and Windows x86_64, and uploads the assembled addon as the `phonon-godot-addon` artifact. It also zips the addon as `phonon-godot-<version>.zip`, with `addons/phonon/` at the root of the zip, and installs that zip into a new project on Linux, Windows and macOS to run the smoke test there.

## Releasing

Pushing a tag `v<version>`, where `<version>` is the one in `Cargo.toml`, runs the same workflow and attaches `phonon-godot-<version>.zip` to a draft GitHub release. That zip is the file to upload to the Godot Asset Store. [`store/launch-checklist.md`](store/launch-checklist.md) lists the steps, and [`store/listing.md`](store/listing.md) is a draft of the store listing. The `store/` folder also holds the thumbnail, the screenshots and the full-size Phonon logo.

## License

The plugin code is licensed under **MIT OR Apache-2.0**, at your option. [LICENSE](LICENSE) states this with both full texts, which are also in [LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE). The addon folder keeps a copy of `LICENSE`, so it travels with the addon.

The models and voices are not part of this repository and have their own licenses. See [Models and licenses](#models-and-licenses).

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this repository is dual licensed as above, without additional terms or conditions.
