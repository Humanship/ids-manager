use anchor_lang::prelude::*;
use crate::AccountClass;
use crate::base::*;
use crate::utils::*;
use crate::cnft::*;


use mpl_bubblegum::utils::get_asset_id;

use anchor_lang::system_program::{create_account, CreateAccount};

use solana_program::instruction::Instruction;

const MEMBERSHIP_GROUP_DISCRIMINATOR:[u8;8] = [106, 195, 37, 126, 222, 139, 217, 237];
const MEMBERSHIP_GROUP_SIZE:usize = 1+1+32+4+1+1+32+8+8;
const TO_MEMBERSHIP_GROUP_COUNT:usize = 8+72;
const TO_MEMBERSHIP_GROUP_ALIVE:usize = 8+80;
const TO_MEMBERSHIP_GROUP_CREATOR:usize = 8+40;
//version,state,store,slot+1+1,creator,totalcreated,alive

const MEMBERSHIP_CONFIG_SIZE:usize = 157;
const MEMBERSHIP_TO_RANGE:usize = 47;
const MEMBERSHIP_TO_SLOT:usize = 9;
const MEMBERSHIP_TO_TYPE:usize = 45;
const MEMBERSHIP_TO_DATA_TYPE:usize = 46;
const MEMBERSHIP_DISCRIMINATOR:[u8;8] = [231, 141, 180, 98, 109, 168, 175, 166];
const HASH_SIZE_IN_SLOT:usize = 7;
const COUNT_SIZE_IN_SLOT:usize = 3;
const MEMBERSHIP_SIZE_PER_SLOT:usize = HASH_SIZE_IN_SLOT+COUNT_SIZE_IN_SLOT;
#[error_code]
pub enum MembershipError {
  #[msg("Something happened.")]
  WrongProofCheck,
  #[msg("Relayer out of sync.")]
  RelayerOutOfSync
}


use solana_program::clock::Clock;



pub mod membership_ix {
    use crate::utils::cyrb53_bytes;

    use super::*;

