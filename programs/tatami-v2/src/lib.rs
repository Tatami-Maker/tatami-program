use anchor_lang::{prelude::*, system_program::{Transfer, transfer}};
use solana_program::{pubkey, pubkey::Pubkey};
use anchor_lang::solana_program::{program::invoke, instruction::Instruction};
use anchor_spl::{token::{Mint, Token, SetAuthority, 
    set_authority, spl_token::instruction::AuthorityType,
    mint_to, MintTo, TokenAccount,
    TransferChecked, transfer_checked
}, associated_token::AssociatedToken};
use anchor_spl::metadata::{
    Metadata, CreateMetadataAccountsV3, create_metadata_accounts_v3
};

declare_id!("HrKLeJB6yoSWkFzVSfsg8Yi3Zs4PKZ7qqjkMz978qqZv");

#[constant]
pub const REALMS_ID: Pubkey = pubkey!("GovER5Lthms3bLBqWub97yVrMmEogzX7xNjdXpPPCVZw");

#[program]
pub mod tatami_v2 {
    use anchor_spl::metadata::mpl_token_metadata::types::{DataV2, Creator};

    use super::*;

    pub fn create_config(ctx: Context<CreateConfig>, fee: u64) -> Result<()> {
        let config = &mut ctx.accounts.config;

        config.creator = ctx.accounts.signer.key();
        config.fee = fee;
        config.bump = ctx.bumps.config;
        Ok(())
    }

    pub fn init_project(
        ctx: Context<InitProject>,
        _decimals: u8,
        name: String, 
        symbol: String, 
        uri: String,
        recipients: u16,
        supply: [u64; 2]
    ) -> Result<()> {
         // create metadata
         let creator = Creator {
            address: ctx.accounts.signer.key(),
            verified: true,
            share: 100
        };

        let data = DataV2 {
            name,
            symbol,
            uri,
            seller_fee_basis_points: 0,
            creators: Some(vec![creator]),
            collection: None,
            uses: None
        };

        create_metadata_accounts_v3(
            ctx.accounts.create_metadata_ctx(), 
            data, 
            true, 
            true, 
            None
        )?;

        // Mint Supply
        let team_wallet_exists = ctx.accounts.team_token_account.is_some();
        let team_token_account_exists = ctx.accounts.team_token_account.is_some();

        if team_wallet_exists && team_token_account_exists {
            mint_to(ctx.accounts.mint_supply_ctx(true), supply[0])?; // Team
        }

        mint_to(ctx.accounts.mint_supply_ctx(false), supply[1])?; // Rest
        
        // Transfer Fee to Vault
        let fee = ctx.accounts.config.fee;
        transfer(ctx.accounts.transfer_sol_ctx(), fee)?;
        
        let project = &mut ctx.accounts.project;

        project.creator = ctx.accounts.signer.key();
        project.mint = ctx.accounts.mint.key();
        project.bump = ctx.bumps.project;
        project.mint_exist = true;
        project.recipients = recipients;
        project.recipients_paid = 0;
        Ok(())
    }

    pub fn initialize_dao(
        ctx:Context<InitializeDao>, 
        name: String,
        supply: u64,
        min_vote_to_govern: u64,
        is_council: bool,
        quorum: u8,
        vote_duration: u32
    ) -> Result<()> {
        ctx.accounts.create_realm(name, min_vote_to_govern, is_council)?;
        ctx.accounts.create_governance(vote_duration, quorum, min_vote_to_govern)?;
        ctx.accounts.create_native_treasury()?;
        ctx.accounts.set_realm_authority()?;
        mint_to(ctx.accounts.mint_dao_allocation(), supply)?;

        let project = &mut ctx.accounts.project;
        project.dao_init = true;
        Ok(())
    }

    pub fn initialize_lp(ctx:Context<InitializeLp>) -> Result<()> {
        // create_market

        Ok(())
    }

