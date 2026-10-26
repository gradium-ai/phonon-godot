extends Control
## Type a line, pick a voice and press Speak.

@onready var speaker: PhononSpeaker = $PhononSpeaker
@onready var text_edit: TextEdit = %Text
@onready var voices: OptionButton = %Voices
@onready var speak_button: Button = %Speak
@onready var stop_button: Button = %Stop
@onready var level: ProgressBar = %Level
@onready var status: Label = %Status


func _ready() -> void:
	speaker.loaded.connect(_on_loaded)
	speaker.load_failed.connect(func(message: String): status.text = "Could not load: " + message)
	speaker.started.connect(func(): status.text = "Speaking")
	speaker.finished.connect(func(): status.text = "Done")
	speaker.error.connect(func(message: String): status.text = "Error: " + message)
	speak_button.pressed.connect(_on_speak)
	stop_button.pressed.connect(speaker.stop)
	status.text = "Loading the model. The first run downloads it."
	speaker.load_model()


func _process(_delta: float) -> void:
	# How loud the output is, from silence at -60 dB to full scale.
	var peak := AudioServer.get_bus_peak_volume_left_db(0, 0)
	level.value = clampf(remap(peak, -60.0, 0.0, 0.0, 1.0), 0.0, 1.0)


func _on_loaded() -> void:
	voices.clear()
	for voice in speaker.voices():
		voices.add_item(voice)
	speak_button.disabled = false
	status.text = "Ready"


func _on_speak() -> void:
	speaker.voice = voices.get_item_text(voices.selected)
	speaker.speak(text_edit.text)
