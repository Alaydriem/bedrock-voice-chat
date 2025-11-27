use std::env;
use std::path::Path;

use bvc_client_lib::audio::recording::renderer::{AudioFormat, AudioFormatRenderer};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        eprintln!("Usage: {} <session_path> <player_name> <output_file>", args[0]);
        eprintln!();
        eprintln!("Arguments:");
        eprintln!("  session_path  - Path to the recording session directory");
        eprintln!("  player_name   - Name of the player to render");
        eprintln!("  output_file   - Output file path (.wav)");
        eprintln!();
        eprintln!("Example:");
        eprintln!("  {} test-data/019aae95-82e2-7676-9466-ec8d5399e798 Alaydriem output.wav", args[0]);
        std::process::exit(1);
    }

    let session_path = Path::new(&args[1]);
    let player_name = &args[2];
    let output_path = Path::new(&args[3]);

    let format = AudioFormat::Bwav;

    println!("Rendering audio for player '{}' from session at {:?}", player_name, session_path);
    println!("Output file: {:?}", output_path);
    println!("Format: {:?}", format);
    println!();

    // Render the audio
    match format.render(session_path, player_name, output_path).await {
        Ok(()) => {
            println!("Successfully rendered audio to {:?}", output_path);
            println!("You can now listen to the output with your headphones.");
        }
        Err(e) => {
            eprintln!("Error rendering audio: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}