    pub fn airdrop_tokens(ctx: Context<AirdropTokens>, amount: u64) -> Result<()> {
        let decimals = ctx.accounts.mint.decimals;
        let recipients = ctx.accounts.project.recipients;
        let recipients_paid = ctx.accounts.project.recipients_paid;

        require_gt!(recipients, recipients_paid, Errors::MaxRecipientsPaid);

        let seeds: &[u8] = b"tatami-vault";

        let (_, bump) = Pubkey::find_program_address(&[seeds], &id());
        
        transfer_checked(
            ctx.accounts.transfer_tokens_ctx().with_signer(&[&[seeds, &[bump]]]), 
            amount, 
            decimals
        )?;

        let project = &mut ctx.accounts.project;
        project.recipients_paid += 1;
        Ok(())
    }

    pub fn burn_authority(ctx: Context<BurnAuthority>) -> Result<()> {
        set_authority(ctx.accounts.set_auth_ctx(), AuthorityType::MintTokens, None)?;
        let project = &mut ctx.accounts.project;
        project.mint_exist = false;
        Ok(())
    }
}


#[derive(Accounts)]
pub struct CreateConfig<'info> {
    #[account(
        init, 
        payer = signer, 
        space = 8 + 8 + 32 + 1,
        seeds = [b"tatami-config"],
        bump
    )]
    pub config: Account<'info, Config>,
    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
#[instruction(decimals: u8)]
pub struct InitProject<'info> {
    #[account(
        init, 
        payer = signer, 
        space = 8 + 2 * 32 + 1 * 4 + 2 * 2,
        seeds = [
            b"tatami-project",
            mint.key().as_ref()
        ],
        bump
    )]
    pub project: Account<'info, Project>,
    #[account(
        seeds = [b"tatami-config"],
        bump = config.bump
    )]
    pub config: Account<'info, Config>,
    #[account(
        mut,
        seeds = [
            b"tatami-vault"
        ],
        bump
    )]
    pub vault: SystemAccount<'info>,
    #[account(
        init,
        payer = signer,
        mint::decimals = decimals,
        mint::authority = signer
    )]
    pub mint: Account<'info, Mint>,
    #[account(
        init,
        payer = signer,
        associated_token::mint = mint,
        associated_token::authority = vault,
    )]
    pub vault_token_account: Account<'info, TokenAccount>,
    #[account(
        init,
        payer = signer,
        associated_token::mint = mint,
        associated_token::authority = team_wallet
    )]
    pub team_token_account: Option<Account<'info, TokenAccount>>,
    pub team_wallet: Option<SystemAccount<'info>>,
    /// CHECK: This account is initialized in the ix
    #[account(mut)]
    pub metadata: UncheckedAccount<'info>,
    #[account(mut)]
    pub signer: Signer<'info>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub metadata_program: Program<'info, Metadata>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>
}

#[derive(Accounts)]
pub struct InitializeDao<'info> {
    #[account(mut, address = project.creator)]
    pub signer: Signer<'info>,
    #[account(
        mut,
        seeds = [
            b"tatami-project",
            mint.key().as_ref()
        ],
        bump = project.bump,
        has_one = mint
    )]
    pub project: Account<'info, Project>,
    pub mint: Account<'info, Mint>,
    #[account(
        init,
        payer = signer,
        mint::decimals = 1,
        mint::authority = signer
    )]
    pub council_mint: Option<Account<'info, Mint>>,
    /// CHECK: CPI Account
    #[account(mut)]
    pub realm_account: UncheckedAccount<'info>,
    /// CHECK: CPI Account
    #[account(mut)]
    pub community_token_holding: UncheckedAccount<'info>,
    /// CHECK: CPI Account
    #[account(mut)]
    pub council_token_holding: Option<UncheckedAccount<'info>>,
    /// CHECK: CPI Account
    #[account(mut)]
    pub realm_config: UncheckedAccount<'info>,
    /// CHECK: CPI Account
    #[account(mut)]
    pub governance: UncheckedAccount<'info>,
    /// CHECK: CPI Account (for seeding)
    pub governed_account: UncheckedAccount<'info>,
    /// CHECK: CPI Account
    #[account(mut)]
    pub native_treasury: UncheckedAccount<'info>,
    #[account(
        init,
        payer = signer,
        associated_token::mint = mint,
        associated_token::authority = native_treasury
    )]
    pub dao_token_account: Account<'info, TokenAccount>,
    /// CHECK: CPI Account
    #[account(address = REALMS_ID)]
    pub realm_program: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub rent: Sysvar<'info, Rent>
}

