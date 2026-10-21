extends Control
## Type a line, pick a voice and press Speak.

@onready var speaker: PhononSpeaker = $PhononSpeaker
@onready var text_edit: LineEdit = %Text
@onready var voices: OptionButton = %Voices
@onready var speak_button: Button = %Speak
@onready var stop_button: Button = %Stop
@onready var status: Label = %Status


func _ready() -> void:
	speaker.loaded.connect(_on_loaded)
	speaker.load_failed.connect(func(message: String): status.text = "Could not load: " + message)
	speaker.started.connect(func(): status.text = "Speaking")
	speaker.finished.connect(func(): status.text = "Done")
	speaker.error.connect(func(message: String): status.text = "Error: " + message)
	speak_button.pressed.connect(_on_speak)
	text_edit.text_submitted.connect(func(_text: String): _on_speak())
	stop_button.pressed.connect(speaker.stop)
	status.text = "Loading the model. The first run downloads it."
	speaker.load_model()


func _on_loaded() -> void:
	voices.clear()
	for voice in speaker.voices():
		voices.add_item(voice)
	speak_button.disabled = false
	status.text = "Ready"


func _on_speak() -> void:
	if speak_button.disabled:
		return
	speaker.voice = voices.get_item_text(voices.selected)
	speaker.speak(text_edit.text)
