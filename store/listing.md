# Asset Store listing draft

The fields of the [Godot Asset Store](https://store.godotengine.org/) form, filled in for the first version. Nothing here has been submitted. Copy each field into the store when the asset is created, and check the ones marked **Check**.

This folder is export-ignored, and none of it goes into the addon zip.

## Upload Asset page

| Field | Value |
|---|---|
| Publisher | Gradium. **Check**: the public publisher name. |
| Publisher URL slug | `gradium` |
| Asset name | Phonon Text-to-Speech |
| Asset URL | `phonon-text-to-speech` |

## Settings tab

**Asset summary**

> Speak text in your game with Phonon, an on-device text-to-speech model. One node, audio that plays while it is generated, and no server or API key.

**Detailed description**

> Phonon Text-to-Speech adds one node, PhononSpeaker, that speaks text with Phonon, Gradium's on-device text-to-speech model. The speech is generated on the player's machine. There is no server and no API key.
>
> - `speak(text)` starts playing as soon as the first audio is ready, while the rest of the line is still being generated. `stop()` cuts it off.
> - `synthesize(text)` generates a line ahead of time and returns it as an AudioStreamWAV.
> - The speech can play through an AudioStreamPlayer, AudioStreamPlayer2D or AudioStreamPlayer3D, so a character can speak from where it stands.
> - Many English voices, including character voices, come from the Phonon voice catalog.
> - Loading and generation run on worker threads, so the game never waits for them.
>
> ```gdscript
> @onready var speaker: PhononSpeaker = $PhononSpeaker
>
> func _ready() -> void:
>     speaker.loaded.connect(func(): speaker.speak("Welcome back, traveler."))
>     speaker.load_model()
> ```
>
> **Models and licenses.** The default model is Phonon 31m, which speaks English. It is under Apache 2.0, so you can ship it in a commercial game. The first `load_model()` downloads it from Hugging Face, so the game needs an internet connection once. Exported games keep it in their `user://` folder. The voice catalog is under Apache 2.0 too.
>
> The plugin can also load the Phonon 40m models, in English, French, German, Spanish and Portuguese, and Phonon 40m multilingual. These are under CC BY-NC 4.0, which does not allow commercial use. A commercial game needs a license from Gradium to use them: talk to Gradium's sales team at https://gradium.ai/contact?subject=Sales.
>
> **Platforms.** Godot 4.7 or later, on macOS 15 or later (Apple silicon and Intel), Linux x86_64 and Windows x86_64. The x86_64 libraries need a CPU with AVX2. Android, iOS and web exports are not supported yet.
>
> The plugin code is licensed under MIT OR Apache-2.0. The source, the full documentation and a demo project are on GitHub.

**Tags** (the store suggests at most ten; none may be the asset's name):

GDExtension, Audio, TTS, Text To Speech, Voice, Accessibility, Dialogue, NPC, Rust, AI

All of them exist on the store already, so the listing shows up in their searches.

**Asset type:** Addon

**License:** a custom license named `MIT OR Apache-2.0`, linking to `https://github.com/gradium-ai/phonon-godot/blob/main/LICENSE`. The store's built-in choices are single licenses (MIT, Apache 2.0, CC0, GPL v3), and its guidelines ask that the listed license match the one in the download. The link works once the repository is public. Choosing MIT alone is the other option: it keeps the asset in the store's MIT filter, and users may take the code under MIT, but it no longer states the dual license exactly.

**Link to source code:** `https://github.com/gradium-ai/phonon-godot`, once the repository is public.

**AI usage disclosure** (mandatory when AI was used). **Check**: confirm or rewrite this before submitting.

> The plugin's Rust and GDScript code, its documentation and the editor class icon were written with the help of an AI coding assistant (a large language model). They were reviewed and tested by the developers. The Phonon logo and the thumbnail's design template are Gradium's own artwork. At run time, the plugin uses Phonon, a neural text-to-speech model, to generate speech.

## Media tab

| Field | File |
|---|---|
| Thumbnail (16:9, required) | `thumbnail.png`, 1920x1080: the whole Phonon monogram and "Phonon / On-device text-to-speech / for Godot" on the background of Gradium's thumbnail generator (blue colourway, flow pattern), drawn at 16:9. The text is set in Inter (SIL Open Font License); design may swap in the brand font. |
| Screenshot 1 | `screenshot-demo.png`, the demo scene speaking a line |
| Screenshot 2 | `screenshot-inspector.png`, a PhononSpeaker node in the inspector |
| Featured image | None |
| Video | None |

`phonon-monogram.png` is the full-size Phonon logo, 1000x1000, for any other place the store asks for a square image.

## Versions tab

| Field | Value |
|---|---|
| Version name | 0.1.0 |
| File | `phonon-godot-0.1.0.zip`, from the draft GitHub release `v0.1.0` |
| Changelog | First release: the PhononSpeaker node with `load_model()`, `speak()`, `stop()`, `synthesize()` and `voices()`, for macOS, Linux x86_64 and Windows x86_64. |
| Minimum Godot version | 4.7 |
| Maximum Godot version | None |
| Additional information | The first load downloads the model from Hugging Face. See the README in `addons/phonon/` for the model licenses. |

## Pricing tab

Free. No donation link.
