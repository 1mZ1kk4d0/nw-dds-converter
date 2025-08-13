use clap::Parser;
use anyhow::{Result, Context};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use tokio::fs;
use tokio::sync::Semaphore;
use walkdir::WalkDir;
use indicatif::{ProgressBar, ProgressStyle};
use std::env;

// Embutir o texconv.exe no binário
const TEXCONV_EXE: &[u8] = include_bytes!("../texconv.exe");

#[derive(Parser)]
#[command(name = "dds-converter")]
#[command(about = "DDS file converter using embedded texconv.exe")]
struct Cli {
    /// Input folder with .dds files
    #[arg(short, long)]
    input: PathBuf,

    /// Output folder for converted files
    #[arg(short, long)]
    output: PathBuf,

    /// Output format (png, jpg, bmp, tga, dds, etc.)
    #[arg(short, long, default_value = "png")]
    format: String,

    /// Number of folder segments to remove from output path
    #[arg(short, long, default_value = "0")]
    strip_segments: usize,

    /// Number of parallel processes
    #[arg(short, long, default_value = "4")]
    concurrency: usize,

    /// Only show which files would be processed
    #[arg(short, long)]
    dry_run: bool,

    /// Show detailed information during processing
    #[arg(short, long)]
    verbose: bool,

    /// Continue processing even if errors occur in specific files
    #[arg(long)]
    continue_on_error: bool,
}

async fn test_texconv(texconv_path: &Path) -> Result<()> {
    let test_output = Command::new(texconv_path)
        .arg("-h")
        .output()
        .context("Failed to run texconv.exe for test")?;
    
    // texconv.exe returns code 1 for -h, but that's normal
    if test_output.status.code() != Some(0) && test_output.status.code() != Some(1) {
        anyhow::bail!("texconv.exe returned unexpected error code in test: code {:?}\n{}", 
                     test_output.status.code(),
                     String::from_utf8_lossy(&test_output.stderr));
    }
    
    Ok(())
}

async fn setup_texconv() -> Result<PathBuf> {
    let temp_dir = env::temp_dir().join("dds-converter-rust");
    fs::create_dir_all(&temp_dir).await
        .context("Failed to create temporary directory")?;
    
    let texconv_path = temp_dir.join("texconv.exe");
    
    if texconv_path.exists() {
        if test_texconv(&texconv_path).await.is_ok() {
            return Ok(texconv_path);
        }
        let _ = fs::remove_file(&texconv_path).await;
    }
    
    fs::write(&texconv_path, TEXCONV_EXE).await
        .context("Failed to extract texconv.exe")?;
    
    test_texconv(&texconv_path).await
        .context("Error testing texconv.exe")?;
    
    Ok(texconv_path)
}

fn calculate_output_path(input_path: &Path, input_dir: &Path, output_dir: &Path, strip_segments: usize, format: &str) -> PathBuf {
    // Get the relative path from input directory to the file
    let relative_path = input_path.strip_prefix(input_dir).unwrap_or(input_path);
    
    // Apply strip_segments if specified
    let path_components: Vec<_> = relative_path.components().collect();
    let components_to_use = if strip_segments < path_components.len() {
        &path_components[strip_segments..]
    } else {
        &path_components[..]
    };
    
    // Build the output path maintaining the directory structure
    let mut result_path = output_dir.to_path_buf();
    for component in components_to_use {
        result_path.push(component);
    }
    
    // Change the extension to the target format
    result_path.with_extension(format)
}

async fn process_file(
    file_path: &Path,
    texconv_path: &Path,
    input_dir: &Path,
    output_dir: &Path,
    strip_segments: usize,
    verbose: bool,
    format: &str,
    continue_on_error: bool,
) -> Result<()> {
    let metadata = fs::metadata(file_path).await
        .context("Failed to read file metadata")?;
    
    if metadata.len() < 128 {
        if verbose {
            println!("⚠️  Skipping very small file: {}", file_path.display());
        }
        return Ok(());
    }

    let output_path = calculate_output_path(file_path, input_dir, output_dir, strip_segments, format);
    
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).await
            .context("Failed to create output directory")?;
    }

    if verbose {
        println!("🔄 Processing: {} -> {}", 
                file_path.display(), output_path.display());
    }

    let output = Command::new(texconv_path)
        .arg("-f")
        .arg("R8G8B8A8_UNORM")
        .arg("-ft")
        .arg(format)
        .arg("-y")  // Overwrite existing files
        .arg("-o")
        .arg(output_path.parent().unwrap())
        .arg(file_path)
        .output()
        .context("Failed to run texconv")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        
        let error_msg = format!(
            "texconv failed for {}: code {}\nStderr: {}\nStdout: {}",
            file_path.display(),
            output.status.code().unwrap_or(-1),
            stderr,
            stdout
        );
        
        if continue_on_error {
            println!("⚠️  {}", error_msg);
            return Ok(());
        } else {
            anyhow::bail!(error_msg);
        }
    }

    if verbose {
        println!("✅ Done: {}", output_path.display());
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    let texconv_path = setup_texconv().await?;
    
    if cli.verbose {
        println!("✅ texconv.exe extracted to: {}", texconv_path.display());
    }
    
    println!("🔍 Searching for DDS files in: {}", cli.input.display());
    
    let dds_files: Vec<PathBuf> = WalkDir::new(&cli.input)
        .into_iter()
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.is_file() && 
               path.extension()
                   .and_then(|ext| ext.to_str())
                   .map(|ext| ext.to_lowercase() == "dds")
                   .unwrap_or(false) {
                Some(path.to_path_buf())
            } else {
                None
            }
        })
        .collect();

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