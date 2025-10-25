use mpl_bubblegum::instructions::CreateTreeConfigCpiBuilder;

use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize, PartialEq, Eq, Debug, Clone)]
pub struct CreateCnft { //17
    pub name:String,
    pub symbol:String,
    pub uri:String,
    pub fee:u16,
  pub creators:Vec<Creator>,
  pub collection:Option<Collection>,
  
}

pub fn create_nft_to_metadata(create_nft:CreateCnft) -> MetadataArgs {
    return MetadataArgs {
        name:create_nft.name,
        symbol:create_nft.symbol,
        uri:create_nft.uri,
        seller_fee_basis_points: create_nft.fee,
        creators:create_nft.creators,
        primary_sale_happened: false,
        is_mutable: true,
        edition_nonce: None,
        collection: create_nft.collection,
        uses: None,
        token_standard: Some(TokenStandard::NonFungible),
        token_program_version: TokenProgramVersion::Original,
    };
}

#[derive(AnchorSerialize, AnchorDeserialize, PartialEq, Eq, Debug, Clone)]
pub struct TokenMetadata
{
  pub name: String,
  pub symbol: String,
  pub arweave:String
}

#[derive(Clone)]
pub struct Bubblegum;
impl anchor_lang::Id for Bubblegum {
  fn id() -> Pubkey {
    mpl_bubblegum::ID
  }
}

const NOOP_ID:[u8;32] = [11, 188, 15, 192, 187, 71, 202, 47, 116, 196, 17, 46, 148, 171, 19, 207, 163, 198, 52, 229, 220, 23, 234, 203, 3, 205, 26, 35, 205, 126, 120, 124];

#[derive(Clone)]
pub struct Noop;
impl anchor_lang::Id for Noop {
  fn id() -> Pubkey {
    Pubkey::new_from_array(NOOP_ID)
  }
}

#[derive(Clone)]
pub struct SplAccountCompression;
impl anchor_lang::Id for SplAccountCompression {
  fn id() -> Pubkey {
    spl_account_compression::ID
  }
}

#[derive(AnchorSerialize, AnchorDeserialize, PartialEq, Eq, Debug, Clone)]
pub struct MetadataArgs { //490
    pub name: String, //36
    pub symbol: String, //14
    pub uri: String, //204
    pub seller_fee_basis_points: u16, //2
    pub primary_sale_happened: bool, //1
    pub is_mutable: bool, //1
    pub edition_nonce: Option<u8>, //2
    pub token_standard: Option<TokenStandard>, //2
    pub collection: Option<Collection>, //34
    pub uses: Option<Uses>, //18
    pub token_program_version: TokenProgramVersion, //2
    pub creators: Vec<Creator>, //174
}


#[derive(AnchorSerialize, AnchorDeserialize, PartialEq, Eq, Debug, Clone)]
pub enum TokenProgramVersion {
    Original,
    Token2022,
}


#[derive(AnchorSerialize, AnchorDeserialize, PartialEq, Eq, Debug, Clone)]
pub enum TokenStandard {
    NonFungible,        // This is a master edition
    FungibleAsset,      // A token with metadata that can also have attributes
    Fungible,           // A token with simple metadata
    NonFungibleEdition, // This is a limited edition
}
#[derive(AnchorSerialize, AnchorDeserialize, PartialEq, Eq, Debug, Clone)]
pub struct Creator {
    pub address: Pubkey,
    pub verified: bool,
    // In percentages, NOT basis points ;) Watch out!
    pub share: u8,
}
#[derive(AnchorSerialize, AnchorDeserialize, PartialEq, Eq, Debug, Clone)]
pub enum UseMethod {
    Burn,
    Multiple,
    Single,
}

#[derive(AnchorSerialize, AnchorDeserialize, PartialEq, Eq, Debug, Clone)]
pub struct Uses {
    // 17 bytes + Option byte
    pub use_method: UseMethod, //1
    pub remaining: u64,        //8
    pub total: u64,            //8
}

#[repr(C)]
#[derive(AnchorSerialize, AnchorDeserialize, PartialEq, Eq, Debug, Clone)]
pub struct Collection {
    pub verified: bool,
    pub key: Pubkey,
}

