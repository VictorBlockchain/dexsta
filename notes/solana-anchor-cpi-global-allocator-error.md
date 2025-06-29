# Preventing #[global_allocator] Conflict in Anchor CPI

## Problem
When making Cross-Program Invocations (CPI) between Anchor programs, you may encounter this error:

```
error: the #[global_allocator] in this crate conflicts with global allocator in: <other program>
```

This happens even if you do **not** explicitly declare a global allocator in your code.

## Why This Happens
- Adding another Anchor program as a dependency in your `Cargo.toml` (even with `features = ["cpi"]`) causes Rust to link both programs' entrypoints and global allocators.
- Solana BPF programs can only have one global allocator and one entrypoint. The linker will fail if it finds more than one.
- This is a linker-level conflict, not a code-level one.

## How to Prevent This Error

### 1. **Do NOT add the other program as a dependency in Cargo.toml**
- Never add another Solana program as a dependency in your `Cargo.toml` for on-chain code, even with `features = ["cpi"]`.
- Instead, use Anchor's `declare_program!()` macro and the target program's IDL.

### 2. **Use `declare_program!()` and the IDL for dependency-free CPI**
- Place the target program's IDL (e.g., `minter.json`) in an `/idls` directory in your project.
- In your Rust code, use:
  ```rust
  declare_program!(minter);
  use minter::cpi::{self, accounts::SomeCpiAccounts};
  use minter::program::Minter;
  ```
- This generates the CPI interface for you **without linking the other program's crate**.

### 3. **For shared types/constants**
- If you need to share types, create a separate crate with only types (no program logic, no entrypoint, no global allocator).
- Both programs can depend on this types-only crate.

## References
- [Anchor Docs: Dependency Free Composability](https://www.anchor-lang.com/docs/features/declare-program)
- [Solana StackExchange: entrypoint/global_allocator conflicts](https://solana.stackexchange.com/questions/5546/entrypoint-function-is-conflicting-with-the-solana-program-entrypoint-function)
- [Anchor CPI and Errors](https://solana.com/developers/courses/onchain-development/anchor-cpi)

## Summary
- **Remove** the other program from your `Cargo.toml` dependencies.
- **Use** the `declare_program!()` macro and the IDL for CPI.
- **If you need types**, use a types-only crate.

This approach will prevent the global allocator conflict and allow you to safely make CPI calls between Anchor programs. 