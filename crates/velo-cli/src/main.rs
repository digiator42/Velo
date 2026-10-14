use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;

mod diag;

#[derive(Parser)]
#[command(
    name = "veloasm",
    about = "veloasm — a fine-grained reactive Rust→WASM SPA framework",
    long_about = "veloasm is a fine-grained reactive Rust→WASM SPA framework with a tiny wasm core.\n\
                  Use `veloasm new` to scaffold, `veloasm dev` to run the dev server, `veloasm build` for production."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Scaffold a new veloasm app from the template
    New {
        /// Name of the new app (used as directory name under examples/)
        name: String,
    },
    /// Start the dev server (trunk serve --watch)
    Dev {
        /// Example name (defaults to current directory)
        name: Option<String>,
    },
    /// Build for production (trunk build --release)
    Build {
        /// Example name (defaults to current directory)
        name: Option<String>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::New { name } => cmd_new(&name),
        Commands::Dev { name } => cmd_dev(name.as_deref()),
        Commands::Build { name } => cmd_build(name.as_deref()),
    }
}

fn cmd_new(name: &str) -> Result<()> {
    // Validate name
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        anyhow::bail!(
            "Invalid name '{}': use only alphanumeric, dash, or underscore",
            name
        );
    }

    let workspace_root = workspace_root()?;
    let template_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("cli-template");
    let target_dir = workspace_root.join("examples").join(name);

    if target_dir.exists() {
        anyhow::bail!("{} already exists", target_dir.display());
    }

    // Copy template
    copy_dir_all(&template_dir, &target_dir)
        .with_context(|| format!("Failed to copy template to {}", target_dir.display()))?;

    // Patch Cargo.toml: replace template name with user-chosen name
    let cargo_toml = target_dir.join("Cargo.toml");
    let content = fs::read_to_string(&cargo_toml)?;
    let content = content.replace("cli-template", name);
    fs::write(&cargo_toml, content)?;

    // Patch lib.rs: replace the template's run_app call if needed
    let lib_rs = target_dir.join("src").join("lib.rs");
    if lib_rs.exists() {
        let content = fs::read_to_string(&lib_rs)?;
        let content = content.replace("cli-template", name);
        fs::write(&lib_rs, content)?;
    }

    println!("  Created veloasm app '{}' at {}", name, target_dir.display());
    println!("  Next steps:");
    println!("    cd examples/{}", name);
    println!("    veloasm dev");
    Ok(())
}

fn cmd_dev(name: Option<&str>) -> Result<()> {
    let dir = resolve_example_dir(name)?;
    let index = dir.join("index.html");
    if !index.exists() {
        anyhow::bail!(
            "No index.html found in {}. Are you in a veloasm project?",
            dir.display()
        );
    }

    println!("  Starting dev server for {}...", index.display());
    println!("  (Ctrl+C to stop)");

    // Pipe Trunk's stderr so rustc diagnostics can be parsed for the dev
    // overlay's full file:line:col panel (Trunk's own WebSocket only carries
    // a pipeline summary). Terminal output is echoed through untouched.
    // Run Trunk from the project directory: it resolves the target crate
    // relative to its working directory, not the index.html path.
    let mut child = Command::new("trunk")
        .arg("serve")
        .arg("--watch")
        .arg(&index)
        .current_dir(&dir)
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| "Failed to run trunk serve. Is trunk installed? (cargo install trunk)")?;

    let diags = diag::DiagState::new();
    match diag::serve(Arc::clone(&diags)) {
        Ok(()) => println!(
            "  Diagnostics sidecar at http://127.0.0.1:{}/_veloasm/diagnostics",
            diag::DIAG_PORT
        ),
        Err(e) => eprintln!(
            "  (diagnostics sidecar unavailable: {e}; overlay falls back to Trunk's summary)"
        ),
    }
    if let Some(stderr) = child.stderr.take() {
        diag::pump_trunk_stderr(stderr, diags);
    }

    let status = child
        .wait()
        .with_context(|| "Failed to wait on trunk serve")?;

    if !status.success() {
        anyhow::bail!("trunk serve exited with status {}", status);
    }
    Ok(())
}

fn cmd_build(name: Option<&str>) -> Result<()> {
    let dir = resolve_example_dir(name)?;
    let index = dir.join("index.html");
    if !index.exists() {
        anyhow::bail!(
            "No index.html found in {}. Are you in a veloasm project?",
            dir.display()
        );
    }

    println!("  Building {} for production...", index.display());

    let status = Command::new("trunk")
        .arg("build")
        .arg("--release")
        .arg(&index)
        .status()
        .with_context(|| "Failed to run trunk build. Is trunk installed? (cargo install trunk)")?;

    if !status.success() {
        anyhow::bail!("trunk build exited with status {}", status);
    }

    println!("  Build complete. Output in {}", dir.join("dist").display());
    Ok(())
}

fn resolve_example_dir(name: Option<&str>) -> Result<PathBuf> {
    match name {
        Some(n) => Ok(workspace_root()?.join("examples").join(n)),
        None => std::env::current_dir().context("Failed to determine current directory"),
    }
}

fn workspace_root() -> Result<PathBuf> {
    // CARGO_MANIFEST_DIR for veloasm-cli is crates/velo-cli, so go up two levels.
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    Ok(manifest.parent().unwrap().parent().unwrap().to_path_buf())
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let dst_path = dst.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_all(&entry.path(), &dst_path)?;
        } else {
            fs::copy(&entry.path(), &dst_path)?;
        }
    }
    Ok(())
}