#[derive(Accounts)]
pub struct AirdropTokens<'info> {
    #[account(mut, address = project.creator)]
    pub signer: Signer<'info>,
    #[account(
        mut,
        seeds = [
            b"tatami-project",
            mint.key().as_ref()
        ],
        bump = project.bump,
        has_one = mint
    )]
    pub project: Account<'info, Project>,
    #[account(
        seeds = [
            b"tatami-vault"
        ],
        bump
    )]
    pub vault: SystemAccount<'info>,
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = vault,
    )]
    pub vault_token_account: Account<'info, TokenAccount>,
    #[account(
        init_if_needed,
        payer = signer,
        associated_token::mint = mint,
        associated_token::authority = receiver,
    )]
    pub recipient_token_account: Account<'info, TokenAccount>,
    /// CHECK: This account is not read from or write to
    pub receiver: UncheckedAccount<'info>,
    pub mint: Account<'info, Mint>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
pub struct BurnAuthority<'info> {
    #[account(mut, address = project.creator)]
    pub signer: Signer<'info>,
    #[account(
        mut,
        seeds = [
            b"tatami-project",
            mint.key().as_ref()
        ],
        bump = project.bump,
        has_one = mint
    )]
    pub project: Account<'info, Project>,
    pub mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct InitializeLp {}

#[account] 
pub struct Project {
    creator: Pubkey,
    mint: Pubkey,
    dao_init: bool,
    lp_init: bool,
    mint_exist: bool,
    bump: u8,
    recipients: u16,
    recipients_paid: u16
}

#[account]
pub struct Config {
    fee: u64,
    creator: Pubkey,
    bump: u8
}

impl<'info> InitProject<'info> {
    pub fn create_metadata_ctx(&self) -> CpiContext<'_, '_, '_, 'info, CreateMetadataAccountsV3<'info>> {
        let cpi_program = self.metadata_program.to_account_info();
        let cpi_accounts = CreateMetadataAccountsV3 {
            metadata: self.metadata.to_account_info(),
            mint: self.mint.to_account_info(),
            mint_authority: self.signer.to_account_info(),
            payer: self.signer.to_account_info(),
            update_authority: self.signer.to_account_info(),
            system_program: self.system_program.to_account_info(),
            rent: self.rent.to_account_info()
        };

        CpiContext::new(cpi_program, cpi_accounts)
    }

    pub fn transfer_sol_ctx(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_program = self.system_program.to_account_info();
        let cpi_accounts = Transfer {
            to: self.vault.to_account_info(),
            from: self.signer.to_account_info()
        };

        CpiContext::new(cpi_program, cpi_accounts)
    }

    pub fn mint_supply_ctx(&self, is_team: bool) -> CpiContext<'_, '_, '_, 'info, MintTo<'info>> {
        let cpi_program = self.token_program.to_account_info();
        let cpi_accounts = MintTo {
            mint: self.mint.to_account_info(),
            to: if is_team {
                self.team_token_account.as_ref().unwrap().to_account_info()
            } else {
                self.vault_token_account.to_account_info()
            },
            authority: self.signer.to_account_info()
        };

        CpiContext::new(cpi_program, cpi_accounts)
    }
}

