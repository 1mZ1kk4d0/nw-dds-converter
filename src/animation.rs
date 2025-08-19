use crate::sprite::SpriteSheet;
use anyhow::{Context, Result};
use image::{DynamicImage, RgbaImage};
use std::path::{Path, PathBuf};

pub fn find_sprite_sequences(input_dir: &Path) -> Result<Vec<(PathBuf, PathBuf)>> {
    let mut sequences = Vec::new();

    let entries: Vec<_> = std::fs::read_dir(input_dir)?
        .filter_map(|entry| entry.ok())
        .collect();

    for entry in entries {
        let path = entry.path();
        if let Some(ext) = path.extension() {
            if ext == "dds" {
                let sprite_path = path.with_extension("sprite");
                if sprite_path.exists() {
                    sequences.push((path, sprite_path));
                }
            }
        }
    }

    Ok(sequences)
}

pub fn find_image_sequences(input_dir: &Path) -> Result<Vec<Vec<PathBuf>>> {
    let mut sequences = Vec::new();
    let mut files: Vec<PathBuf> = std::fs::read_dir(input_dir)?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            let ext = path.extension()?.to_str()?;
            if matches!(ext, "png" | "dds" | "jpg" | "jpeg" | "bmp" | "tga") {
                Some(path)
            } else {
                None
            }
        })
        .collect();

    files.sort();

    let mut groups: std::collections::HashMap<String, Vec<PathBuf>> =
        std::collections::HashMap::new();

    for file in files {
        let file_name = file.file_stem().and_then(|s| s.to_str()).unwrap_or("");

        let base_name = if let Some(pos) = file_name.rfind('_') {
            let (base, suffix) = file_name.split_at(pos);
            if suffix[1..].chars().all(|c| c.is_ascii_digit()) {
                base.to_string()
            } else {
                file_name.to_string()
            }
        } else {
            let mut base_end = file_name.len();
            while base_end > 0
                && file_name
                    .chars()
                    .nth(base_end - 1)
                    .unwrap_or('a')
                    .is_ascii_digit()
            {
                base_end -= 1;
            }
            if base_end < file_name.len() {
                file_name[..base_end].to_string()
            } else {
                file_name.to_string()
            }
        };

        groups.entry(base_name).or_insert_with(Vec::new).push(file);
    }

    for (_, mut group) in groups {
        if group.len() > 1 {
            group.sort();
            sequences.push(group);
        }
    }

    Ok(sequences)
}

pub fn create_webp_animation(
    image_files: &[PathBuf],
    output_path: &Path,
    frame_delay: u16,
) -> Result<()> {
    let mut frames = Vec::new();

    for image_path in image_files {
        let img = load_image_file(image_path)
            .with_context(|| format!("Failed to open image file: {}", image_path.display()))?;
        frames.push(img.to_rgba8());
    }

    create_webp_animation_with_ffmpeg(&frames, output_path, frame_delay)
}

fn load_image_file(path: &Path) -> Result<DynamicImage> {
    image::open(path).context("Failed to load image")
}

fn create_webp_animation_with_ffmpeg(
    frames: &[RgbaImage],
    output_path: &Path,
    frame_delay: u16,
) -> Result<()> {
    if frames.is_empty() {
        anyhow::bail!("No frames to create WebP animation");
    }

    println!("Creating WebP animation with {} frames and transparency using ffmpeg", frames.len());
    
    // Criar diretório temporário
    let temp_dir = std::env::temp_dir().join("webp_animation_frames");
    std::fs::create_dir_all(&temp_dir)?;
    
    // Salvar frames como PNG temporários (preserva transparência)
    for (i, frame) in frames.iter().enumerate() {
        let frame_path = temp_dir.join(format!("frame_{:04}.png", i));
        frame.save(&frame_path)?;
    }
    
    let framerate = 1000.0 / frame_delay as f32;
    
    // Executar ffmpeg para criar WebP animado com transparência
    let output = std::process::Command::new("ffmpeg")
        .arg("-y") // Overwrite output
        .arg("-framerate")
        .arg(framerate.to_string())
        .arg("-i")
        .arg(temp_dir.join("frame_%04d.png"))
        .arg("-c:v")
        .arg("libwebp")
        .arg("-lossless")
        .arg("0")
        .arg("-compression_level")
        .arg("6")
        .arg("-q:v")
        .arg("85")
        .arg("-loop")
        .arg("0") // Infinite loop
        .arg(output_path)
        .output();
    
    // Limpar arquivos temporários
    let _ = std::fs::remove_dir_all(&temp_dir);
    
    match output {
        Ok(result) => {
            if result.status.success() {
                println!("WebP animation created successfully with {} frames and transparency!", frames.len());
                Ok(())
            } else {
                let stderr = String::from_utf8_lossy(&result.stderr);
                println!("ffmpeg failed: {}", stderr);
                
                // Fallback: criar WebP estático do primeiro frame
                println!("Creating static WebP as fallback...");
                let encoder = webp::Encoder::from_rgba(&frames[0], frames[0].width(), frames[0].height());
                let encoded = encoder.encode(85.0);
                std::fs::write(output_path, &*encoded)?;
                println!("Created static WebP with transparency: {}", output_path.display());
                
                Ok(())
            }
        }
        Err(e) => {
            println!("ffmpeg not found: {}", e);
            println!("Install ffmpeg for animated WebP support");
            
            // Fallback: criar WebP estático do primeiro frame
            println!("Creating static WebP as fallback...");
            let encoder = webp::Encoder::from_rgba(&frames[0], frames[0].width(), frames[0].height());
            let encoded = encoder.encode(85.0);
            std::fs::write(output_path, &*encoded)?;
            println!("Created static WebP with transparency: {}", output_path.display());
            
            Ok(())
        }
    }
}

fn is_frame_mostly_black(frame: &RgbaImage) -> bool {
    let total_pixels = (frame.width() * frame.height()) as usize;
    let mut black_pixels = 0;
    let mut transparent_pixels = 0;
    
    for pixel in frame.pixels() {
        let [r, g, b, a] = pixel.0;
        
        if a < 10 {
            transparent_pixels += 1;
        } else if r < 10 && g < 10 && b < 10 {
            black_pixels += 1;
        }
    }
    
    let empty_pixels = black_pixels + transparent_pixels;
    (empty_pixels as f32 / total_pixels as f32) > 0.95
}

pub fn create_animation_from_sprite_sheet(
    dds_path: &Path,
    sprite_path: &Path,
    output_path: &Path,
    frame_delay: u16,
    format: &str,
) -> Result<()> {
    let sprite_sheet = SpriteSheet::from_xml_file(sprite_path)
        .with_context(|| format!("Failed to load sprite sheet: {}", sprite_path.display()))?;

    println!("Found {} frames in sprite sheet", sprite_sheet.cells.len());

    let texture = image::open(dds_path)
        .with_context(|| format!("Failed to load DDS texture: {}", dds_path.display()))?;

    let mut frames = sprite_sheet
        .extract_frames(&texture)
        .context("Failed to extract frames from sprite sheet")?;

    println!("Extracted {} frames from texture", frames.len());
    
    // Manter exatamente 23 frames (remover apenas o último se for preto)
    if frames.len() == 24 && is_frame_mostly_black(&frames[23]) {
        frames.pop();
        println!("Removed last black frame");
    }
    println!("Using {} frames for animation", frames.len());

    match format {
        "webp" => {
            create_webp_animation_with_ffmpeg(&frames, output_path, frame_delay)?;
        }
        _ => {
            anyhow::bail!("Only WebP format is supported (with transparency)");
        }
    }

    Ok(())
}