// src/lib.rs
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Mint, Transfer as SplTransfer};
use anchor_spl::associated_token::AssociatedToken;

declare_id!("G1BVSiFojnXFaPG1WUgJAcYaB7aGKLKWtSqhMreKgA82");

#[program]
pub mod stealth_swap {
    use super::*;

    /*----------------------------------------------------------
     * 1.  USDC → XMR : Alice locks USDC for Bob
     *---------------------------------------------------------*/
    pub fn create_usdc_to_xmr_swap(
        ctx: Context<CreateUsdcToXmr>,
        swap_id: [u8; 32],
        secret_hash: [u8; 32],
        usdc_amount: u64,
        xmr_amount: u64,
        monero_sub_address: [u8; 64],
        expiry: i64,
        relayer_fee: u64,
    ) -> Result<()> {
        let swap = &mut ctx.accounts.swap;
        swap.direction          = Direction::UsdcToXmr;
        swap.swap_id            = swap_id;
        swap.alice              = *ctx.accounts.alice.key;
        swap.bob                = *ctx.accounts.bob.key;
        swap.secret_hash        = secret_hash;
        swap.expiry             = expiry;
        swap.relayer_fee        = relayer_fee;
        swap.is_redeemed        = false;
        swap.is_refunded        = false;
        swap.usdc_amount        = usdc_amount;
        swap.xmr_amount         = xmr_amount;
        swap.monero_sub_address = monero_sub_address;
        swap.monero_lock_txid   = [0; 32];
        swap.bump               = ctx.bumps.swap;

        let cpi_accounts = SplTransfer {
            from: ctx.accounts.alice_usdc.to_account_info(),
            to:   ctx.accounts.vault_usdc.to_account_info(),
            authority: ctx.accounts.alice.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);
        token::transfer(cpi_ctx, usdc_amount)?;

        msg!("USDC→XMR swap {:?}", &swap_id[..8]);
        Ok(())
    }

    pub fn record_monero_lock_proof(
        ctx: Context<RecordProof>,
        _swap_id: [u8; 32],
        monero_lock_txid: [u8; 32],
    ) -> Result<()> {
        let swap = &mut ctx.accounts.swap;
        require!(swap.direction == Direction::UsdcToXmr, ErrorCode::WrongDirection);
        swap.monero_lock_txid = monero_lock_txid;
        msg!("Monero lock txid recorded");
        Ok(())
    }

pub fn redeem_usdc(
    ctx: Context<RedeemUsdc>,
    _swap_id: [u8; 32],
    adaptor_sig: Vec<u8>,
) -> Result<()> {
    let swap = &mut ctx.accounts.swap;
    require!(!swap.is_redeemed && !swap.is_refunded, ErrorCode::AlreadyFinalized);
    require!(swap.direction == Direction::UsdcToXmr, ErrorCode::WrongDirection);
    require!(adaptor_sig.len() == 64, ErrorCode::InvalidAdaptorSig);

    // copy values before mutable use
    let swap_bump   = swap.bump;
    let swap_id     = swap.swap_id;
    let relayer_fee = swap.relayer_fee;

    let vault_balance = ctx.accounts.vault_usdc.amount;
    let to_bob        = vault_balance.checked_sub(relayer_fee).unwrap();

    // drop mutable borrow
    drop(swap);

    // build seeds
    let seeds = &[b"swap", swap_id.as_ref(), &[swap_bump]];
    let signer_seeds = &[&seeds[..]];

    // CPI 1: relayer fee
    if relayer_fee > 0 {
        let cpi_accounts = SplTransfer {
            from: ctx.accounts.vault_usdc.to_account_info(),
            to:   ctx.accounts.relayer_token.to_account_info(),
            authority: ctx.accounts.swap.to_account_info(),
        };
        token::transfer(
            CpiContext::new_with_signer(ctx.accounts.token_program.to_account_info(), cpi_accounts, signer_seeds),
            relayer_fee,
        )?;
    }

    // CPI 2: remainder to Bob
    let cpi_accounts = SplTransfer {
        from: ctx.accounts.vault_usdc.to_account_info(),
        to:   ctx.accounts.bob_token.to_account_info(),
        authority: ctx.accounts.swap.to_account_info(),
    };
    token::transfer(
        CpiContext::new_with_signer(ctx.accounts.token_program.to_account_info(), cpi_accounts, signer_seeds),
        to_bob,
    )?;

    // mark redeemed
    let swap = &mut ctx.accounts.swap;
    swap.is_redeemed = true;

    msg!("USDC redeemed by Bob");
    Ok(())
}