impl<'info> InitializeDao<'info> {
    pub fn create_realm(&self, name: String, min_vote_to_govern: u64, is_council: bool) -> Result<()> {
        let mut create_realm_keys = vec![
            self.realm_account.to_account_info(),
            self.signer.to_account_info(),
            self.mint.to_account_info(),
            self.community_token_holding.to_account_info(),
            self.signer.to_account_info(),
            self.system_program.to_account_info(),
            self.token_program.to_account_info(),
            self.rent.to_account_info()
        ];

        if is_council {
            if let Some(council_mint) = self.council_mint.as_ref() {
                if let Some(council_token_holding) = self.council_token_holding.as_ref() {
                    create_realm_keys.push(council_mint.to_account_info());
                    create_realm_keys.push(council_token_holding.to_account_info());
                } else {
                    return Err(Errors::NoCouncilTokenHolding.into());
                }
            };
        }

        create_realm_keys.push(self.realm_config.to_account_info());

        let create_realm_args = CreateRealmConfig {
            name,
            config: RealmConfigArgs {
                use_council_mint: is_council,
                min_community_weight_to_create_governance: min_vote_to_govern,
                community_mint_max_voter_weight_source: MintMaxVoterWeightSource::SupplyFraction(10000000000),
                community_token_config_args: GoverningTokenConfigArgs {
                    use_max_voter_weight_addin: false,
                    use_voter_weight_addin: false,
                    token_type: GoverningTokenType::Liquid
                },
                council_token_config_args: GoverningTokenConfigArgs {
                    use_max_voter_weight_addin: false,
                    use_voter_weight_addin: false,
                    token_type: GoverningTokenType::Membership
                }
            }
        };

        let mut serialize_args: Vec<u8> = vec![0];
        create_realm_args.serialize(&mut serialize_args)?;

        let create_realm_ix = Instruction {
            program_id: self.realm_program.key(),
            accounts: create_realm_keys.to_account_metas(None),
            data: serialize_args
        };

        invoke(&create_realm_ix, &create_realm_keys)?;

        Ok(())
    }

    pub fn create_governance(&self, vote_duration: u32, quorum: u8, min_vote_to_govern: u64) -> Result<()> {
        require_gte!(100, quorum, Errors::InvalidQuorum);
        require_gte!(quorum, 0, Errors::InvalidQuorum);

        let create_gov_keys = vec![
            self.realm_account.to_account_info(),
            self.governance.to_account_info(),
            self.governed_account.to_account_info(),
            self.system_program.to_account_info(),
            self.signer.to_account_info(),
            self.system_program.to_account_info(),
            self.signer.to_account_info(),
            self.realm_config.to_account_info()
        ];

        let create_gov_args = GovernanceConfig {
            community_vote_threshold: VoteThreshold::YesVotePercentage(quorum),
            min_community_weight_to_create_proposal: min_vote_to_govern,
            min_transaction_hold_up_time: 0,
            voting_base_time: vote_duration,
            community_vote_tipping: VoteTipping::Strict,
            community_veto_vote_threshold: VoteThreshold::Disabled,
            council_veto_vote_threshold: VoteThreshold::YesVotePercentage(60),
            council_vote_threshold: VoteThreshold::YesVotePercentage(60),
            min_council_weight_to_create_proposal: 1,
            council_vote_tipping: VoteTipping::Strict,
            voting_cool_off_time: 43200,
            deposit_exempt_proposal_count: 10
        };

        let mut serialize_args: Vec<u8> = vec![4];
        create_gov_args.serialize(&mut serialize_args)?;

        let create_gov_ix = Instruction {
            program_id: self.realm_program.key(),
            accounts: create_gov_keys.to_account_metas(None),
            data: serialize_args
        };

        invoke(&create_gov_ix, &create_gov_keys)?;

        Ok(())
    }

    pub fn create_native_treasury(&self) -> Result<()> {
        let create_treasury_keys = vec![
            self.governance.to_account_info(),
            self.native_treasury.to_account_info(),
            self.signer.to_account_info(),
            self.system_program.to_account_info()
        ];

        let create_treasury_ix = Instruction {
            program_id: self.realm_program.key(),
            accounts: create_treasury_keys.to_account_metas(None),
            data: vec![25]
        };

        invoke(&create_treasury_ix, &create_treasury_keys)?;
        Ok(())
    }

    pub fn set_realm_authority(&self) -> Result<()> {
        let set_authority_keys = vec![
            self.realm_account.to_account_info(),
            self.signer.to_account_info(),
            self.governance.to_account_info()
        ];

        let set_authority_args = SetRealmAuthorityArgs {
            action: SetRealmAuthorityAction::SetChecked
        };

        let mut serialized_args = vec![21];
        set_authority_args.serialize(&mut serialized_args)?;

        let set_authority_ix = Instruction {
            program_id: self.realm_program.key(),
            accounts: set_authority_keys.to_account_metas(None),
            data: serialized_args
        };

        invoke(&set_authority_ix, &set_authority_keys)?;
        Ok(())
    }