    pub fn delete_membership<'a, 'b, 'c, 'info>(ctx: Context<'a, 'b, 'c, 'info, DeleteMembership<'info>>, registry_slot:u32, membership_sync_check:MembershipSyncCheck, new_hash_bytes:[u8;7], cnft: CnftMembership) -> Result<()> {
  
       
        let store = &ctx.accounts.store;
        let store_key = store.key();
        
        let creator_key = ctx.accounts.creator.key();
        
        let bubblegum_program_info = ctx.accounts.bubblegum_program.to_account_info();
        let tree_authority_info = ctx.accounts.tree_authority.to_account_info();
        let merkle_tree_info = ctx.accounts.merkle_tree.to_account_info();
        let compression_program_info = ctx.accounts.compression_program.to_account_info();
        let log_wrapper_info = ctx.accounts.log_wrapper.to_account_info();
        let system_program_info = ctx.accounts.system_program.to_account_info();
        
        let nonce = cnft.index as u64;
        let leaf_index = cnft.index;
        
        
        let mut creators = vec![];

        let id_hash_key = ctx.accounts.id_hash.key();
        let link_hash_key = ctx.accounts.link_hash.key();

        let membership_manager_key = ctx.accounts.membership_manager.key();
        let membership_slot = &ctx.accounts.membership_slot;
        let membership_slot_key = membership_slot.key();
        
        creators.push(Creator {
            address:membership_manager_key.clone(),
            share:100,
            verified:false
        });
        
        creators.push(Creator{
            address:id_hash_key.clone(),
            share:0,
            verified:false
        });
        
        creators.push(Creator {
            address:link_hash_key.clone(),
            share:0,
            verified:false
        });

        creators.push(Creator {
                address:ctx.accounts.unique_hash.key(),
                share:0,
                verified:false
        });
        creators.push(Creator {
            address:membership_slot_key,
            share:0,
            verified:true
        });

        msg!("creators {:?}",creators);
        
        let creator_data = creators.iter().map(|c| { Ok([c.address.as_ref(), &[c.verified as u8], &[c.share]].concat()) }).collect::<Result<Vec<_>>>()?;
        let creator_hash = solana_program::keccak::hashv( creator_data.iter().map(|c| c.as_slice()).collect::<Vec<&[u8]>>().as_ref());

        
        let mut slot_bytes:[u8;6] = [0;6];
        {
            let membership = &mut ctx.accounts.membership;
            let info = membership.to_account_info();
            let mut ref_data = info.try_borrow_mut_data()?;

            let slot = u32::from_le_bytes(ref_data[MEMBERSHIP_TO_SLOT..MEMBERSHIP_TO_SLOT+4].try_into().unwrap());
            let sbytes = &slot.to_le_bytes();
            let memebership_type = ref_data[MEMBERSHIP_TO_TYPE];
            let memebership_type_data = ref_data[MEMBERSHIP_TO_DATA_TYPE];
            slot_bytes = [sbytes[0], sbytes[1], sbytes[2], sbytes[3], memebership_type, memebership_type_data];



            let my_membership_slot = 8+MEMBERSHIP_CONFIG_SIZE + registry_slot as usize * MEMBERSHIP_SIZE_PER_SLOT;

            let bytes_counts_at_slot = &ref_data[my_membership_slot+7..my_membership_slot+10];
            let count_at_slot = u32::from_le_bytes([bytes_counts_at_slot[0],bytes_counts_at_slot[1],bytes_counts_at_slot[2],0]);

            let bytes_hash_at_slot = &ref_data[my_membership_slot..my_membership_slot+7];


            if bytes_hash_at_slot != &membership_sync_check.last_slot_hash {
                msg!("wrong hash");
                return Err(MembershipError::RelayerOutOfSync.into()) 
            }

            if count_at_slot != membership_sync_check.last_slot_members_count {
                msg!("wrong count {:?} {:?} {:?}",count_at_slot,membership_sync_check.last_slot_members_count,bytes_counts_at_slot);
                return Err(MembershipError::RelayerOutOfSync.into()) 
            }

            let new_count_bytes = &(count_at_slot-1).to_le_bytes();
            ref_data[my_membership_slot+7..my_membership_slot+10].copy_from_slice(&new_count_bytes[0..3]);
            ref_data[my_membership_slot..my_membership_slot+7].copy_from_slice(&new_hash_bytes[0..7]);


        }

        {
            let membership_group = &mut ctx.accounts.membership_group;
            let info_group = membership_group.to_account_info();
            let mut ref_data = info_group.try_borrow_mut_data()?;

            let membership_creator = Pubkey::new_from_array(ref_data[TO_MEMBERSHIP_GROUP_CREATOR..TO_MEMBERSHIP_GROUP_CREATOR+32].try_into().unwrap());

            let master = &ctx.accounts.master;
            if (membership_creator != creator_key && store.creator != creator_key && master.manager != creator_key) || store.master != master.key() {
                msg!("wrong auth");
                return Err(GeneralError::GeneralError.into())
            }

            let cuantos_van = u64::from_le_bytes(ref_data[TO_MEMBERSHIP_GROUP_ALIVE..TO_MEMBERSHIP_GROUP_ALIVE+8].try_into().unwrap());
            if cuantos_van > 0 {
                ref_data[TO_MEMBERSHIP_GROUP_ALIVE..TO_MEMBERSHIP_GROUP_ALIVE+8].copy_from_slice(&(cuantos_van-1).to_le_bytes());
            }
        }

        let unique_hash_bump = &ctx.bumps.unique_hash.to_le_bytes();    
        let unique_hash_seeds = vec![
            b"unique_document".as_ref(),
            id_hash_key.as_ref(),
            link_hash_key.as_ref(),
            unique_hash_bump
        ];

        let unique_info_delegate = ctx.accounts.unique_hash.to_account_info();
        
        let id_hash_info = ctx.accounts.id_hash.to_account_info();

        msg!("datas {:?} {:?} {:?} {:?}",nonce,leaf_index,creator_hash,cnft.data_hash);

        let result = burn_cnft(
            &tree_authority_info,
            &id_hash_info,
            &unique_info_delegate,
            &merkle_tree_info,
            &log_wrapper_info,
            &compression_program_info,
            &system_program_info,
            &bubblegum_program_info,
            cnft.root,
            cnft.data_hash,
            hash_to_u8_array(creator_hash),
            nonce,
            leaf_index,
            unique_hash_seeds,
            ctx.remaining_accounts
        );
        
        match result {
        Ok(()) => {
        }
        Err(err) => {
            // Handle error case  
            return Err(err)
        }
        } 
        Ok(())
    }

