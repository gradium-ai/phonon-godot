extends SceneTree
## End-to-end check of PhononSpeaker, headless. It loads the default model, writes two
## synthesized lines and a recording of a spoken line as WAV files, and checks the signals of
## speak() and stop(). Exits with status 0 on success.
##
##   godot --headless --path demo --import
##   godot --headless --path demo --script res://tests/smoke_test.gd -- OUTPUT_DIR [QUANT]

const TIMEOUT_MS := 600_000

var speaker: PhononSpeaker
var events: Array = []
var failures: Array[String] = []


func _initialize() -> void:
	_run()


func _run() -> void:
	var args := OS.get_cmdline_user_args()
	var out_dir: String = args[0] if args.size() > 0 else OS.get_user_data_dir()
	DirAccess.make_dir_recursive_absolute(out_dir)

	# Record everything the speaker plays, to check the streamed audio as well.
	var record := AudioEffectRecord.new()
	AudioServer.add_bus_effect(0, record)

	speaker = PhononSpeaker.new()
	if args.size() > 1:
		speaker.quant = args[1]
	root.add_child(speaker)
	speaker.loaded.connect(func(): events.append(["loaded"]))
	speaker.load_failed.connect(func(message): events.append(["load_failed", message]))
	speaker.started.connect(func(): events.append(["started"]))
	speaker.finished.connect(func(): events.append(["finished"]))
	speaker.error.connect(func(message): events.append(["error", message]))
	speaker.synthesized.connect(func(id, audio): events.append(["synthesized", id, audio]))

	speaker.load_model()
	var loaded = await _wait_for(["loaded", "load_failed"])
	if loaded == null or loaded[0] != "loaded":
		_fail("load: %s" % [loaded])
		_finish()
		return
	print("voices: ", speaker.voices())
	_check(speaker.is_loaded(), "is_loaded() after loaded")
	_check(speaker.voices().size() > 1, "voices() lists the catalog")

	# synthesize() in the default voice, then in a catalog voice.
	await _synthesize("The quick brown fox jumps over the lazy dog.", out_dir.path_join("synth_default.wav"))
	speaker.voice = "Easton"
	await _synthesize("It costs $12.50 and leaves at 7 pm.", out_dir.path_join("synth_easton.wav"))

	# An unknown voice fails with error and a null result.
	speaker.voice = "nobody-by-this-name"
	var id := speaker.synthesize("Hello.")
	var result = await _wait_for(["synthesized"])
	_check(result != null and result[1] == id and result[2] == null, "unknown voice gives null audio")
	_check(_seen("error"), "unknown voice emits error")
	speaker.voice = ""

	# stop() cuts a long line short, and no finished follows.
	events.clear()
	speaker.speak("This is a long sentence that should be cut off well before it ends, because the test stops it after a moment. ".repeat(4))
	_check(speaker.is_speaking(), "is_speaking() after speak()")
	var started = await _wait_for(["started", "error"])
	_check(started != null and started[0] == "started", "speak() emits started")
	await _sleep(0.5)
	var stop_at := Time.get_ticks_msec()
	speaker.stop()
	_check(not speaker.is_speaking(), "is_speaking() false after stop()")
	print("stop() returned in %d ms" % (Time.get_ticks_msec() - stop_at))
	await _sleep(1.0)
	_check(not _seen("finished"), "no finished after stop()")

	# A full line plays out and emits finished; a handler that speaks again works.
	events.clear()
	record.set_recording_active(true)
	speaker.finished.connect(func(): speaker.speak("And this is the second line."), CONNECT_ONE_SHOT)
	speaker.speak("Hello from Godot. This line is streamed while it is generated.")
	var first = await _wait_for(["finished", "error"])
	_check(first != null and first[0] == "finished", "speak() emits finished")
	events.clear()
	var second = await _wait_for(["finished", "error"])
	_check(second != null and second[0] == "finished", "speak() from a finished handler")
	await _sleep(0.3)
	record.set_recording_active(false)
	var recording := record.get_recording()
	if recording == null:
		_fail("nothing was recorded from speak()")
	else:
		var path := out_dir.path_join("spoken.wav")
		recording.save_to_wav(path)
		print("wrote %s (%.2f s)" % [path, recording.get_length()])

	_finish()


func _synthesize(text: String, path: String) -> void:
	var id := speaker.synthesize(text)
	var result = await _wait_for(["synthesized"])
	if result == null or result[1] != id or result[2] == null:
		_fail("synthesize(%s): %s" % [text, result])
		return
	var audio: AudioStreamWAV = result[2]
	audio.save_to_wav(path)
	print("wrote %s (%.2f s)" % [path, audio.get_length()])
	_check(audio.get_length() > 1.0, "synthesize() returns audio")


func _wait_for(names: Array) -> Variant:
	var deadline := Time.get_ticks_msec() + TIMEOUT_MS
	while Time.get_ticks_msec() < deadline:
		for event in events:
			if event[0] in names:
				events.erase(event)
				return event
		await process_frame
	_fail("timed out waiting for %s" % [names])
	return null


func _seen(name: String) -> bool:
	return events.any(func(event): return event[0] == name)


func _sleep(seconds: float) -> void:
	await create_timer(seconds).timeout


func _check(ok: bool, what: String) -> void:
	print("%s: %s" % ["ok" if ok else "FAILED", what])
	if not ok:
		failures.append(what)


func _fail(what: String) -> void:
	_check(false, what)


func _finish() -> void:
	speaker.queue_free()
	await process_frame
	print("%d failure(s)" % failures.size())
	quit(1 if failures else 0)
