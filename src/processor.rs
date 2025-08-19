use anyhow::{Result, Context};
use std::path::{Path, PathBuf};
use std::process::Command;
use tokio::fs;

pub fn calculate_output_path(
    input_path: &Path, 
    input_dir: &Path, 
    output_dir: &Path, 
    strip_segments: usize, 
    format: &str
) -> PathBuf {
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

pub async fn process_file(
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
