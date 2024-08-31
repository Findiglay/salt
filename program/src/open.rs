use std::mem::size_of;

use coal_api::{consts::*, instruction::OpenArgs, loaders::*, state::{CoalProof, WoodProof}};
use solana_program::{
    account_info::AccountInfo,
    clock::Clock,
    entrypoint::ProgramResult,
    keccak::hashv,
    program_error::ProgramError,
    slot_hashes::SlotHash,
    system_program,
    sysvar::{self, Sysvar},
};

use crate::utils::{create_pda, AccountDeserialize, Discriminator};

pub fn process_open<'a, 'info>(accounts: &'a [AccountInfo<'info>], data: &[u8]) -> ProgramResult {
    let proof_info = &accounts[3];

    if proof_info.data.borrow()[0].eq(&(CoalProof::discriminator() as u8)) {
        return process_open_coal(accounts, data)
    }

    if proof_info.data.borrow()[0].eq(&(WoodProof::discriminator() as u8)) {
        return process_open_wood(accounts, data)
    }

    return Err(solana_program::program_error::ProgramError::InvalidAccountData);
}

/// Open creates a new proof account to track a miner's state.
fn process_open_coal<'a, 'info>(accounts: &'a [AccountInfo<'info>], data: &[u8]) -> ProgramResult {
    // Parse args.
    let args = OpenArgs::try_from_bytes(data)?;

    // Load accounts.
    let [signer, miner_info, payer_info, proof_info, system_program, slot_hashes_info] = accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    load_signer(signer)?;
    load_any(miner_info, false)?;
    load_signer(payer_info)?;
    load_uninitialized_pda(
        proof_info,
        &[COAL_PROOF, signer.key.as_ref()],
        args.bump,
        &coal_api::id(),
    )?;
    load_program(system_program, system_program::id())?;
    load_sysvar(slot_hashes_info, sysvar::slot_hashes::id())?;

    // Initialize proof.
    create_pda(
        proof_info,
        &coal_api::id(),
        8 + size_of::<CoalProof>(),
        &[COAL_PROOF, signer.key.as_ref(), &[args.bump]],
        system_program,
        payer_info,
    )?;
    let clock = Clock::get().or(Err(ProgramError::InvalidAccountData))?;
    let mut proof_data = proof_info.data.borrow_mut();
    proof_data[0] = CoalProof::discriminator() as u8;
    let proof = CoalProof::try_from_bytes_mut(&mut proof_data)?;
    proof.authority = *signer.key;
    proof.balance = 0;
    proof.challenge = hashv(&[
        signer.key.as_ref(),
        &slot_hashes_info.data.borrow()[0..size_of::<SlotHash>()],
    ])
    .0;
    proof.last_hash = [0; 32];
    proof.last_hash_at = clock.unix_timestamp;
    proof.last_stake_at = clock.unix_timestamp;
    proof.miner = *miner_info.key;
    proof.total_hashes = 0;
    proof.total_rewards = 0;

    Ok(())
}

/// Open creates a new proof account to track a miner's state.
fn process_open_wood<'a, 'info>(accounts: &'a [AccountInfo<'info>], data: &[u8]) -> ProgramResult {
    // Parse args.
    let args = OpenArgs::try_from_bytes(data)?;

    // Load accounts.
    let [signer, miner_info, payer_info, proof_info, system_program, slot_hashes_info] = accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    load_signer(signer)?;
    load_any(miner_info, false)?;
    load_signer(payer_info)?;
    load_uninitialized_pda(
        proof_info,
        &[WOOD_PROOF, signer.key.as_ref()],
        args.bump,
        &coal_api::id(),
    )?;
    load_program(system_program, system_program::id())?;
    load_sysvar(slot_hashes_info, sysvar::slot_hashes::id())?;

    // Initialize proof.
    create_pda(
        proof_info,
        &coal_api::id(),
        8 + size_of::<CoalProof>(),
        &[WOOD_PROOF, signer.key.as_ref(), &[args.bump]],
        system_program,
        payer_info,
    )?;
    let clock = Clock::get().or(Err(ProgramError::InvalidAccountData))?;
    let mut proof_data = proof_info.data.borrow_mut();
    proof_data[0] = CoalProof::discriminator() as u8;
    let proof = CoalProof::try_from_bytes_mut(&mut proof_data)?;
    proof.authority = *signer.key;
    proof.balance = 0;
    proof.challenge = hashv(&[
        signer.key.as_ref(),
        &slot_hashes_info.data.borrow()[0..size_of::<SlotHash>()],
    ])
    .0;
    proof.last_hash = [0; 32];
    proof.last_hash_at = clock.unix_timestamp;
    proof.last_stake_at = clock.unix_timestamp;
    proof.miner = *miner_info.key;
    proof.total_hashes = 0;
    proof.total_rewards = 0;

    Ok(())
}
