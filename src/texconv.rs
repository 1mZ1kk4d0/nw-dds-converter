use anyhow::{Result, Context};
use std::path::{Path, PathBuf};
use std::process::Command;
use tokio::fs;
use std::env;

// Embutir o texconv.exe no binário
const TEXCONV_EXE: &[u8] = include_bytes!("../texconv.exe");

pub async fn test_texconv(texconv_path: &Path) -> Result<()> {
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

pub async fn setup_texconv() -> Result<PathBuf> {
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