    pub fn register_membership(ctx: Context<RegisterMembership>, registry_slot:u32, membership_sync_check:MembershipSyncCheck, timestamp:u32, proof:[u8;64], arweave:String, version:u16, bundler:AssetBundler) -> Result<()> {

        /*{

            let my_membership_slot = 8+MEMBERSHIP_CONFIG_SIZE + registry_slot as usize * MEMBERSHIP_SIZE_PER_SLOT;
            
            let membership = &mut ctx.accounts.membership;
            let info = membership.to_account_info();
            let mut ref_data = info.try_borrow_mut_data()?;

            ref_data[my_membership_slot..my_membership_slot+7].copy_from_slice(&membership_sync_check.last_slot_hash);

            msg!("sl {:?}",membership_sync_check.last_slot_hash);

        }
        return Ok(());*/
 
        let membership_key = ctx.accounts.membership.key();

        let store_key = ctx.accounts.store.key();

        let creator = &mut ctx.accounts.creator;
        let creator_key = creator.key();
        let creator_info = creator.to_account_info();

        let id_hash_key = ctx.accounts.id_hash.key();
        let link_hash_key = ctx.accounts.link_hash.key();

        let membership_manager_key = ctx.accounts.membership_manager.key();

        let clock = Clock::get().unwrap();
        let clock32 = clock.unix_timestamp.clamp(0, u32::MAX as i64) as u32;
        let dif = if clock32 > timestamp { clock32 - timestamp } else { timestamp - clock32 };
        if dif > 90 {
            msg!("expired");
            return Err(GeneralError::GeneralError.into())
        }

        let mut message:[u8;72] = [0;72];
        message[0..4].copy_from_slice(&timestamp.to_le_bytes());
        message[4..4+32].copy_from_slice(&id_hash_key.to_bytes());
        message[36..36+32].copy_from_slice(&link_hash_key.to_bytes());
        message[68..68+4].copy_from_slice(&registry_slot.to_le_bytes());

       

        let signature_verified = verify_signature(&message, &proof, &membership_manager_key.to_bytes());

        match signature_verified {
                Ok(()) => {
            }
            Err(err) => {
                return Err(GeneralError::GeneralError.into()) 
            }
        }



        let bubblegum_program_info = ctx.accounts.bubblegum_program.to_account_info();
        let tree_authority_info = ctx.accounts.tree_authority.to_account_info();
        let merkle_tree_info = ctx.accounts.merkle_tree.to_account_info();
        let compression_program_info = ctx.accounts.compression_program.to_account_info();
        let log_wrapper_info = ctx.accounts.log_wrapper.to_account_info();
        let merkle_manager_info = ctx.accounts.merkle_manager.to_account_info();
        
        
        let edition_account_info = ctx.accounts.edition_account.to_account_info();
        let collection_mint_info = ctx.accounts.collection_mint.to_account_info();
        let bubblegum_signer_info = ctx.accounts.bubblegum_signer.to_account_info();
        let collection_metadata_info = ctx.accounts.collection_metadata.to_account_info();
        let token_metadata_program_info = ctx.accounts.token_metadata_program.to_account_info();
        let system_program_info = ctx.accounts.system_program.to_account_info();

        let id_hash_info = ctx.accounts.id_hash.to_account_info();

        //let range_bytes = range.try_to_vec().unwrap();
        //let membership_bump = &ctx.bumps.membership.to_le_bytes();
        let merkle_manager_bump = &ctx.bumps.merkle_manager.to_le_bytes();
        
        let membership_slot = &ctx.accounts.membership_slot;
        let membership_slot_info = membership_slot.to_account_info();
        let membership_slot_key = membership_slot.key();
        let membership_slot_bump = &ctx.bumps.membership_slot.to_le_bytes();
        
        let registry_slot_bytes = registry_slot.to_le_bytes();

        

        let membership_slot_signature = &[
            b"membership_slot".as_ref(),
            membership_key.as_ref(),
            &registry_slot_bytes.as_ref(),
            membership_slot_bump
        ];

        /*let membership_signature = &[
            b"membership".as_ref(),
            store_key.as_ref(),
            &range_bytes.as_ref(),
            &[membership_type.clone() as u8, membership_data_type.clone() as u8],
            &slot_bytes.as_ref(),
            membership_bump
        ];*/

        let merkle_manager = &[
            b"tree".as_ref(),
            merkle_manager_bump
        ];

        let signature:Vec<&[&[u8]]> = vec![
            membership_slot_signature,
            merkle_manager
        ];

        let mut creators:Vec<Creator> = Vec::new();

        creators.push(Creator{
            address:membership_manager_key,
            share:100,
            verified:false
        });

        creators.push(Creator{
            address:id_hash_key,
            share:0,
            verified:false
        });

        creators.push(Creator{
            address:link_hash_key,
            share:0,
            verified:false
        });

        creators.push(Creator{
            address:ctx.accounts.unique_hash.key(),
            share:0,
            verified:false
        });
        
        creators.push(Creator{
            address:membership_slot_key,
            share:0,
            verified:true
        });


        let mut uri = match bundler {
            AssetBundler::Arweave => {
                "https://arweave.net/".to_string()
            }
            AssetBundler::IrysGateway => {
                "https://gateway.irys.xyz/".to_string()
            }
            AssetBundler::RawTurboDev => {
                "https://turbo.ardrive.dev/raw/".to_string()
            }
        };
        uri += arweave.as_str();

        let mut payload:Vec<u8> = vec![];
        
        payload.extend(version.to_le_bytes());

        let store = &ctx.accounts.store;

        {
            let membership_group = &mut ctx.accounts.membership_group;
            let info_group = membership_group.to_account_info();
            let mut ref_data = info_group.try_borrow_mut_data()?;

            let membership_creator = Pubkey::new_from_array(ref_data[TO_MEMBERSHIP_GROUP_CREATOR..TO_MEMBERSHIP_GROUP_CREATOR+32].try_into().unwrap());

            let master = &ctx.accounts.master;
            if (membership_creator != creator_key && store.creator != creator_key && master.manager != creator_key) || store.master != master.key() {
                msg!("wrong auth");
                return Err(GeneralError::GeneralError.into())
            }

            let cuantos_van = u64::from_le_bytes(ref_data[TO_MEMBERSHIP_GROUP_COUNT..TO_MEMBERSHIP_GROUP_COUNT+8].try_into().unwrap());
            payload.extend(cuantos_van.to_le_bytes());
            ref_data[TO_MEMBERSHIP_GROUP_COUNT..TO_MEMBERSHIP_GROUP_COUNT+8].copy_from_slice(&(cuantos_van+1).to_le_bytes());

            let cuantos_alive = u64::from_le_bytes(ref_data[TO_MEMBERSHIP_GROUP_ALIVE..TO_MEMBERSHIP_GROUP_ALIVE+8].try_into().unwrap());
            ref_data[TO_MEMBERSHIP_GROUP_ALIVE..TO_MEMBERSHIP_GROUP_ALIVE+8].copy_from_slice(&(cuantos_alive+1).to_le_bytes());

        }

       
        
        
        payload.extend(timestamp.to_le_bytes());
        //payload.push(hidden_birthdate);
        //payload.push(hidden_nation);
        

        //payload.push(is_adult);
        /*if let Some(birthdate) = birthdate {
            payload.push(1);
            payload.extend(birthdate.to_le_bytes());
        }
        if let Some(nation) = nation {
            payload.push(2);
            payload.extend(nation);
        }*/

        if payload.len() > 0 {
            uri += "?p=";
            uri += encode_bytes_to_ascii_string(&payload, true).as_str();
        }

        let metadata = MetadataArgs {
            name:"Humanship V0".to_string(),
            symbol:"ID".to_string(),
            uri,
            seller_fee_basis_points: 0,
            creators,
            primary_sale_happened: false,
            is_mutable: true,
            edition_nonce: None,
            collection: Some(Collection { verified:true,key:ctx.accounts.collection_mint.key() }),
            uses: None,
            token_standard: Some(TokenStandard::NonFungible),
            token_program_version: TokenProgramVersion::Original,
        };

        let unique_info_delegate = ctx.accounts.unique_hash.to_account_info();

        let result = mint_to_collection_cnft(
            &bubblegum_program_info,
            &tree_authority_info,
            &id_hash_info,
            &unique_info_delegate,
            &merkle_tree_info,
            &creator_info,
            &merkle_manager_info,
            &merkle_manager_info,
            &bubblegum_program_info,
            &collection_mint_info,
            &collection_metadata_info,
            &edition_account_info,
            &bubblegum_signer_info,
            &log_wrapper_info,
            &compression_program_info,
            &token_metadata_program_info,
            &system_program_info,
            metadata,
            &signature,
            &[membership_slot_info]);
        
        match result {
            Ok(()) => {},
            Err(err) => return Err(err),
        }

       

        {
            
            let membership = &mut ctx.accounts.membership;
            let info = membership.to_account_info();
            let mut ref_data = info.try_borrow_mut_data()?;

            if ref_data[8] != 2 {
                msg!("mal {:?}",ref_data[8]);
                return Err(GeneralError::GeneralError.into())
            }
            
            let mut membership_config = MembershipConfig::try_from_slice(&ref_data[8..START_FROM_MEMBERSHIP]).unwrap();
            membership_config.total_members += 1;
            ref_data[8..START_FROM_MEMBERSHIP].copy_from_slice(&membership_config.try_to_vec()?);

            
            let my_membership_slot = 8+MEMBERSHIP_CONFIG_SIZE + registry_slot as usize * MEMBERSHIP_SIZE_PER_SLOT;

            let bytes_counts_at_slot = &ref_data[my_membership_slot+7..my_membership_slot+10];
            let count_at_slot = u32::from_le_bytes([bytes_counts_at_slot[0],bytes_counts_at_slot[1],bytes_counts_at_slot[2],0]);

            let bytes_hash_at_slot = &ref_data[my_membership_slot..my_membership_slot+7];
            

            let tree_data = tree_authority_info.try_borrow_data()?;
            let num_minted = u64::from_le_bytes(tree_data[80..88].try_into().unwrap());

            let asset_id = get_asset_id(&ctx.accounts.merkle_tree.key(), num_minted - 1);

            /*let asset_id_0 = get_asset_id(&ctx.accounts.merkle_tree.key(), num_minted);
            let asset_id_1 = get_asset_id(&ctx.accounts.merkle_tree.key(), num_minted+1);*/
            
            let asset_bytes = asset_id.to_bytes();
            msg!("ass-1 {:?}",asset_id);
           // msg!("ass0 {:?}",asset_id_0);
            //msg!("ass1 {:?}",asset_id_1);

            msg!("da {:?}",num_minted);

            if bytes_hash_at_slot != &membership_sync_check.last_slot_hash {
                msg!("wrong hash");
                return Err(MembershipError::RelayerOutOfSync.into()) 
            }

            if count_at_slot != membership_sync_check.last_slot_members_count {
                msg!("wrong count {:?} {:?} {:?}",count_at_slot,membership_sync_check.last_slot_members_count,bytes_counts_at_slot);
                return Err(MembershipError::RelayerOutOfSync.into()) 
            }

           

            let mut prev_plus_new_hash = vec![];
            prev_plus_new_hash.extend_from_slice(bytes_hash_at_slot);
            prev_plus_new_hash.extend_from_slice(&asset_bytes);

            let next_hash = cyrb53_bytes(&prev_plus_new_hash, 0);
            let next_hash_full_bytes = &next_hash.to_le_bytes();
//msg!("next {:?} {:?}", prev_plus_new_hash, next_hash);

            if (count_at_slot+1) > 16_777_215 { //max number with 3 bytes
                msg!("mal2 {:?}",count_at_slot);
                return Err(GeneralError::GeneralError.into()) 
            }

            
            let new_count_bytes = &(count_at_slot+1).to_le_bytes();
            ref_data[my_membership_slot+7..my_membership_slot+10].copy_from_slice(&new_count_bytes[0..3]);
            ref_data[my_membership_slot..my_membership_slot+7].copy_from_slice(&next_hash_full_bytes[0..7]);

        }

        Ok(())
    }

    
    pub fn adjust_membership_size(ctx: Context<AdjustMembershipSize>) -> Result<()> {

        let creator = &mut ctx.accounts.creator;
        let creator_key = creator.key();

        let master = &ctx.accounts.master;
        let master_key = master.key();

        let store = &ctx.accounts.store;
        let store_key = store.key();
        
        let mut necesito:usize = 0;
        let membership = &mut ctx.accounts.membership;
            let info = membership.to_account_info();
        
        let mut data_len:usize = 0;
        {
            data_len = info.data_len();
        }
        {
            
            let mut ref_data = info.try_borrow_mut_data()?;
            if ref_data[8] != 1 {
                return Ok(())
            }
          
            let mut membership_config = MembershipConfig::try_from_slice(&ref_data[8..START_FROM_MEMBERSHIP]).unwrap();

            if membership_config.store != store_key {
                return Err(GeneralError::GeneralError.into())
            }
            
            let slots = match membership_config.membership_type {
                MembershipType::Birthday => {
                    membership_config.range[1] - membership_config.range[0]
                }
            };
            

            let bytes_in_membership = 8+MEMBERSHIP_CONFIG_SIZE + slots as usize * MEMBERSHIP_SIZE_PER_SLOT;

            if data_len > 0 {
                if data_len < bytes_in_membership {
                    let dif = bytes_in_membership - data_len;
                    necesito = data_len + if dif > 10240 { 10240 } else { dif };
                    if dif <= 10240 {
                        membership_config.state = 2;
                        membership_config.ready = 10;
                        ref_data[8..START_FROM_MEMBERSHIP].copy_from_slice(&membership_config.try_to_vec()?);
                    }

                }
            } else {
                return Err(GeneralError::GeneralError.into())
            }
        }
        if master.manager != creator_key && store.creator != creator_key {
            msg!("wrong master or creator");
            return Err(GeneralError::GeneralError.into())
        }
        {

            

            let result = info.realloc(necesito, true);
            match result {
                    Ok(()) => {
                    }
                    Err(err) => {
                        return Err(GeneralError::GeneralError.into()) 
                    }
            }

        }

        Ok(())
    }

