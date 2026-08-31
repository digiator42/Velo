# Installation & Toolchain Setup

To develop WebAssembly applications with Velo, you will need the standard Rust toolchain with the `wasm32-unknown-unknown` target, and [Trunk](https://trunkrs.dev) as the development server and build tool.

---

## 1. Rust Toolchain

Make sure you have Rust installed via [rustup](https://rustup.rs/):

```bash
# Check rustc version (Rust 1.75+ recommended)
rustc --version
```

Add the `wasm32-unknown-unknown` compilation target:

```bash
rustup target add wasm32-unknown-unknown
```

---

## 2. Installing Trunk

**Trunk** is a WASM web application bundler for Rust. It handles compiling Rust to WASM, binding JS interfaces via `wasm-bindgen`, copying HTML/CSS assets, and providing a local development server with hot reload.

Install Trunk using Cargo:

```bash
cargo install trunk
```

Verify the installation:

```bash
trunk --version
```

---

## 3. Adding Velo to Your Project

In your `Cargo.toml`, configure the crate type to `cdylib` and add `velo` as a dependency:

```toml
[package]
name = "my-velo-app"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
velo = { version = "0.1.0" } # Or path = "../crates/velo"
wasm-bindgen = "0.2"
web-sys = "0.3"
```

### Release Profile Configuration

To get minimal WebAssembly binary sizes in production, add this profile to your root `Cargo.toml`:

```toml
[profile.release]
opt-level = "z"   # Optimize for size
lto = true        # Enable Link-Time Optimization
codegen-units = 1 # Maximize optimization units
panic = "abort"   # Strip panic formatting strings
```
