## KFA (Khmer Forced Aligner) - Native Rust Port

A stupid-fast Khmer Forced Aligner powered by **Wav2Vec2CTC**, **Phonetisaurus**, and **ONNXRuntime**, written entirely in native Rust.

### Features
- Native Viterbi CTC Forced Alignment rewritten in Rust via `ndarray`.
- Full Khmer syllable-level unicode normalization and CRF-based tokenization.
- Support for generating both `jsonl` metrics and exact `whisper`-compatible segment outputs.
- Optional external crate (`transcribe-audio-to-text`) mimicking Google Speech Recognition exactly to pre-transcribe inputs.
- Hardware Acceleration via `ort` v2 (CUDA support when provided).

---

### Installation & Build

Ensure you have [Rust](https://rustup.rs/) installed. Since the pipeline uses Dynamic ORT loading (`load-dynamic`), make sure you have `onnxruntime.dll` (or `.so`/`.dylib` depending on your OS) available and pointed to by your environment prior to running.

```shell
git clone https://github.com/seanghay/kfa.git
cd kfa
cargo build --release
```

### Setup

Download the CPU or GPU version of ONNXRuntime (minimum v1.16+) and either:
- Drop the `.dll` directly into your workspace.
- Export it as an environment variable: `$env:ORT_DYLIB_PATH="C:\path\to\onnxruntime.dll"`

---

### Usage

The `kfa-cli` orchestrates reading the source Audio file and a pre-generated **Text** file to produce the highly accurate start/end subtitle alignments.

> [!Note]
> The **input audio sample rate must be exactly 16kHz Mono**. Any other configurations will be warned and drastically affect model performance.

#### Command Line

```shell
# Output exact timing metrics as JSON Lines
cargo run --release -p kfa-cli -- -a data/audio.wav -t data/text.txt -o output.jsonl -f jsonl

# Output standard Whisper-style JSON format
cargo run --release -p kfa-cli -- -a data/audio.wav -t data/text.txt -o output.json -f whisper
```

> **Pro-tip**: You can provide the `--device cuda` flag if your Dynamic ONNX dll has been compiled with CUDA Execution provider dependencies!

#### Audio Transcription Crate

Don't have a transcript ready? KFA provides a bundled utility `transcribe-audio-to-text` designed to replicate raw Google Speech Recognition inference to bootstrap your alignment files.

This crate automatically interfaces with your system's `ffmpeg` wrapper to convert your audio input into Google's preferred 16kHz `flac` format behind the scenes before making requests.

```shell
cargo run --release -p transcribe-audio-to-text -- --audio path/to/your/audio.wav > transcript.txt
```

---

### References
- [MMS: Scaling Speech Technology to 1000+ languages](https://github.com/facebookresearch/fairseq/tree/main/examples/mms)
- [CTC FORCED ALIGNMENT API TUTORIAL](https://pytorch.org/audio/main/tutorials/ctc_forced_alignment_api_tutorial.html)
- [Phonetisaurus](https://github.com/AdolfVonKleist/Phonetisaurus)
- [Fine-Tune Wav2Vec2 for English ASR](https://huggingface.co/blog/fine-tune-wav2vec2-english)

### License

`Apache-2.0`