    pub fn create_membership(ctx: Context<CreateMembership>, range:[u32;2], slot:u32, membership_type:MembershipType, membership_data_type:MembershipDataType) -> Result<()> {

        
        let creator = &mut ctx.accounts.creator;
        let creator_key = creator.key();
        let creator_info = creator.to_account_info();

        let slots = match membership_type {
            MembershipType::Birthday => {
                range[1] - range[0]
            }
        };

        let bytes_in_membership = 8+MEMBERSHIP_CONFIG_SIZE + slots as usize * MEMBERSHIP_SIZE_PER_SLOT;
            
        let rent = Rent::get()?;
        let min_base = rent.minimum_balance(bytes_in_membership);

        let store = &ctx.accounts.store;
        let store_key = store.key();
        //let store_hash = cyrb53_bytes(&store_key.to_bytes(), 0);

        let universe_hash = cyrb53_bytes(&store.universe.to_bytes(), 0);

        let master = &ctx.accounts.master;
        let master_key = master.key();
        
        if master.universe_hash != universe_hash {
            return Err(GeneralError::GeneralError.into())
        }
        
        if master.manager != creator_key && store.creator != creator_key {
            msg!("wrong master or creator");
            return Err(GeneralError::GeneralError.into())
        }
        let slot_bytes = slot.to_le_bytes();
        let system_program = &ctx.accounts.system_program;
        let system_program_info = system_program.to_account_info();
        let mut new_group = false;
        {

            let membership_group = &mut ctx.accounts.membership_group;
            let membership_group_bump = &ctx.bumps.membership_group.to_le_bytes();
            let membership_group_info = membership_group.to_account_info();

            let data_len = membership_group_info.data_len();

            let bytes_in_group = 8+MEMBERSHIP_GROUP_SIZE;
            let min_group_base = rent.minimum_balance(bytes_in_group);

            if data_len == 0 {
                new_group = true;
                

                let created = create_account(
                    CpiContext::new_with_signer(
                    system_program_info.clone(),
                    CreateAccount {
                        from: creator_info.clone(),
                        to: membership_group_info.clone()
                    },
                    &[&[
                        b"membership_group".as_ref(),
                        store_key.as_ref(),
                        &[slot_bytes[0], slot_bytes[1], slot_bytes[2], slot_bytes[3], membership_type.clone() as u8, membership_data_type.clone() as u8],
                        membership_group_bump
                    ]]
                    ),
                    min_group_base,
                    bytes_in_group as u64,
                    &crate::ID
                );

                match created {
                    Ok(()) => {
                    }
                    Err(err) => {
                        return Err(err)
                    }
                }

            }

        }

        {
            if new_group {
                let membership_group = &mut ctx.accounts.membership_group;
                let info_group = membership_group.to_account_info();
                let mut ref_data = info_group.try_borrow_mut_data()?;
                //version,state,store,slot,creator,count,extra
                ref_data[0..8].copy_from_slice(&MEMBERSHIP_GROUP_DISCRIMINATOR);
                ref_data[8] = 0;
                ref_data[9] = 1;
                ref_data[10..10+32].copy_from_slice(&store_key.to_bytes());
                ref_data[42..42+4].copy_from_slice(&slot_bytes);
                ref_data[46] = membership_type.clone() as u8;
                ref_data[47] = membership_data_type.clone() as u8;
                ref_data[48..48+32].copy_from_slice(&creator_key.to_bytes());
            }
            
        }

        {

            let membership = &mut ctx.accounts.membership;
            let membership_bump = &ctx.bumps.membership.to_le_bytes();
            

            let membership_info = membership.to_account_info();
            

            let data_len = membership_info.data_len();

            if data_len > 0 {
                return Ok(());
            }
            //seeds = [b"membership".as_ref(), store.key().as_ref(), &range.try_to_vec().unwrap().as_ref(), &[membership_type as u8], &slot.to_le_bytes().as_ref()],

            let range_bytes = range.try_to_vec().unwrap();
            

            let created = create_account(
                CpiContext::new_with_signer(
                system_program_info.clone(),
                CreateAccount {
                    from: creator_info.clone(),
                    to: membership_info.clone()
                },
                &[&[
                    b"membership".as_ref(),
                    store_key.as_ref(),
                    &range_bytes.as_ref(),
                    &[slot_bytes[0], slot_bytes[1], slot_bytes[2], slot_bytes[3], membership_type.clone() as u8, membership_data_type.clone() as u8],
                    membership_bump
                ]]
                ),
                min_base,
                if bytes_in_membership > 10240 { 10240 } else { bytes_in_membership } as u64,
                &crate::ID
            );

            match created {
                Ok(()) => {
                }
                Err(err) => {
                    return Err(err)
                }
            }

        }

        

        let membership_config = MembershipConfig
        {
            state:1,
            ready: if bytes_in_membership <= 10240 { 10 } else { 1 },
            slot,
            store:store_key,
            membership_type,
            membership_data_type,
            range,
            creator:creator_key,
            version:0,
            slots,
            master:master_key,
            total_members:0,
            extra:[0;32]
        };
       
        {
            let membership = &mut ctx.accounts.membership;
            let info = membership.to_account_info();
            let mut ref_data = info.try_borrow_mut_data()?;

            ref_data[0..8].copy_from_slice(&MEMBERSHIP_DISCRIMINATOR);
            ref_data[8..START_FROM_MEMBERSHIP].copy_from_slice(&membership_config.try_to_vec()?);
            
        }

        Ok(())
    }
}


