#![windows_subsystem = "windows"]

use anyhow::Result;
use clap::Parser;
use log::info;
use std::sync::mpsc;

mod anim;
mod config;
mod ipc;
mod layout;
mod platform;
mod session;
mod theme;

use config::Config;
use ipc::{start_ipc_server, IpcCommand};
use platform::Platform;
use theme::ThemeManager;

#[derive(Parser, Debug)]
#[command(author, version, about = "UltraWM — ultimate tiling window manager for Windows")]
struct Args {
    /// Start the daemon
    #[arg(short, long)]
    start: bool,

    /// Run diagnostics
    #[arg(short, long)]
    doctor: bool,

    /// Switch to next theme
    #[arg(short, long)]
    theme: bool,

    /// Show current theme
    #[arg(long)]
    show_theme: bool,

    /// List available themes
    #[arg(long)]
    list_themes: bool,

    /// Install UltraWM as the default shell (replaces Explorer)
    #[arg(long)]
    install: bool,

    /// Uninstall UltraWM as shell (restores Explorer)
    #[arg(long)]
    uninstall: bool,
}

fn main() -> Result<()> {
    env_logger::init();
    let args = Args::parse();

    if args.doctor {
        return run_doctor();
    }

    if args.list_themes {
        let themes = ThemeManager::list_themes()?;
        for t in themes {
            println!("{}", t);
        }
        return Ok(());
    }

    if args.show_theme {
        let tm = ThemeManager::load()?;
        println!("Current theme: {:?}", tm.current_theme().name);
        return Ok(());
    }

    if args.theme {
        let mut tm = ThemeManager::load()?;
        tm.next_theme()?;
        println!("Switched to theme: {:?}", tm.current_theme().name);
        return Ok(());
    }

    if args.install {
        return install_shell();
    }

    if args.uninstall {
        return uninstall_shell();
    }

    // Default: start daemon
    if args.start {
        return run_daemon();
    }

    println!("UltraWM v{}", env!("CARGO_PKG_VERSION"));
    println!("Usage: ultrawm --start | --doctor | --theme | --list-themes | --show-theme | --install | --uninstall");
    Ok(())
}

fn run_doctor() -> Result<()> {
    println!("=== UltraWM Doctor ===");
    let platform = Platform::new()?;
    platform.diagnose()?;
    Ok(())
}

fn install_shell() -> Result<()> {
    use winreg::enums::*;
    use std::env::current_exe;

    let exe_path = current_exe()?;
    let exe_str = exe_path.to_string_lossy().to_string();

    println!("Installing UltraWM as shell...");
    println!("Path: {}", exe_str);

    // Set HKCU shell (user-level, no admin needed)
    let hkcu = winreg::RegKey::predef(HKEY_CURRENT_USER);
    let key = hkcu.open_subkey_with_flags(
        "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Winlogon",
        KEY_SET_VALUE,
    )?;

    // Backup current shell
    let current: String = key.get_value("Shell").unwrap_or_else(|_| "explorer.exe".to_string());
    key.set_value("ShellBackup", &current)?;

    // Set UltraWM as shell
    key.set_value("Shell", &exe_str)?;

    println!("Shell set to: {}", exe_str);
    println!("Previous shell backed up. Restart or run 'ultrawm --uninstall' to restore Explorer.");
    Ok(())
}

fn uninstall_shell() -> Result<()> {
    use winreg::enums::*;

    println!("Uninstalling UltraWM as shell...");

    let hkcu = winreg::RegKey::predef(HKEY_CURRENT_USER);
    let key = hkcu.open_subkey_with_flags(
        "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Winlogon",
        KEY_SET_VALUE | KEY_QUERY_VALUE,
    )?;

    // Restore backup or default to explorer
    let shell = key.get_value::<String, _>("ShellBackup")
        .unwrap_or_else(|_| "explorer.exe".to_string());

    key.set_value("Shell", &shell)?;
    let _ = key.delete_value("ShellBackup");

    println!("Shell restored to: {}", shell);
    println!("Restart to apply changes.");
    Ok(())
}

fn run_daemon() -> Result<()> {
    info!("UltraWM daemon starting...");

    let config = Config::load()?;
    let mut platform = Platform::new()?;
    let mut theme_mgr = ThemeManager::load()?;

    platform.initialize(&config)?;

    // Start IPC named pipe server
    let (ipc_tx, ipc_rx) = mpsc::channel::<IpcCommand>();
    let _ipc_handle = start_ipc_server(ipc_tx)?;
    info!("IPC server started on \\\\.\\pipe\\ultrawm-ipc");

    platform.run_event_loop(&mut theme_mgr, Some(ipc_rx))?;

    Ok(())
}
