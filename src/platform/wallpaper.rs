use std::path::PathBuf;
use windows::{
    core::*,
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        UI::WindowsAndMessaging::*,
    },
};

const WALLPAPER_W: i32 = 1920;
const WALLPAPER_H: i32 = 1080;

pub fn apply_wallpaper(hex_color: &str) -> anyhow::Result<()> {
    let rgb = parse_hex(hex_color);
    let r = (rgb & 0xFF) as u8;
    let g = ((rgb >> 8) & 0xFF) as u8;
    let b = ((rgb >> 16) & 0xFF) as u8;

    let bmp = generate_gradient_bmp(r, g, b)?;
    let bmp_path = save_bmp(&bmp)?;

    unsafe {
        let wp_w: Vec<u16> = bmp_path.encode_utf16().chain(Some(0)).collect();
        let _ = SystemParametersInfoW(
            SPI_SETDESKWALLPAPER,
            0,
            Some(wp_w.as_ptr() as *mut _),
            SPIF_UPDATEINIFILE | SPIF_SENDCHANGE,
        );
    }

    Ok(())
}

fn generate_gradient_bmp(r: u8, g: u8, b: u8) -> anyhow::Result<Vec<u8>> {
    let w = WALLPAPER_W;
    let h = WALLPAPER_H;

    let row_size = ((w * 3 + 3) & !3) as usize;
    let data_size = row_size * h as usize;
    let file_size = 14 + 40 + data_size;

    let mut bmp = Vec::with_capacity(file_size);

    // BMP file header (14 bytes)
    bmp.extend_from_slice(b"BM");
    bmp.extend_from_slice(&(file_size as u32).to_le_bytes());
    bmp.extend_from_slice(&[0u8; 4]); // reserved
    bmp.extend_from_slice(&[54u8, 0, 0, 0]); // pixel data offset

    // DIB header (BITMAPINFOHEADER, 40 bytes)
    bmp.extend_from_slice(&[40u8, 0, 0, 0]); // header size
    bmp.extend_from_slice(&(w as i32).to_le_bytes());
    bmp.extend_from_slice(&(h as i32).to_le_bytes());
    bmp.extend_from_slice(&[1u8, 0]); // planes
    bmp.extend_from_slice(&[24u8, 0]); // bits per pixel
    bmp.extend_from_slice(&[0u8; 4]); // compression
    bmp.extend_from_slice(&[0u8; 4]); // image size
    bmp.extend_from_slice(&[0u8; 4]); // x ppm
    bmp.extend_from_slice(&[0u8; 4]); // y ppm
    bmp.extend_from_slice(&[0u8; 4]); // colors used
    bmp.extend_from_slice(&[0u8; 4]); // important colors

    // Pixel data (BGR, bottom-up)
    for y in 0..h {
        let t = y as f32 / h as f32;
        let cr = (r as f32 * (1.0 - t * 0.3)) as u8;
        let cg = (g as f32 * (1.0 - t * 0.3)) as u8;
        let cb = (b as f32 * (1.0 - t * 0.3)) as u8;

        for _x in 0..w {
            bmp.push(cb);
            bmp.push(cg);
            bmp.push(cr);
        }
        // Pad row to 4-byte boundary
        let padding = row_size - (w * 3) as usize;
        bmp.extend_from_slice(&vec![0u8; padding]);
    }

    Ok(bmp)
}

fn save_bmp(data: &[u8]) -> anyhow::Result<String> {
    let temp_dir = std::env::temp_dir();
    let bmp_path = temp_dir.join("ultrawm_wallpaper.bmp");

    std::fs::write(&bmp_path, data)?;
    Ok(bmp_path.to_string_lossy().to_string())
}

fn parse_hex(s: &str) -> u32 {
    let s = s.trim_start_matches('#');
    if s.len() == 6 {
        let r = u32::from_str_radix(&s[0..2], 16).unwrap_or(0);
        let g = u32::from_str_radix(&s[2..4], 16).unwrap_or(0);
        let b = u32::from_str_radix(&s[4..6], 16).unwrap_or(0);
        (0xFF << 24) | (b << 16) | (g << 8) | r
    } else if s.len() == 8 {
        u32::from_str_radix(s, 16).unwrap_or(0xFF000000)
    } else {
        0xFF7F7F7F
    }
}