/*#[derive(Accounts)]
pub struct VerifyMembership<'info> {
    pub system_program: Program<'info, System>,
}*/

#[derive(Accounts)]
#[instruction(range:[u32;2], slot:u32, membership_type:MembershipType, membership_data_type:MembershipDataType)]
pub struct CreateMembership<'info> {
  #[account(
        mut,
        seeds = [b"membership".as_ref(), store.key().as_ref(), &range.try_to_vec().unwrap().as_ref(), {
            let slot_bytes = slot.to_le_bytes();
            &[slot_bytes[0], slot_bytes[1], slot_bytes[2], slot_bytes[3], membership_type.clone() as u8, membership_data_type.clone() as u8]
        }],
        bump
    )]
    /// CHECK
    pub membership: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds = [b"membership_group".as_ref(), store.key().as_ref(), {
            let slot_bytes = slot.to_le_bytes();
            &[slot_bytes[0], slot_bytes[1], slot_bytes[2], slot_bytes[3], membership_type as u8, membership_data_type as u8]
        }],
        bump
    )]
    /// CHECK
    pub membership_group: UncheckedAccount<'info>,
    #[account(mut)]
    pub master: Box<Account<'info, Master>>,
    #[account(mut)]
    pub store: Box<Account<'info, Store>>,
    #[account(mut)]
    pub creator: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct AdjustMembershipSize<'info> {
  #[account( mut )]
    /// CHECK
    pub membership: UncheckedAccount<'info>,
    #[account(mut)]
    pub master: Box<Account<'info, Master>>,
    #[account(mut)]
    pub store: Box<Account<'info, Store>>,
    #[account(mut)]
    pub creator: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(registry_slot:u32, membership_sync_check:MembershipSyncCheck, new_hash_bytes:[u8;7], cnft:CnftMembership)]