    /*----------------------------------------------------------
     * 2.  XMR → USDC : Bob locks USDC, Alice reveals secret
     *---------------------------------------------------------*/
    pub fn create_xmr_to_usdc_swap(
        ctx: Context<CreateXmrToUsdc>,
        swap_id: [u8; 32],
        secret_hash: [u8; 32],
        usdc_amount: u64,
        xmr_amount: u64,
        alice_solana: Pubkey,
        expiry: i64,
        relayer_fee: u64,
    ) -> Result<()> {
        let swap = &mut ctx.accounts.swap;
        swap.direction    = Direction::XmrToUsdc;
        swap.swap_id      = swap_id;
        swap.alice        = *ctx.accounts.alice.key;
        swap.bob          = *ctx.accounts.bob.key;
        swap.secret_hash  = secret_hash;
        swap.expiry       = expiry;
        swap.relayer_fee  = relayer_fee;
        swap.is_redeemed  = false;
        swap.is_refunded  = false;
        swap.usdc_amount  = usdc_amount;
        swap.xmr_amount   = xmr_amount;
        swap.alice_solana = alice_solana;
        swap.bump         = ctx.bumps.swap;

        msg!("XMR→USDC swap {:?}", &swap_id[..8]);
        Ok(())
    }

pub fn redeem_usdc_alice(
    ctx: Context<RedeemUsdcAlice>,
    _swap_id: [u8; 32],
    adaptor_sig: Vec<u8>,
) -> Result<()> {
    let swap = &mut ctx.accounts.swap;
    require!(!swap.is_redeemed && !swap.is_refunded, ErrorCode::AlreadyFinalized);
    require!(swap.direction == Direction::XmrToUsdc, ErrorCode::WrongDirection);
    require!(adaptor_sig.len() == 64, ErrorCode::InvalidAdaptorSig);

    let swap_bump   = swap.bump;
    let swap_id     = swap.swap_id;
    let relayer_fee = swap.relayer_fee;

    let vault_balance = ctx.accounts.vault_usdc.amount;
    let to_alice      = vault_balance.checked_sub(relayer_fee).unwrap();

    drop(swap);

    let seeds = &[b"swap", swap_id.as_ref(), &[swap_bump]];
    let signer_seeds = &[&seeds[..]];

    if relayer_fee > 0 {
        let cpi_accounts = SplTransfer {
            from: ctx.accounts.vault_usdc.to_account_info(),
            to:   ctx.accounts.relayer_token.to_account_info(),
            authority: ctx.accounts.swap.to_account_info(),
        };
        token::transfer(
            CpiContext::new_with_signer(ctx.accounts.token_program.to_account_info(), cpi_accounts, signer_seeds),
            relayer_fee,
        )?;
    }

    let cpi_accounts = SplTransfer {
        from: ctx.accounts.vault_usdc.to_account_info(),
        to:   ctx.accounts.alice_token.to_account_info(),
        authority: ctx.accounts.swap.to_account_info(),
    };
    token::transfer(
        CpiContext::new_with_signer(ctx.accounts.token_program.to_account_info(), cpi_accounts, signer_seeds),
        to_alice,
    )?;

    ctx.accounts.swap.is_redeemed = true;
    msg!("USDC redeemed by Alice");
    Ok(())
}

