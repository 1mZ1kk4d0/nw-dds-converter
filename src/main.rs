mod cli;
mod texconv;
mod processor;
mod utils;
mod animation;
mod sprite;

use clap::Parser;
use anyhow::{Result, Context};
use std::sync::Arc;
use std::path::{Path, PathBuf};
use tokio::sync::Semaphore;
use indicatif::{ProgressBar, ProgressStyle};

use cli::Cli;
use texconv::setup_texconv;
use processor::{calculate_output_path, process_file};
use utils::find_dds_files;
use animation::{find_image_sequences, find_sprite_sequences, create_webp_animation, create_animation_from_sprite_sheet};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Handle animation mode
    if cli.animation_mode {
        return handle_animation_mode(&cli).await;
    }
    
    let texconv_path = setup_texconv().await?;
    
    if cli.verbose {
        println!("✅ texconv.exe extracted to: {}", texconv_path.display());
    }
    
    println!("🔍 Searching for DDS files in: {}", cli.input.display());
    
    let dds_files = find_dds_files(&cli.input);

    if dds_files.is_empty() {
        println!("❌ No .dds files found!");
        return Ok(());
    }

    if cli.dry_run {
        println!("🔍 Dry-run mode - files that would be processed:");
        for file in &dds_files {
            let output_path = calculate_output_path(&file, &cli.input, &cli.output, cli.strip_segments, &cli.format);
            println!("  {} -> {}", file.display(), output_path.display());
        }
        return Ok(());
    }

    println!("📊 Found {} DDS files", dds_files.len());
    
    let progress = ProgressBar::new(dds_files.len() as u64);
    progress.set_style(
        ProgressStyle::default_bar()
            .template("{elapsed_precise} [{bar:50.cyan/blue}] {pos}/{len} ({percent}%) {msg}")
            .unwrap()
    );

    let semaphore = Arc::new(Semaphore::new(cli.concurrency));
    let mut tasks = Vec::new();

    for file in dds_files {
        let permit = semaphore.clone().acquire_owned().await?;
        let texconv_path = texconv_path.clone();
        let input_dir = cli.input.clone();
        let output_dir = cli.output.clone();
        let strip_segments = cli.strip_segments;
        let verbose = cli.verbose;
        let format = cli.format.clone();
        let continue_on_error = cli.continue_on_error;
        let progress = progress.clone();

        let task = tokio::spawn(async move {
            let _permit = permit;
            let result = process_file(
                &file,
                &texconv_path,
                &input_dir,
                &output_dir,
                strip_segments,
                verbose,
                &format,
                continue_on_error,
            ).await;
            
            progress.inc(1);
            
            if let Err(e) = &result {
                progress.println(format!("❌ Error in {}: {}", file.display(), e));
            }
            
            result
        });
        
        tasks.push(task);
    }

    let mut error_count = 0;
    for task in tasks {
        if let Err(e) = task.await? {
            error_count += 1;
            if !cli.continue_on_error {
                progress.finish_with_message("❌ Stopped due to error");
                return Err(e);
            }
        }
    }

    progress.finish_with_message("✅ Processing completed!");
    
    if error_count > 0 {
        println!("⚠️  Processing completed with {} error(s)", error_count);
    } else {
        println!("🎉 All files were processed successfully!");
    }

    Ok(())
}