pub struct DeleteMembership<'info> {
  #[account(
        mut,
        seeds = [b"membership".as_ref(), store.key().as_ref(), {
                
                let mut range:[u32;2] = [0,0];

                let info = membership.to_account_info();
                let ref_data = info.try_borrow_data()?;

                let r0 = u32::from_le_bytes(ref_data[MEMBERSHIP_TO_RANGE..MEMBERSHIP_TO_RANGE+4].try_into().unwrap());
                range[0] = r0;

                let r1 = u32::from_le_bytes(ref_data[MEMBERSHIP_TO_RANGE+4..MEMBERSHIP_TO_RANGE+8].try_into().unwrap());
                range[1] = r1;

                &range.try_to_vec().unwrap().as_ref()

            }, {

            let info = membership.to_account_info();
            let ref_data = info.try_borrow_data()?;

            let slot = u32::from_le_bytes(ref_data[MEMBERSHIP_TO_SLOT..MEMBERSHIP_TO_SLOT+4].try_into().unwrap());

            let slot_bytes = slot.to_le_bytes();
            &[slot_bytes[0], slot_bytes[1], slot_bytes[2], slot_bytes[3], ref_data[MEMBERSHIP_TO_TYPE], ref_data[MEMBERSHIP_TO_DATA_TYPE]]
        }],
        bump
    )]
    /// CHECK
    pub membership: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds = [b"membership_group".as_ref(), store.key().as_ref(), {

            let info = membership.to_account_info();
            let ref_data = info.try_borrow_data()?;

            let slot = u32::from_le_bytes(ref_data[MEMBERSHIP_TO_SLOT..MEMBERSHIP_TO_SLOT+4].try_into().unwrap());

            let slot_bytes = slot.to_le_bytes();
            &[slot_bytes[0], slot_bytes[1], slot_bytes[2], slot_bytes[3], ref_data[MEMBERSHIP_TO_TYPE], ref_data[MEMBERSHIP_TO_DATA_TYPE]]
        }],
        bump
    )]
    /// CHECK
    pub membership_group: UncheckedAccount<'info>,
    /// CHECK
    pub id_hash: UncheckedAccount<'info>,
    /// CHECK
    pub link_hash: UncheckedAccount<'info>,
    /// CHECK
    
    #[account(
        mut,
        seeds = [b"unique_document".as_ref(), id_hash.key().as_ref(), link_hash.key().as_ref()],
        bump
    )]
    pub unique_hash: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK
    pub membership_manager: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds = [b"membership_slot".as_ref(), membership.key().as_ref(), &registry_slot.to_le_bytes().as_ref()],
        bump
    )]
    /// CHECK
    pub membership_slot: UncheckedAccount<'info>,
    #[account(mut)]
    pub master: Box<Account<'info, Master>>,
    #[account(mut)]
    pub store: Box<Account<'info, Store>>,


    /// CHECK: unsafe
    #[account(mut)]
    pub merkle_tree: UncheckedAccount<'info>, 
    #[account(mut)]
    /// CHECK: unsafe
    pub tree_authority: UncheckedAccount<'info>,
    #[account( mut, seeds = [b"tree".as_ref()], bump )]
    /// CHECK: unsafe
    pub merkle_manager: UncheckedAccount<'info>, 
    /// CHECK: Optional collection authority record PDA.
    pub collection_authority_record_pda:UncheckedAccount<'info>,
    /// CHECK: This account is checked in the instruction
    pub edition_account: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: This account is checked in the instruction
    pub collection_metadata: UncheckedAccount<'info>,
    /// CHECK: This account is checked in the instruction
    pub collection_mint: UncheckedAccount<'info>,
    /// CHECK: This is just used as a signing PDA.
    #[account()]
    pub bubblegum_signer: UncheckedAccount<'info>,


    #[account(mut)]
    pub creator: Signer<'info>,

    
    pub log_wrapper: Program<'info, Noop>,
    pub token_metadata_program: Program<'info, MplTokenMetadata>,
    pub bubblegum_program: Program<'info, Bubblegum>,
    pub compression_program: Program<'info, SplAccountCompression>,

    pub system_program: Program<'info, System>,
}