    /*----------------------------------------------------------
     * 3.  Refund after expiry
     *---------------------------------------------------------*/
pub fn refund(ctx: Context<Refund>, _swap_id: [u8; 32]) -> Result<()> {
    let swap = &mut ctx.accounts.swap;
    require!(!swap.is_redeemed && !swap.is_refunded, ErrorCode::AlreadyFinalized);
    require!(Clock::get()?.unix_timestamp > swap.expiry, ErrorCode::NotYetExpired);

    let bump           = swap.bump;
    let swap_id        = swap.swap_id;
    let vault_balance  = ctx.accounts.vault_usdc.amount;

    drop(swap);

    let seeds = &[b"swap", swap_id.as_ref(), &[bump]];
    let signer_seeds = &[&seeds[..]];

    let cpi_accounts = SplTransfer {
        from: ctx.accounts.vault_usdc.to_account_info(),
        to:   ctx.accounts.funder_token.to_account_info(),
        authority: ctx.accounts.swap.to_account_info(),
    };
    token::transfer(
        CpiContext::new_with_signer(ctx.accounts.token_program.to_account_info(), cpi_accounts, signer_seeds),
        vault_balance,
    )?;

    ctx.accounts.swap.is_refunded = true;
    msg!("Swap refunded");
    Ok(())
}
}

/*==============================================================
 * Data
 *============================================================*/
#[account]
pub struct Swap {
    pub direction: Direction,
    pub swap_id: [u8; 32],
    pub alice: Pubkey,
    pub bob: Pubkey,
    pub secret_hash: [u8; 32],
    pub expiry: i64,
    pub relayer_fee: u64,
    pub is_redeemed: bool,
    pub is_refunded: bool,
    pub usdc_amount: u64,
    pub xmr_amount: u64,
    pub monero_sub_address: [u8; 64],
    pub monero_lock_txid: [u8; 32],
    pub alice_solana: Pubkey,
    pub bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    UsdcToXmr,
    XmrToUsdc,
}

impl Swap {
    pub const LEN: usize = 1 + 32 + 32 + 32 + 32 + 8 + 8 + 1 + 1 + 8 + 8 + 64 + 32 + 32 + 1;
}

/*==============================================================
 * Contexts
 *============================================================*/
#[derive(Accounts)]
#[instruction(swap_id:[u8;32], secret_hash:[u8;32], usdc_amount:u64, xmr_amount:u64, monero_sub_address:[u8;64], expiry:i64, relayer_fee:u64)]
pub struct CreateUsdcToXmr<'info> {
    #[account(
        init,
        payer = alice,
        space = 8 + Swap::LEN,
        seeds = [b"swap", swap_id.as_ref()],
        bump
    )]
    pub swap: Account<'info, Swap>,

    #[account(mut)]
    pub alice: Signer<'info>,

    /// CHECK: Bob’s pubkey
    pub bob: AccountInfo<'info>,

    #[account(
        mut,
        associated_token::mint = usdc_mint,
        associated_token::authority = alice,
    )]
    pub alice_usdc: Account<'info, TokenAccount>,

    #[account(
        init,
        payer = alice,
        associated_token::mint = usdc_mint,
        associated_token::authority = swap,
    )]
    pub vault_usdc: Account<'info, TokenAccount>,

    pub usdc_mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(swap_id:[u8;32], monero_lock_txid:[u8;32])]
pub struct RecordProof<'info> {
    #[account(mut, has_one = bob)]
    pub swap: Account<'info, Swap>,
    pub bob: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(swap_id:[u8;32], adaptor_sig:Vec<u8>)]