async fn handle_animation_mode(cli: &Cli) -> Result<()> {
    println!("🎬 Animation mode: Converting sequences to {}", cli.animation_format.to_uppercase());
    println!("🔍 Searching for sequences in: {}", cli.input.display());
    
    // First, look for sprite sheets (DDS + .sprite files)
    let sprite_sequences = find_sprite_sequences(&cli.input)?;
    
    if !sprite_sequences.is_empty() {
        println!("📊 Found {} sprite sheet(s)", sprite_sequences.len());
        
        // Create output directory
        tokio::fs::create_dir_all(&cli.output).await?;
        
        for (dds_path, sprite_path) in sprite_sequences {
            println!("🎞️  Processing sprite sheet: {}", dds_path.display());
            
            if cli.verbose {
                println!("  DDS: {}", dds_path.display());
                println!("  Sprite: {}", sprite_path.display());
            }
            
            // Generate output filename
            let base_name = dds_path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("animation");
            
            let output_filename = format!("{}.{}", base_name, cli.animation_format);
            let output_path = cli.output.join(output_filename);
            
            println!("📤 Creating: {}", output_path.display());
            
            create_animation_from_sprite_sheet(
                &dds_path,
                &sprite_path,
                &output_path,
                cli.frame_delay,
                &cli.animation_format
            )?;
        }
        
        println!("🎉 All sprite sheet animations created successfully!");
        return Ok(());
    }
    
    // Fallback to regular image sequences
    let sequences = find_image_sequences(&cli.input)?;
    
    if sequences.is_empty() {
        println!("❌ No image sequences found!");
        println!("💡 Make sure your image files follow a naming pattern like:");
        println!("   - animation_001.png, animation_002.png");
        println!("   - named_bg_1.dds, named_bg_2.dds");
        println!("   - frame1.jpg, frame2.jpg");
        return Ok(());
    }
    
    println!("📊 Found {} PNG sequence(s)", sequences.len());
    
    // Create output directory
    tokio::fs::create_dir_all(&cli.output).await?;
    
    for (seq_idx, sequence) in sequences.iter().enumerate() {
        println!("🎞️  Processing sequence {} with {} frames", seq_idx + 1, sequence.len());
        
        if cli.verbose {
            for (i, file) in sequence.iter().enumerate() {
                println!("  Frame {}: {}", i + 1, file.display());
            }
        }
        
        // Generate output filename based on first file in sequence
        let first_file = &sequence[0];
        let base_name = first_file.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("animation");
        
        // Remove numeric suffix if present
        let clean_base = if let Some(pos) = base_name.rfind('_') {
            let (base, suffix) = base_name.split_at(pos);
            if suffix[1..].chars().all(|c| c.is_ascii_digit()) {
                base
            } else {
                base_name
            }
        } else {
            base_name
        };
        
        let output_filename = format!("{}.{}", clean_base, cli.animation_format);
        let output_path = cli.output.join(output_filename);
        
        println!("📤 Creating: {}", output_path.display());
        
        // Check if we need to convert DDS files first
        let has_dds = sequence.iter().any(|f| {
            f.extension().and_then(|s| s.to_str()).unwrap_or("") == "dds"
        });
        
        let processed_sequence = if has_dds {
            println!("🔄 Converting DDS files to PNG first...");
            let texconv_path = setup_texconv().await?;
            convert_dds_sequence_to_png(sequence, &texconv_path, &cli.output).await?
        } else {
            sequence.clone()
        };
        
        match cli.animation_format.as_str() {
            "webp" => {
                create_webp_animation(&processed_sequence, &output_path, cli.frame_delay)?;
                println!("✅ WebP animation created successfully!");
            }
            _ => {
                println!("❌ Only WebP format is supported (with transparency)");
                continue;
            }
        }
    }
    
    println!("🎉 All animations created successfully!");
    Ok(())
}

async fn convert_dds_sequence_to_png(
    dds_files: &[PathBuf], 
    texconv_path: &Path, 
    temp_dir: &Path
) -> Result<Vec<PathBuf>> {
    let mut png_files = Vec::new();
    
    // Create temp directory for PNG conversion
    let png_temp_dir = temp_dir.join("temp_png");
    tokio::fs::create_dir_all(&png_temp_dir).await?;
    
    for dds_file in dds_files {
        let png_name = dds_file.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("frame");
        let png_path = png_temp_dir.join(format!("{}.png", png_name));
        
        // Convert DDS to PNG using texconv
        let output = std::process::Command::new(texconv_path)
            .arg("-f")
            .arg("R8G8B8A8_UNORM")
            .arg("-ft")
            .arg("png")
            .arg("-y")
            .arg("-o")
            .arg(&png_temp_dir)
            .arg(dds_file)
            .output()
            .context("Failed to run texconv for DDS conversion")?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("texconv failed for {}: {}", dds_file.display(), stderr);
        }
        
        png_files.push(png_path);
    }
    
    Ok(png_files)
}