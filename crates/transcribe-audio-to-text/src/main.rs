use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use transcribe_audio_to_text::transcribe_audio_to_text;

#[derive(Parser)]
#[command(
    author,
    version,
    about = "Transcribe Khmer audio to text using Google Speech Recognition"
)]
struct Args {
    #[arg(short, long)]
    audio: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let transcript = transcribe_audio_to_text(&args.audio)?;
    println!("Transcription:\n{}", transcript);
    Ok(())
}