pub fn mint_to_collection_cnft<'a>(
  bubblegum_program: &AccountInfo<'a>,
  tree_authority: &AccountInfo<'a>,
  owner: &AccountInfo<'a>,
  delegate: &AccountInfo<'a>,
  merkle_tree: &AccountInfo<'a>,
  payer: &AccountInfo<'a>,
  tree_delegate: &AccountInfo<'a>,
  collection_authority: &AccountInfo<'a>,
  collection_authority_record_pda: &AccountInfo<'a>,
  collection_mint: &AccountInfo<'a>,
  collection_metadata: &AccountInfo<'a>,
  edition_account: &AccountInfo<'a>,
  bubblegum_signer: &AccountInfo<'a>,
  log_wrapper: &AccountInfo<'a>,
  compression_program: &AccountInfo<'a>,
  token_metadata_program: &AccountInfo<'a>,
  system_program: &AccountInfo<'a>,
  metadata:MetadataArgs,
  signature:&[&[&[u8]]],
  remaining_accounts:&[AccountInfo<'a>]
) -> Result<()> {
  
  
  let remaining_accounts_len = remaining_accounts.len();
   let mut accounts = Vec::with_capacity(
    16 // space for the 7 AccountMetas
    + remaining_accounts_len,
   );
   accounts.extend(vec![
    AccountMeta::new(tree_authority.key(), false), //tree_auth
    AccountMeta::new_readonly(owner.key(), false), //leaf_owner
    AccountMeta::new_readonly(delegate.key(), false), //leaf_delegate
    AccountMeta::new(merkle_tree.key(), false), //merkle_tree
    AccountMeta::new_readonly(payer.key(), true), //payer
    AccountMeta::new_readonly(tree_delegate.key(), true), //tree_delegate
    AccountMeta::new_readonly(collection_authority.key(), true), //collection_authority
    AccountMeta::new_readonly(collection_authority_record_pda.key(), false), //collection_authority_record_pda
    AccountMeta::new_readonly(collection_mint.key(), false), //collection_mint
    AccountMeta::new(collection_metadata.key(), false), //collection_metadata
    AccountMeta::new_readonly(edition_account.key(), false), //edition_account
    AccountMeta::new_readonly(bubblegum_signer.key(), false), //bubblegum_signer
    AccountMeta::new_readonly(log_wrapper.key(), false),
    AccountMeta::new_readonly(compression_program.key(), false),
    AccountMeta::new_readonly(token_metadata_program.key(), false),
    AccountMeta::new_readonly(system_program.key(), false),
   ]);
   
   let mint_to_collection_discriminator: [u8; 8] = [153, 18, 178, 47, 197, 158, 86, 15];
   
   let metadata_vec = metadata.try_to_vec().unwrap();
   
   
   
   let mut data = Vec::with_capacity(
     8 // The length of mint_to_collection_discriminator,
     + metadata_vec.len()
  );
  
  data.extend(mint_to_collection_discriminator);
  data.extend(metadata_vec);
  
  let mut account_infos = Vec::with_capacity(
    16 // space for the 7 AccountInfos
    + remaining_accounts_len,
   );
   
   account_infos.extend(vec![
     tree_authority.clone(), //tree_auth
     owner.clone(), //leaf_owner
     delegate.clone(), //leaf_delegate
     merkle_tree.clone(), //merkle_tree
     payer.clone(), //payer
     tree_delegate.clone(), //tree_delegate
     collection_authority.clone(), //collection_authority
     collection_authority_record_pda.clone(), //collection_authority_record_pda
     collection_mint.clone(), //collection_mint
     collection_metadata.clone(), //collection_metadata
     edition_account.clone(), //edition_account
     bubblegum_signer.clone(), //bubblegum_signer
     log_wrapper.clone(),
     compression_program.clone(),
     token_metadata_program.clone(),
     system_program.clone(),
    ]);
    
     for acc in remaining_accounts.iter() {
      accounts.push(AccountMeta::new_readonly(acc.key(), true));
      account_infos.push(acc.clone());
     }
     
   let instruction = solana_program::instruction::Instruction {
     program_id: bubblegum_program.key(),
     accounts,
     data,
    };
    
     
   let acc2 = account_infos.clone();
   solana_program::program::invoke_signed(&instruction, &acc2[..], signature)?;
   
  
  Ok(())
}

pub fn burn_cnft<'a>(
   tree_authority: &AccountInfo<'a>,
   owner: &AccountInfo<'a>,
   delegate: &AccountInfo<'a>,
   merkle_tree: &AccountInfo<'a>,
   log_wrapper: &AccountInfo<'a>,
   compression_program: &AccountInfo<'a>,
   system_program: &AccountInfo<'a>,
   bubblegum_program: &AccountInfo<'a>,
   root:[u8; 32],
   data_hash:[u8; 32],
   creator_hash:[u8; 32],
   nonce:u64,
   index:u32,
   signature:Vec<&[u8]>,
   remaining_accounts:&[AccountInfo<'a>]
) -> Result<()> {
 
   let remaining_accounts_len = remaining_accounts.len();
   let mut accounts = Vec::with_capacity(
    7 // space for the 7 AccountMetas
    + remaining_accounts_len,
   );
   accounts.extend(vec![
    AccountMeta::new_readonly(tree_authority.key(), false),
    AccountMeta::new_readonly(owner.key(), false),
    AccountMeta::new_readonly(delegate.key(), true), //leaf_delegate
    AccountMeta::new(merkle_tree.key(), false),
    AccountMeta::new_readonly(log_wrapper.key(), false),
    AccountMeta::new_readonly(compression_program.key(), false),
    AccountMeta::new_readonly(system_program.key(), false),
   ]);
   
   let burn_discriminator: [u8; 8] = [116, 110, 29, 56, 107, 219, 42, 93];
   
   let mut data = Vec::with_capacity(
    8 // The length of burn_discriminator,
    + root.len()
    + data_hash.len()
    + creator_hash.len()
    + 8 // The length of the nonce
    + 4, // The length of the index
   );
   
   data.extend(burn_discriminator);
   data.extend(root);
   data.extend(data_hash);
   data.extend(creator_hash);
   data.extend(nonce.to_le_bytes());
   data.extend(index.to_le_bytes());
   
   let mut account_infos = Vec::with_capacity(
    7 // space for the 7 AccountInfos
    + remaining_accounts_len,
   );
   
   
   account_infos.extend(vec![
    tree_authority.clone(),
    owner.clone(),
    delegate.clone(), //leaf delegate
    merkle_tree.clone(),
    log_wrapper.clone(),
    compression_program.clone(),
    system_program.clone(),
   ]);
   
   for acc in remaining_accounts.iter() {
    accounts.push(AccountMeta::new_readonly(acc.key(), false));
    account_infos.push(acc.clone());
   }
   
   let instruction = solana_program::instruction::Instruction {
    program_id: bubblegum_program.key(),
    accounts,
    data,
   };
   
   let outer = &[signature.as_slice()];
   
   let acc2 = account_infos.clone();
   
   solana_program::program::invoke_signed(&instruction, &acc2[..], outer)?;
   
   
   
   Ok(())
}