    pub fn mint_dao_allocation(&self) -> CpiContext<'_, '_, '_, 'info, MintTo<'info>> {
        let cpi_program = self.token_program.to_account_info();
        let cpi_accounts = MintTo {
            to: self.dao_token_account.to_account_info(),
            authority: self.signer.to_account_info(),
            mint: self.mint.to_account_info()          
        };

        CpiContext::new(cpi_program, cpi_accounts)
    }
}

impl<'info> AirdropTokens<'info> {
    pub fn transfer_tokens_ctx(&self) -> CpiContext<'_, '_, '_, 'info, TransferChecked<'info>> {
        let cpi_program = self.token_program.to_account_info();
        let cpi_accounts = TransferChecked {
            from: self.vault_token_account.to_account_info(),
            to: self.recipient_token_account.to_account_info(),
            mint: self.mint.to_account_info(),
            authority: self.vault.to_account_info()
        };

        CpiContext::new(cpi_program, cpi_accounts)
    }
}

impl<'info> BurnAuthority<'info> {
    pub fn set_auth_ctx(&self) -> CpiContext<'_, '_, '_, 'info, SetAuthority<'info>> {
        let cpi_program = self.token_program.to_account_info();
        let cpi_accounts = SetAuthority {
            current_authority: self.signer.to_account_info(),
            account_or_mint: self.mint.to_account_info()
        };

        CpiContext::new(cpi_program, cpi_accounts)
    }
}

#[error_code]
pub enum Errors {
    #[msg("council token holding account not provided")]
    NoCouncilTokenHolding,
    #[msg("quorum should be in the range of 1 and 100")]
    InvalidQuorum,
    #[msg("max recipients has been paid out")]
    MaxRecipientsPaid
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct CreateRealmConfig {
    name: String,
    config: RealmConfigArgs
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct RealmConfigArgs {
    use_council_mint: bool,
    min_community_weight_to_create_governance: u64,
    community_mint_max_voter_weight_source: MintMaxVoterWeightSource,
    community_token_config_args: GoverningTokenConfigArgs,
    council_token_config_args: GoverningTokenConfigArgs, 
}

#[derive(AnchorDeserialize, AnchorSerialize)]
pub struct GoverningTokenConfigArgs {
    use_voter_weight_addin: bool,
    use_max_voter_weight_addin: bool,
    token_type: GoverningTokenType,
}

#[derive(AnchorDeserialize, AnchorSerialize)]
pub enum GoverningTokenType {
    Liquid,
    Membership,
    Dormant
}

#[derive(AnchorDeserialize, AnchorSerialize)]
pub enum MintMaxVoterWeightSource {
    SupplyFraction(u64),
    Absolute(u64),
}

#[derive(AnchorDeserialize, AnchorSerialize)]
pub struct GovernanceConfig {
    pub community_vote_threshold: VoteThreshold,
    pub min_community_weight_to_create_proposal: u64,
    pub min_transaction_hold_up_time: u32,
    pub voting_base_time: u32,
    pub community_vote_tipping: VoteTipping,
    pub council_vote_threshold: VoteThreshold,
    pub council_veto_vote_threshold: VoteThreshold,
    pub min_council_weight_to_create_proposal: u64,
    pub council_vote_tipping: VoteTipping,
    pub community_veto_vote_threshold: VoteThreshold,
    pub voting_cool_off_time: u32,
    pub deposit_exempt_proposal_count: u8,
}

#[derive(AnchorDeserialize, AnchorSerialize)]
pub enum VoteThreshold {
    YesVotePercentage(u8),
    QuorumPercentage(u8),
    Disabled
}

#[derive(AnchorDeserialize, AnchorSerialize)]
pub enum VoteTipping {
    Strict,
    Early,
    Disabled
}

#[derive(AnchorDeserialize, AnchorSerialize)]
pub struct  SetRealmAuthorityArgs {
    action: SetRealmAuthorityAction
}

#[derive(AnchorDeserialize, AnchorSerialize)]
pub enum SetRealmAuthorityAction {
    SetUnchecked,
    SetChecked,
    Remove
}