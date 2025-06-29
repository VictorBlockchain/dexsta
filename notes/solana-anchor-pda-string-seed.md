# Using a String as a PDA Seed in Anchor

## Problem
You want to enforce on-chain uniqueness for a string (e.g., a title) using a PDA in Anchor. You tried to add a field like `pub title_seed: [u8; 32]` to your context struct, but Anchor gives an `invalid account type given` error, even if it's the first field.

## Solution
**Do not** add a non-account field to your context struct. Instead, use the `#[instruction(...)]` macro to reference instruction arguments directly in your PDA seeds.

### Example
```rust
#[derive(Accounts)]
#[instruction(title: String)]
pub struct MintXft<'info> {
    #[account(
        init_if_needed,
        payer = payer,
        space = 8 + 8 + 1,
        seeds = [b"title_lookup", title.as_bytes()],
        bump
    )]
    pub title_lookup: Account<'info, TitleLookup>,
    // ... other accounts ...
}

pub fn mint_xft(
    ctx: Context<MintXft>,
    title: String,
    // ...
) -> Result<()> {
    // ...
}
```

### Key Points
- Use `#[instruction(title: String)]` on your context struct.
- Reference the instruction argument in the `seeds` array: `seeds = [b"title_lookup", title.as_bytes()]`.
- Do **not** add a `[u8; 32]` or `Vec<u8>` field to your context struct.
- The client must pass the correct PDA for `title_lookup` using the same seeds.

### References
- [Anchor PDAs and Accounts (Solana course)](https://solana.com/developers/courses/onchain-development/anchor-pdas#pdas-with-anchor)
- [Anchor Book: Program Derived Address](https://www.anchor-lang.com/docs/basics/pda) 