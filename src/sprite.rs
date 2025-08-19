use anyhow::{Result, Context};
use std::path::Path;
use image::{DynamicImage, RgbaImage, ImageBuffer};

#[derive(Debug, Clone)]
pub struct SpriteCell {
    pub top_left: (f32, f32),
    pub top_right: (f32, f32),
    pub bottom_left: (f32, f32),
    pub bottom_right: (f32, f32),
}

#[derive(Debug)]
pub struct SpriteSheet {
    pub cells: Vec<SpriteCell>,
}

impl SpriteSheet {
    pub fn from_xml_file(sprite_path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(sprite_path)
            .context("Failed to read sprite file")?;
        
        Self::from_xml_content(&content)
    }
    
    pub fn from_xml_content(xml_content: &str) -> Result<Self> {
        let mut cells = Vec::new();
        
        // Simple XML parsing - look for Cell elements
        for line in xml_content.lines() {
            let line = line.trim();
            if line.starts_with("<Cell ") {
                if let Some(cell) = Self::parse_cell_line(line)? {
                    cells.push(cell);
                }
            }
        }
        
        Ok(SpriteSheet { cells })
    }
    
    fn parse_cell_line(line: &str) -> Result<Option<SpriteCell>> {
        // Parse attributes like: topLeft="0,0.16666667" topRight="0.25,0.16666667"
        let mut top_left = None;
        let mut top_right = None;
        let mut bottom_left = None;
        let mut bottom_right = None;
        
        // Extract attributes
        if let Some(tl) = Self::extract_attribute(line, "topLeft") {
            top_left = Some(Self::parse_coords(&tl)?);
        }
        if let Some(tr) = Self::extract_attribute(line, "topRight") {
            top_right = Some(Self::parse_coords(&tr)?);
        }
        if let Some(bl) = Self::extract_attribute(line, "bottomLeft") {
            bottom_left = Some(Self::parse_coords(&bl)?);
        }
        if let Some(br) = Self::extract_attribute(line, "bottomRight") {
            bottom_right = Some(Self::parse_coords(&br)?);
        }
        
        // Handle cells that don't have topLeft (first cell)
        if top_left.is_none() && top_right.is_some() && bottom_left.is_some() {
            // Calculate topLeft from bottomLeft
            if let (Some(tr), Some(bl)) = (top_right, bottom_left) {
                top_left = Some((bl.0, tr.1));
            }
        }
        
        if let (Some(tl), Some(tr), Some(bl), Some(br)) = (top_left, top_right, bottom_left, bottom_right) {
            Ok(Some(SpriteCell {
                top_left: tl,
                top_right: tr,
                bottom_left: bl,
                bottom_right: br,
            }))
        } else {
            Ok(None)
        }
    }
    
    fn extract_attribute(line: &str, attr_name: &str) -> Option<String> {
        let pattern = format!("{}=\"", attr_name);
        if let Some(start) = line.find(&pattern) {
            let start = start + pattern.len();
            if let Some(end) = line[start..].find('"') {
                return Some(line[start..start + end].to_string());
            }
        }
        None
    }
    
    fn parse_coords(coords_str: &str) -> Result<(f32, f32)> {
        let parts: Vec<&str> = coords_str.split(',').collect();
        if parts.len() != 2 {
            anyhow::bail!("Invalid coordinate format: {}", coords_str);
        }
        
        let x: f32 = parts[0].parse().context("Failed to parse X coordinate")?;
        let y: f32 = parts[1].parse().context("Failed to parse Y coordinate")?;
        
        Ok((x, y))
    }
    
    pub fn extract_frames(&self, texture: &DynamicImage) -> Result<Vec<RgbaImage>> {
        let rgba_texture = texture.to_rgba8();
        let (tex_width, tex_height) = rgba_texture.dimensions();
        let mut frames = Vec::new();
        
        for cell in &self.cells {
            // Convert UV coordinates to pixel coordinates
            let x1 = (cell.top_left.0 * tex_width as f32) as u32;
            let y1 = (cell.top_left.1 * tex_height as f32) as u32;
            let x2 = (cell.bottom_right.0 * tex_width as f32) as u32;
            let y2 = (cell.bottom_right.1 * tex_height as f32) as u32;
            
            let width = x2.saturating_sub(x1);
            let height = y2.saturating_sub(y1);
            
            if width == 0 || height == 0 {
                continue;
            }
            
            // Extract the sub-image
            let mut frame = ImageBuffer::new(width, height);
            
            for y in 0..height {
                for x in 0..width {
                    let src_x = x1 + x;
                    let src_y = y1 + y;
                    
                    if src_x < tex_width && src_y < tex_height {
                        let pixel = rgba_texture.get_pixel(src_x, src_y);
                        frame.put_pixel(x, y, *pixel);
                    }
                }
            }
            
            frames.push(frame);
        }
        
        Ok(frames)
    }
}