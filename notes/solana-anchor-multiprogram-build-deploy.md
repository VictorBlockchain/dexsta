# Solana Anchor Multi-Program Build & Deploy: Troubleshooting and Success Guide

## Overview
This guide documents the process, pitfalls, and solutions for building and deploying a multi-program Solana Anchor workspace (token, NFT, marketplace, vault, etc.) on macOS (Apple Silicon), with a focus on SBF compatibility and Anchor best practices. It is designed to be shared and reused for any similar NFT minting dapp with a vault, and to help you (or your AI assistant) quickly diagnose and resolve build/deploy issues.

---

## Environment & Versions
- **Solana CLI:** `solana-cli 2.2.19`
- **Anchor CLI:** `anchor-cli 0.31.1`
- **Rust:** `rustc 1.88.0 (6b00bc388 2025-06-23)`
- **anchor-lang:** `0.31.1`
- **anchor-spl:** `0.31.1`
- **OS:** macOS (Apple Silicon)

---

## Toolkit: Solana Environment Management
To ensure a clean, reproducible environment, the **Solana Toolkit** (such as [`mucho`](https://github.com/Block-Logic-Technology/mucho) or similar) was used. This toolkit automates the installation and management of:
- Solana CLI
- Anchor CLI
- Rust toolchain (with SBF support)
- Other dependencies (Node, npm, pnpm, etc.)

**Key commands:**
- Install the toolkit (example):
  ```sh
  curl -fsSL https://raw.githubusercontent.com/Block-Logic-Technology/mucho/main/install.sh | bash
  # or follow the toolkit's official instructions
  ```
- Use the toolkit to install/update all required tools:
  ```sh
  mucho install
  mucho update
  mucho doctor
  ```
- The toolkit ensures all binaries are on the correct version and in your PATH, reducing version mismatch issues.

**Why use a toolkit?**
- Handles all environment setup in one place.
- Ensures compatibility between Solana, Anchor, and Rust.
- Simplifies upgrades and troubleshooting.
- Great for onboarding new developers or resetting a broken environment.

---

## Project & Workspace Structure
- **Monorepo**: All programs are in the `programs/` directory.
- **Programs**: Each program (e.g., `token_program`, `nft_program`, `marketplace_program`, `vault_program`, `nft_oneofone`) has its own `Cargo.toml` and `src/lib.rs`.
- **Workspace management**: Use `pnpm`, `Cargo`, and `Anchor` for dependency and build orchestration.
- **Tests**: Place integration tests in the `tests/` directory (e.g., `tests/solanatemplate.ts`).

---

## Key Troubleshooting Steps & Common Errors

### 1. **Entrypoint out of bounds**
- **Meaning**: The Solana loader cannot find a valid entrypoint in your `.so` file. This is almost always a build artifact/toolchain/configuration issue, not a code logic issue.
- **Fix**:
  - Ensure you use `cargo build-sbf` (not `cargo build` or only `anchor build`).
  - Check `[features]` in `Cargo.toml` matches the Anchor template (see below).
  - Ensure a valid `declare_id!` in `src/lib.rs` (not the all-ones placeholder).
  - Clean and rebuild: `cargo clean && cargo build-sbf`.
  - If on Apple Silicon, avoid Docker if possible (slow, unreliable); use native SBF build or a Linux VM if needed.

### 2. **Proc-macro panicked: failed to load macro**
- **Meaning**: Your editor/linter is trying to use Anchor macros in a native build context. This does **not** affect SBF builds or deployment.
- **Fix**: Ignore for SBF builds. Only relevant for native builds.

### 3. **Other build/deploy errors**
- Check for version mismatches (Solana, Anchor, Rust, anchor-lang, anchor-spl).
- Ensure all programs have a unique, valid program ID and keypair.
- Make sure your workspace manifest includes all programs.

---

## How to Verify SBF Output
- After building, check the `.so` file:
  ```sh
  file target/deploy/your_program.so
  # Should output: ELF 64-bit LSB shared object, *unknown arch 0x107*
  ```
- If it says `Mach-O` or `x86_64`, you built natively—**not** for SBF!

---

## How to Check Program IDs
- Generate a keypair:
  ```sh
  solana-keygen new -o target/deploy/your_program-keypair.json --no-bip39-passphrase
  solana-keygen pubkey target/deploy/your_program-keypair.json
  # Use this pubkey in declare_id!("...") in src/lib.rs
  ```

---

## Cargo.toml Features Section (Critical!)
```toml
[features]
no-entrypoint = []
no-idl = []
no-log-ix-name = []
cpi = ["no-entrypoint"]
default = []
idl-build = ["anchor-lang/idl-build", "anchor-spl/idl-build"]
```
- All programs must match this structure for SBF compatibility.

---

## Clean Build & Deploy Checklist
1. Use the toolkit to set up your environment.
2. Ensure all program `Cargo.toml` files have the correct `[features]` section.
3. Generate and set valid program IDs for each program.
4. Run:
   ```sh
   cargo clean && cargo build-sbf
   anchor deploy
   ```
5. Check `.so` files with `file` command.
6. If any program fails to deploy, check its build output, program ID, and Cargo.toml.

---

## What to Provide When Asking for Help
If you need to ask for help (in another chat or with an AI assistant), provide:
- The full error message(s) and command(s) you ran.
- The output of `file target/deploy/your_program.so` for the failing program.
- The full `Cargo.toml` for the failing program.
- The `declare_id!` line from `src/lib.rs`.
- Your Solana, Anchor, and Rust versions (`solana --version`, `anchor --version`, `rustc --version`).
- A summary of your workspace structure (list of programs, how you build, etc.).
- Any toolkit or environment manager you used (e.g., mucho).

---

## Example Cargo.toml (for all programs)
```
[package]
name = "your_program"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "lib"]
name = "your_program"

[features]
no-entrypoint = []
no-idl = []
no-log-ix-name = []
cpi = ["no-entrypoint"]
default = []
idl-build = ["anchor-lang/idl-build", "anchor-spl/idl-build"]

[dependencies]
anchor-lang = "0.31.1"
anchor-spl = "0.31.1"
```

---

## Final Working Example
- All programs deployed successfully after aligning the `[features]` section and using `cargo build-sbf`.
- Anchor, Solana, and Rust versions are critical—keep them up to date and consistent across the workspace.

---

## Commands Used
- `cargo build-sbf`
- `cargo clean && cargo build-sbf`
- `anchor deploy`
- `solana-keygen new -o target/deploy/your_program-keypair.json --no-bip39-passphrase`
- `file target/deploy/your_program.so`
- `anchor --version`, `solana --version`, `rustc --version`

---

## Using `mucho validator` for Local Solana Development

The `mucho` toolkit provides a convenient wrapper for running the Solana test validator with multi-program support. Here are best practices and troubleshooting tips for using `mucho validator` in a multi-program Anchor workspace:

### Why Use `mucho validator`?
- **Unified workflow:** Runs the Solana test validator with all your compiled programs auto-loaded.
- **No Docker required:** Native, fast, and reliable on macOS (including Apple Silicon).
- **Auto-discovers programs:** Scans your workspace for `.so` files and loads them into the validator.
- **Solana Explorer integration:** Provides a local explorer URL for easy inspection.

### How to Start the Validator
```sh
mucho validator
```
- This will start `solana-test-validator` and auto-load all programs found in your workspace's `target/deploy/` directory.
- The output will show which programs are loaded (e.g., `Compiled program 'nft_oneofone' was found with no config info`).
- The ledger and logs are stored in the `test-ledger` directory by default.

### Differences from `anchor localnet`
- `anchor localnet` also starts a validator, but is tightly coupled to Anchor's test runner and may have port conflicts or slower startup on macOS.
- `mucho validator` is more flexible and works well with custom test runners (e.g., `ts-mocha`, `jest`).
- Both use the same default RPC port (`8899`). **Do not run both at the same time.**

### Port Conflicts & Troubleshooting
- If you see `Error: Your configured rpc port: 8899 is already in use`, another validator is running.
- Stop all validators:
  - Find running processes: `ps aux | grep solana-test-validator`
  - Kill them: `kill -9 <PID>`
- Or, use `lsof -i :8899` to find and kill the process using the port.
- Only one validator should run at a time on a given port.

### Program Auto-Discovery
- `mucho validator` looks for `.so` files in `target/deploy/`.
- Ensure you have built your programs with `cargo build-sbf` or `anchor build` before starting the validator.
- If you add or rebuild programs, restart the validator to reload them.
- The validator will print which programs it loads at startup.

### Best Practices
- Always clean and rebuild (`cargo clean && cargo build-sbf`) before starting the validator for a fresh test environment.
- Use the provided Solana Explorer URL to inspect accounts, transactions, and program logs.
- Set your environment variables for tests:
  - `ANCHOR_PROVIDER_URL=http://127.0.0.1:8899`
  - `ANCHOR_WALLET=~/.config/solana/id.json` (or your test wallet)
- For integration tests, use `ts-mocha` or your preferred runner, and point to the correct validator URL.
- If you encounter issues with program loading, check the validator logs in `test-ledger/validator.log`.

### Example Workflow
```sh
# 1. Clean and build all programs
cargo clean && cargo build-sbf

# 2. Start the validator
mucho validator

# 3. In a new terminal, run your tests
export ANCHOR_PROVIDER_URL=http://127.0.0.1:8899
export ANCHOR_WALLET=~/.config/solana/id.json
pnpm test  # or ts-mocha tests/*.ts
```

### Stopping the Validator
- Use `Ctrl+C` in the terminal running `mucho validator`.
- Or, kill the process as described above.

---

## Conclusion
If you follow the above structure and steps, you will have a reliable, reproducible build and deploy process for any multi-program Solana Anchor workspace, even on macOS/Apple Silicon. 