use anchor_lang::prelude::*;

declare_id!("FAAk54pcwJFvHD76YaB5sZzqXCEhUCVpP3cBvggKofuS");

pub mod instructions;
pub mod errors;
pub mod constants;
pub mod states;
pub mod events;

pub use instructions::*;
pub use events::*;

#[program]
pub mod star_fee_distribution {
    use super::*;
    
    pub fn deposit(ctx: Context<Deposit>, params: DepositParams) -> Result<()> {
        Deposit::handle(ctx, params)
    }
    pub fn initialize_honorary_position(
        ctx: Context<InitializeHonoraryPosition>, 
        config: HonoraryPositionConfig
    ) -> Result<()> {
        InitializeHonoraryPosition::handle(ctx, config)
    }
    pub fn claim_fees_to_pda(ctx: Context<ClaimFeesToPDA>) -> Result<()> {
        ClaimFeesToPDA::handle(ctx)
    }

    pub fn distribute_fees(ctx: Context<DistributeFees>, params: FeeDistributionParams) -> Result<()> {
        DistributeFees::handle(ctx, params)
    }


    pub fn withdraw(ctx: Context<Withdraw>, params: WithdrawParams) -> Result<()> {
        Withdraw::handle(ctx, params)
    }

    pub fn crank_fee_distribution(ctx: Context<CrankFeeDistribution>, params: DistributionParams) -> Result<()> {
        CrankFeeDistribution::handle(ctx, params)
    }

    pub fn distribute_to_investor(ctx: Context<DistributeToInvestor>, params: InvestorDistributionParams) -> Result<()> {
        DistributeToInvestor::handle(ctx, params)
    }

    pub fn route_creator_remainder(ctx: Context<RouteCreatorRemainder>) -> Result<()> {
        RouteCreatorRemainder::handle(ctx)
    }

    pub fn initialize_distribution_config(
        ctx: Context<InitializeDistributionConfig>,
        params: InitializeDistributionConfigParams
    ) -> Result<()> {
        InitializeDistributionConfig::handle(ctx, params)
    }

}