pub struct RedeemUsdc<'info> {
    #[account(mut, seeds=[b"swap", swap.swap_id.as_ref()], bump=swap.bump)]
    pub swap: Account<'info, Swap>,
    #[account(mut)]
    pub bob: Signer<'info>,

    #[account(
        mut,
        associated_token::mint = usdc_mint,
        associated_token::authority = swap,
    )]
    pub vault_usdc: Account<'info, TokenAccount>,

    #[account(
        init_if_needed,
        payer = bob,
        associated_token::mint = usdc_mint,
        associated_token::authority = bob,
    )]
    pub bob_token: Account<'info, TokenAccount>,

    #[account(
        init_if_needed,
        payer = bob,
        associated_token::mint = usdc_mint,
        associated_token::authority = relayer,
    )]
    pub relayer_token: Account<'info, TokenAccount>,

    /// CHECK: relayer
    pub relayer: AccountInfo<'info>,

    pub usdc_mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(swap_id:[u8;32], secret_hash:[u8;32], usdc_amount:u64, xmr_amount:u64, alice_solana:Pubkey, expiry:i64, relayer_fee:u64)]
pub struct CreateXmrToUsdc<'info> {
    #[account(
        init,
        payer = bob,
        space = 8 + Swap::LEN,
        seeds = [b"swap", swap_id.as_ref()],
        bump
    )]
    pub swap: Account<'info, Swap>,

    /// CHECK: Alice pubkey
    pub alice: AccountInfo<'info>,

    #[account(mut)]
    pub bob: Signer<'info>,

    #[account(
        mut,
        associated_token::mint = usdc_mint,
        associated_token::authority = bob,
    )]
    pub bob_usdc: Account<'info, TokenAccount>,

    #[account(
        init,
        payer = bob,
        associated_token::mint = usdc_mint,
        associated_token::authority = swap,
    )]
    pub vault_usdc: Account<'info, TokenAccount>,

    pub usdc_mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(swap_id:[u8;32], adaptor_sig:Vec<u8>)]
pub struct RedeemUsdcAlice<'info> {
    #[account(mut, seeds=[b"swap", swap.swap_id.as_ref()], bump=swap.bump)]
    pub swap: Account<'info, Swap>,
    #[account(mut)]
    pub alice: Signer<'info>,

    #[account(
        mut,
        associated_token::mint = usdc_mint,
        associated_token::authority = swap,
    )]
    pub vault_usdc: Account<'info, TokenAccount>,

    #[account(
        init_if_needed,
        payer = alice,
        associated_token::mint = usdc_mint,
        associated_token::authority = alice,
    )]
    pub alice_token: Account<'info, TokenAccount>,

    #[account(
        init_if_needed,
        payer = alice,
        associated_token::mint = usdc_mint,
        associated_token::authority = relayer,
    )]
    pub relayer_token: Account<'info, TokenAccount>,

    /// CHECK: relayer
    pub relayer: AccountInfo<'info>,

    pub usdc_mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(swap_id:[u8;32])]
pub struct Refund<'info> {
    #[account(mut, seeds=[b"swap", swap.swap_id.as_ref()], bump=swap.bump)]
    pub swap: Account<'info, Swap>,

    #[account(mut)]
    pub funder: Signer<'info>,

    #[account(
        mut,
        associated_token::mint = usdc_mint,
        associated_token::authority = swap,
    )]
    pub vault_usdc: Account<'info, TokenAccount>,

    #[account(
        init_if_needed,
        payer = funder,
        associated_token::mint = usdc_mint,
        associated_token::authority = funder,
    )]
    pub funder_token: Account<'info, TokenAccount>,

    pub usdc_mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

/*==============================================================
 * Errors
 *============================================================*/
#[error_code]
pub enum ErrorCode {
    #[msg("Invalid adaptor signature")]
    InvalidAdaptorSig,
    #[msg("Already finalized")]
    AlreadyFinalized,
    #[msg("Not yet expired")]
    NotYetExpired,
    #[msg("Wrong direction")]
    WrongDirection,
    #[msg("Invalid preimage")]
    InvalidPreimage,
}
