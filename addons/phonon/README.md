# Phonon for Godot

Speak text in your game with Phonon, Gradium's on-device text-to-speech model. The speech is generated on the player's machine, with no server and no API key.

The addon adds one node, `PhononSpeaker`. It needs Godot 4.7 or later, on macOS 15 or later, Linux x86_64 or Windows x86_64. Android, iOS and web exports are not supported in this version.

On macOS, clear the quarantine flag of a downloaded copy once, or Godot cannot load the library:

```sh
xattr -dr com.apple.quarantine addons/phonon/bin/macos
```

## Quick start

```gdscript
extends Node

@onready var speaker: PhononSpeaker = $PhononSpeaker


func _ready() -> void:
	speaker.loaded.connect(func(): speaker.speak("Welcome back, traveler."))
	speaker.load_failed.connect(func(message: String): push_error(message))
	speaker.load_model()
```

`load_model()` loads the model on a worker thread and emits `loaded` or `load_failed`. `speak(text)` starts playing while the rest of the line is still being generated, and emits `started` and `finished`. `stop()` stops it. `synthesize(text)` returns a request id at once, and `synthesized(id, audio)` follows with an `AudioStreamWAV`. `voices()` lists the voices the `voice` property can name. The editor's help (F1, then search for `PhononSpeaker`) has the full reference.

## Models and licenses

The default model is Phonon 31m, `hf://gradium/phonon-31m-en`, which speaks English. It is under Apache 2.0, so you can ship it in a commercial game.

The Phonon 40m models (`hf://gradium/phonon-40m-en`, `-fr`, `-de`, `-es`, `-pt` and `hf://gradium/phonon-40m-multilingual`) are under CC BY-NC 4.0, which does not allow commercial use. A commercial game needs a license from Gradium to use them. To get one, [talk to Gradium's sales team](https://gradium.ai/contact?subject=Sales). The [model table on the Phonon 31m page](https://huggingface.co/gradium/phonon-31m-en#the-phonon-family) lists the license of each model.

The voices other than the model's default come from the voice catalog [`gradium/phonon-voices`](https://huggingface.co/gradium/phonon-voices), which is under Apache 2.0, as its Hugging Face page states.

## Model download

The first `load_model()` downloads the model from Hugging Face. In the editor it goes to the shared Hugging Face cache. In an exported game it goes to `user://phonon/huggingface`. The `HF_HUB_CACHE` environment variable overrides both. To ship a game that needs no download, put the model's files in a folder beside the game and set `model` to that folder's absolute path.

## License

The addon code is licensed under MIT OR Apache-2.0, at your option. See `LICENSE`. The libraries also contain third-party code, listed with its licenses in `THIRD_PARTY_NOTICES.md`.