#[derive(Accounts)]
#[instruction(registry_slot:u32, membership_sync_check:MembershipSyncCheck, timestamp:u32, proof:[u8;64], arweave:String, version:u16, bundler:AssetBundler)]
pub struct RegisterMembership<'info> {
  #[account(
        mut,
        seeds = [b"membership".as_ref(), store.key().as_ref(), {
                
                let mut range:[u32;2] = [0,0];

                let info = membership.to_account_info();
                let ref_data = info.try_borrow_data()?;

                let r0 = u32::from_le_bytes(ref_data[MEMBERSHIP_TO_RANGE..MEMBERSHIP_TO_RANGE+4].try_into().unwrap());
                range[0] = r0;

                let r1 = u32::from_le_bytes(ref_data[MEMBERSHIP_TO_RANGE+4..MEMBERSHIP_TO_RANGE+8].try_into().unwrap());
                range[1] = r1;

                &range.try_to_vec().unwrap().as_ref()

            }, {

            let info = membership.to_account_info();
            let ref_data = info.try_borrow_data()?;

            let slot = u32::from_le_bytes(ref_data[MEMBERSHIP_TO_SLOT..MEMBERSHIP_TO_SLOT+4].try_into().unwrap());

            let slot_bytes = slot.to_le_bytes();
            &[slot_bytes[0], slot_bytes[1], slot_bytes[2], slot_bytes[3], ref_data[MEMBERSHIP_TO_TYPE], ref_data[MEMBERSHIP_TO_DATA_TYPE]]
        }],
        bump
    )]
    /// CHECK
    pub membership: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds = [b"membership_group".as_ref(), store.key().as_ref(), {

            let info = membership.to_account_info();
            let ref_data = info.try_borrow_data()?;

            let slot = u32::from_le_bytes(ref_data[MEMBERSHIP_TO_SLOT..MEMBERSHIP_TO_SLOT+4].try_into().unwrap());

            let slot_bytes = slot.to_le_bytes();
            &[slot_bytes[0], slot_bytes[1], slot_bytes[2], slot_bytes[3], ref_data[MEMBERSHIP_TO_TYPE], ref_data[MEMBERSHIP_TO_DATA_TYPE]]
        }],
        bump
    )]
    /// CHECK
    pub membership_group: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK
    pub id_hash: UncheckedAccount<'info>,
    /// CHECK
    pub link_hash: UncheckedAccount<'info>,
    /// CHECK
    pub unique_hash: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK
    pub membership_manager: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds = [b"membership_slot".as_ref(), membership.key().as_ref(), &registry_slot.to_le_bytes().as_ref()],
        bump
    )]
    /// CHECK
    pub membership_slot: UncheckedAccount<'info>,
    #[account(mut)]
    pub master: Box<Account<'info, Master>>,
    #[account(mut)]
    pub store: Box<Account<'info, Store>>,


    /// CHECK: unsafe
    #[account(mut)]
    pub merkle_tree: UncheckedAccount<'info>, 
    #[account(mut)]
    /// CHECK: unsafe
    pub tree_authority: UncheckedAccount<'info>,
    #[account( mut, seeds = [b"tree".as_ref()], bump )]
    /// CHECK: unsafe
    pub merkle_manager: UncheckedAccount<'info>, 
    /// CHECK: Optional collection authority record PDA.
    pub collection_authority_record_pda:UncheckedAccount<'info>,
    /// CHECK: This account is checked in the instruction
    pub edition_account: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: This account is checked in the instruction
    pub collection_metadata: UncheckedAccount<'info>,
    /// CHECK: This account is checked in the instruction
    pub collection_mint: UncheckedAccount<'info>,
    /// CHECK: This is just used as a signing PDA.
    #[account()]
    pub bubblegum_signer: UncheckedAccount<'info>,


    #[account(mut)]
    pub creator: Signer<'info>,


    pub log_wrapper: Program<'info, Noop>,
    pub token_metadata_program: Program<'info, MplTokenMetadata>,
    pub bubblegum_program: Program<'info, Bubblegum>,
    pub compression_program: Program<'info, SplAccountCompression>,

    pub system_program: Program<'info, System>,
}


