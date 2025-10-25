use anchor_lang::prelude::*;

use base::*;
mod base;
mod utils;
use membership::*;
mod membership;
mod cnft;
declare_id!("7QiBiGFJZqXoLuZ75K9nmsh5UhYHAPutAVR7xfoacMM7");

#[program]
pub mod humanship {

    use super::*;

    pub fn create_universe(ctx: Context<CreateUniverse>,slot:u16) -> Result<()> {
        base::base_ix::create_universe(ctx, slot)
    }

    pub fn create_membership(ctx: Context<CreateMembership>, range:[u32;2], slot:u32, membership_type:MembershipType, membership_data_type:MembershipDataType) -> Result<()> {
        membership::membership_ix::create_membership(ctx, range, slot, membership_type, membership_data_type)
    }

pub fn adjust_membership_size(ctx: Context<AdjustMembershipSize>) -> Result<()> {

        membership::membership_ix::adjust_membership_size(ctx)

    }

    pub fn delete_membership<'a, 'b, 'c, 'info>(ctx: Context<'a, 'b, 'c, 'info, DeleteMembership<'info>>, registry_slot:u32, membership_sync_check:MembershipSyncCheck,  new_hash_bytes:[u8;7], cnft: CnftMembership) -> Result<()> {

        membership::membership_ix::delete_membership(ctx, registry_slot, membership_sync_check, new_hash_bytes, cnft)

    }
    pub fn register_membership(ctx: Context<RegisterMembership>, registry_slot:u32, membership_sync_check:MembershipSyncCheck, timestamp:u32, proof:[u8;64], arweave:String, version:u16, bundler:AssetBundler) -> Result<()> {

        membership::membership_ix::register_membership(ctx, registry_slot, membership_sync_check, timestamp, proof, arweave, version, bundler)

    }

    pub fn create_store(ctx: Context<CreateStore>, slot: u16) -> Result<()> {
        base::base_ix::create_store(ctx, slot)
    }

    pub fn create_master(ctx: Context<CreateMaster>,slot:u16) -> Result<()> {
        base::base_ix::create_master(ctx, slot)
    }

    pub fn feed_global_tree(ctx: Context<FeedGlobalTree>, max_depth:u32, max_buffer_size:u32, public:bool) -> Result<()> {
        base::base_ix::feed_global_tree(ctx, max_depth, max_buffer_size, public)
    }

    pub fn create_collection<'a, 'b, 'c, 'info>(ctx: Context<'a, 'b, 'c, 'info, CreateCollection<'info>>, token_metadata:crate::cnft::TokenMetadata, vault_type:String) -> Result<()> {
        base::base_ix::create_collection(ctx,token_metadata,vault_type)
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, PartialEq, Eq, Debug, Clone)]
#[repr(u8)]
pub enum AccountClass {
  UniverseV1 = 0,//0
  StoreV1 = 1,//1
  MasterV1 = 2,//2
}