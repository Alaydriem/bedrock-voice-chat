use std::env;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        eprintln!("Usage: {} <session_path> <player_name> <output.wav>", args[0]);
        eprintln!();
        eprintln!("Example:");
        eprintln!("  {} test-data/019aae95-82e2-7676-9466-ec8d5399e798 Alaydriem output.wav", args[0]);
        std::process::exit(1);
    }

    let session_path = Path::new(&args[1]);
    let player_name = &args[2];
    let output_path = Path::new(&args[3]);

    println!("Rendering audio for player '{}' from session at {:?}", player_name, session_path);
    println!("Output file: {:?}", output_path);
    println!();

    // Import the renderer module
    use bvc_client_lib::audio::recording::renderer::{AudioRenderer, BwavRenderer};

    // Create renderer
    let mut renderer = BwavRenderer::new();

    // Render the audio
    match renderer.render(session_path, player_name, output_path) {
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