#[derive(AnchorSerialize, AnchorDeserialize, PartialEq, Eq, Debug, Clone)]
#[repr(u8)]
pub enum MembershipType {
  Birthday = 0
}

#[derive(AnchorSerialize, AnchorDeserialize, PartialEq, Eq, Debug, Clone)]
#[repr(u8)]
pub enum MembershipDataType {
  Identification = 0
}

#[derive(AnchorSerialize, AnchorDeserialize, PartialEq, Eq, Debug, Clone)]
#[repr(u8)]
pub enum AssetBundler {
  Arweave = 0,
  IrysGateway = 1,
  RawTurboDev = 2
}



#[derive(AnchorSerialize, AnchorDeserialize, PartialEq, Eq, Debug, Clone)]
#[repr(u8)]
pub enum DocumentType {
  Passport = 0,
  GovernmentId = 1,
  AgeEstimation = 2
}

#[derive(AnchorSerialize, AnchorDeserialize, PartialEq, Eq, Debug, Clone)]
pub struct MembershipSyncCheck
{ //155
  pub last_slot_members_count: u32,
  pub last_slot_hash: [u8;7]
}

const START_FROM_MEMBERSHIP:usize = 8 + MEMBERSHIP_CONFIG_SIZE;

#[derive(AnchorSerialize, AnchorDeserialize, PartialEq, Eq, Debug, Clone)]
pub struct MembershipConfig
{ //157
  pub state: u8, //1
  pub slot:u32, //4
  pub store:Pubkey, //32
  pub membership_type: MembershipType, //1
  pub membership_data_type: MembershipDataType, //1
  pub range:[u32;2], //8
  pub creator: Pubkey, //32
  pub version: u8, //1
  pub slots:u32, //4
  pub master:Pubkey, //32
  pub total_members:u64, //8
  pub ready:u8, //1
  pub extra:[u8;32], //32
}

#[derive(AnchorSerialize, AnchorDeserialize, PartialEq, Eq, Debug, Clone)]
pub struct CnftMembership
{
    pub root: [u8;32],
    pub data_hash:[u8;32],
    pub index:u32
}


#[account(zero_copy)]
#[repr(C)]
pub struct Membership {
  /*
  */
  pub data:[u8;10_000_000]
}


