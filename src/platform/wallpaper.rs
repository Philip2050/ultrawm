use std::path::PathBuf;
use windows::{
    core::*,
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        UI::WindowsAndMessaging::*,
    },
};

pub fn apply_wallpaper_monitor(hex_color: &str, monitor_idx: usize, width: i32, height: i32) -> anyhow::Result<()> {
    let rgb = parse_hex(hex_color);
    let r = (rgb & 0xFF) as u8;
    let g = ((rgb >> 8) & 0xFF) as u8;
    let b = ((rgb >> 16) & 0xFF) as u8;

    let bmp = generate_gradient_bmp(r, g, b, width, height)?;
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

pub fn apply_wallpaper_image_monitor(path: &str) -> anyhow::Result<()> {
    unsafe {
        let wp_w: Vec<u16> = path.encode_utf16().chain(Some(0)).collect();
        let _ = SystemParametersInfoW(
            SPI_SETDESKWALLPAPER,
            0,
            Some(wp_w.as_ptr() as *mut _),
            SPIF_UPDATEINIFILE | SPIF_SENDCHANGE,
        );
    }

    Ok(())
}

pub fn apply_wallpaper(hex_color: &str, width: i32, height: i32) -> anyhow::Result<()> {
    let rgb = parse_hex(hex_color);
    let r = (rgb & 0xFF) as u8;
    let g = ((rgb >> 8) & 0xFF) as u8;
    let b = ((rgb >> 16) & 0xFF) as u8;

    let bmp = generate_gradient_bmp(r, g, b, width, height)?;
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

pub fn apply_wallpaper_image(path: &str) -> anyhow::Result<()> {
    unsafe {
        let wp_w: Vec<u16> = path.encode_utf16().chain(Some(0)).collect();
        let _ = SystemParametersInfoW(
            SPI_SETDESKWALLPAPER,
            0,
            Some(wp_w.as_ptr() as *mut _),
            SPIF_UPDATEINIFILE | SPIF_SENDCHANGE,
        );
    }
    Ok(())
}

pub fn apply_theme_wallpaper(background: &str, accent: &str, width: i32, height: i32) -> anyhow::Result<()> {
    let bg = parse_hex(background);
    let ac = parse_hex(accent);
    let bg_r = (bg & 0xFF) as u8;
    let bg_g = ((bg >> 8) & 0xFF) as u8;
    let bg_b = ((bg >> 16) & 0xFF) as u8;
    let ac_r = (ac & 0xFF) as u8;
    let ac_g = ((ac >> 8) & 0xFF) as u8;
    let ac_b = ((ac >> 16) & 0xFF) as u8;

    let bmp = generate_accent_wallpaper(bg_r, bg_g, bg_b, ac_r, ac_g, ac_b, width, height)?;
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

fn generate_gradient_bmp(r: u8, g: u8, b: u8, w: i32, h: i32) -> anyhow::Result<Vec<u8>> {
    let row_size = ((w * 3 + 3) & !3) as usize;
    let data_size = row_size * h as usize;
    let file_size = 14 + 40 + data_size;

    let mut bmp = Vec::with_capacity(file_size);

    bmp.extend_from_slice(b"BM");
    bmp.extend_from_slice(&(file_size as u32).to_le_bytes());
    bmp.extend_from_slice(&[0u8; 4]);
    bmp.extend_from_slice(&[54u8, 0, 0, 0]);

    bmp.extend_from_slice(&[40u8, 0, 0, 0]);
    bmp.extend_from_slice(&(w as i32).to_le_bytes());
    bmp.extend_from_slice(&(h as i32).to_le_bytes());
    bmp.extend_from_slice(&[1u8, 0]);
    bmp.extend_from_slice(&[24u8, 0]);
    bmp.extend_from_slice(&[0u8; 4]);
    bmp.extend_from_slice(&[0u8; 4]);
    bmp.extend_from_slice(&[0u8; 4]);
    bmp.extend_from_slice(&[0u8; 4]);
    bmp.extend_from_slice(&[0u8; 4]);
    bmp.extend_from_slice(&[0u8; 4]);

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
        let padding = row_size - (w * 3) as usize;
        bmp.extend_from_slice(&vec![0u8; padding]);
    }

    Ok(bmp)
}

fn generate_accent_wallpaper(
    bg_r: u8, bg_g: u8, bg_b: u8,
    ac_r: u8, ac_g: u8, ac_b: u8,
    w: i32, h: i32,
) -> anyhow::Result<Vec<u8>> {
    let row_size = ((w * 3 + 3) & !3) as usize;
    let data_size = row_size * h as usize;
    let file_size = 14 + 40 + data_size;

    let mut bmp = Vec::with_capacity(file_size);

    bmp.extend_from_slice(b"BM");
    bmp.extend_from_slice(&(file_size as u32).to_le_bytes());
    bmp.extend_from_slice(&[0u8; 4]);
    bmp.extend_from_slice(&[54u8, 0, 0, 0]);

    bmp.extend_from_slice(&[40u8, 0, 0, 0]);
    bmp.extend_from_slice(&(w as i32).to_le_bytes());
    bmp.extend_from_slice(&(h as i32).to_le_bytes());
    bmp.extend_from_slice(&[1u8, 0]);
    bmp.extend_from_slice(&[24u8, 0]);
    bmp.extend_from_slice(&[0u8; 4]);
    bmp.extend_from_slice(&[0u8; 4]);
    bmp.extend_from_slice(&[0u8; 4]);
    bmp.extend_from_slice(&[0u8; 4]);
    bmp.extend_from_slice(&[0u8; 4]);
    bmp.extend_from_slice(&[0u8; 4]);

    for y in 0..h {
        let t = y as f32 / h as f32;
        for x in 0..w {
            let s = x as f32 / w as f32;
            // Diagonal gradient from background to accent
            let blend = ((t + s) / 2.0).clamp(0.0, 1.0);
            let cr = (bg_r as f32 * (1.0 - blend) + ac_r as f32 * blend) as u8;
            let cg = (bg_g as f32 * (1.0 - blend) + ac_g as f32 * blend) as u8;
            let cb = (bg_b as f32 * (1.0 - blend) + ac_b as f32 * blend) as u8;
            bmp.push(cb);
            bmp.push(cg);
            bmp.push(cr);
        }
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
