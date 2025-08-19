use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "dds-converter")]
#[command(about = "DDS file converter using embedded texconv.exe")]
pub struct Cli {
    /// Input folder with .dds files
    #[arg(short, long)]
    pub input: PathBuf,

    /// Output folder for converted files
    #[arg(short, long)]
    pub output: PathBuf,

    /// Output format (png, jpg, bmp, tga, dds, etc.)
    #[arg(short, long, default_value = "png")]
    pub format: String,

    /// Number of folder segments to remove from output path
    #[arg(short, long, default_value = "0")]
    pub strip_segments: usize,

    /// Number of parallel processes
    #[arg(short, long, default_value = "4")]
    pub concurrency: usize,

    /// Only show which files would be processed
    #[arg(short, long)]
    pub dry_run: bool,

    /// Show detailed information during processing
    #[arg(short, long)]
    pub verbose: bool,

    /// Continue processing even if errors occur in specific files
    #[arg(long)]
    pub continue_on_error: bool,

    /// Create animated GIF/WebP from PNG sequence (requires --animation-mode)
    #[arg(long)]
    pub animation_mode: bool,

    /// Frame delay in milliseconds for animations (default: 100ms)
    #[arg(long, default_value = "100")]
    pub frame_delay: u16,

    /// Animation output format (webp with transparency)
    #[arg(long, default_value = "webp")]
    pub animation_format: String,
}
