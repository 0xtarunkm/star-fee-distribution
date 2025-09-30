use anchor_lang::prelude::*;

declare_id!("FAAk54pcwJFvHD76YaB5sZzqXCEhUCVpP3cBvggKofuS");

pub mod instructions;
pub mod errors;

pub use instructions::*;

#[program]
pub mod star_fee_distribution {
    use super::*;

    pub fn initialize_honorary_position(
        ctx: Context<InitializeHonoraryPosition>, 
        config: HonoraryPositionConfig
    ) -> Result<()> {
        InitializeHonoraryPosition::handle(ctx, config)
    }
}

