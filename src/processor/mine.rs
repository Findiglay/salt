use std::mem::size_of;

use solana_program::{
    account_info::AccountInfo,
    clock::Clock,
    entrypoint::ProgramResult,
    keccak::{hashv, Hash as KeccakHash},
    program::set_return_data,
    program_error::ProgramError,
    pubkey::Pubkey,
    slot_hashes::SlotHash,
    sysvar::{self, Sysvar},
};

use crate::{
    error::OreError,
    instruction::MineArgs,
    loaders::*,
    state::{Bus, Proof, Treasury},
    utils::AccountDeserialize,
    EPOCH_DURATION, START_AT,
};

/// Mine is the primary workhorse instruction of the Ore program. Its responsibilities include:
/// 1. Verify the provided hash is valid.
/// 2. Increment the user's claimable rewards counter.
/// 3. Generate a new challenge for the miner.
/// 4. Update the miner's lifetime stats.
///
/// Safety requirements:
/// - Mine is a permissionless instruction and can be called by any signer.
/// - Can only succeed if START_AT has passed.
/// - Can only succeed if the last reset was less than 60 seconds ago.
/// - Can only succeed if the provided SHA3 hash and nonce are valid and satisfy the difficulty.
/// - The the provided proof account must be associated with the signer.
/// - The provided bus, treasury, and slot hash sysvar must be valid.
pub fn process_mine<'a, 'info>(
    _program_id: &Pubkey,
    accounts: &'a [AccountInfo<'info>],
    data: &[u8],
) -> ProgramResult {
    // Parse args
    let args = MineArgs::try_from_bytes(data)?;

    // Load accounts
    let [signer, bus_info, proof_info, treasury_info, slot_hashes_info] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    load_signer(signer)?;
    load_any_bus(bus_info, true)?;
    load_proof(proof_info, signer.key, true)?;
    load_treasury(treasury_info, false)?;
    load_sysvar(slot_hashes_info, sysvar::slot_hashes::id())?;

    // Validate mining has starting
    let clock = Clock::get().or(Err(ProgramError::InvalidAccountData))?;
    if clock.unix_timestamp.lt(&START_AT) {
        return Err(OreError::NotStarted.into());
    }

    // Validate epoch is active
    let treasury_data = treasury_info.data.borrow();
    let treasury = Treasury::try_from_bytes(&treasury_data)?;
    let threshold = treasury.last_reset_at.saturating_add(EPOCH_DURATION);
    if clock.unix_timestamp.ge(&threshold) {
        return Err(OreError::NeedsReset.into());
    }

    // Validate provided hash
    let mut proof_data = proof_info.data.borrow_mut();
    let proof = Proof::try_from_bytes_mut(&mut proof_data)?;
    // let hash = validate_hash(
    //     proof.hash.into(),
    //     *signer.key,
    //     u64::from_le_bytes(args.nonce),
    //     proof.difficulty.into(),
    // )?;

    // TODO Calculate reward based on difficulty
    // TODO Calculate rewards multiplier
    // TODO Calculate reward payout amount

    // Update claimable rewards
    let mut bus_data = bus_info.data.borrow_mut();
    let bus = Bus::try_from_bytes_mut(&mut bus_data)?;
    bus.rewards = bus
        .rewards
        .checked_sub(treasury.reward_rate)
        .ok_or(OreError::BusRewardsInsufficient)?;
    proof.balance = proof.balance.saturating_add(treasury.reward_rate);

    // Hash recent slot hash into the next challenge to prevent pre-mining attacks
    proof.hash = hashv(&[
        // TODO
        // hash.as_ref(),
        &slot_hashes_info.data.borrow()[0..size_of::<SlotHash>()],
    ])
    .into();

    // Update lifetime stats
    proof.total_hashes = proof.total_hashes.saturating_add(1);
    proof.total_rewards = proof.total_rewards.saturating_add(treasury.reward_rate);

    // Log the mined rewards
    set_return_data(treasury.reward_rate.to_le_bytes().as_slice());

    Ok(())
}

/// Validates the provided hash, ensursing it is equal to SHA3(current_hash, singer, nonce).
/// Fails if the provided hash is valid but does not satisfy the required difficulty.
pub(crate) fn validate_hash(
    current_hash: KeccakHash,
    signer: Pubkey,
    nonce: u64,
    difficulty: KeccakHash,
) -> Result<KeccakHash, ProgramError> {
    let hash = hashv(&[
        nonce.to_le_bytes().as_slice(),
        current_hash.as_ref(),
        signer.as_ref(),
    ]);
    if hash.gt(&difficulty) {
        return Err(OreError::HashInvalid.into());
    }
    Ok(hash)
}

#[cfg(test)]
mod tests {
    use solana_program::{
        keccak::{Hash, HASH_BYTES},
        pubkey::Pubkey,
    };

    use crate::validate_hash;

    #[test]
    fn test_validate_hash_pass() {
        let h1 = Hash::new_from_array([1; HASH_BYTES]);
        let signer = Pubkey::new_unique();
        let nonce = 10u64;
        let difficulty = Hash::new_from_array([255; HASH_BYTES]);
        let res = validate_hash(h1, signer, nonce, difficulty);
        assert!(res.is_ok());
    }

    #[test]
    fn test_validate_hash_fail() {
        let h1 = Hash::new_from_array([1; HASH_BYTES]);
        let signer = Pubkey::new_unique();
        let nonce = 10u64;
        let difficulty = Hash::new_from_array([0; HASH_BYTES]);
        let res = validate_hash(h1, signer, nonce, difficulty);
        assert!(res.is_err());
    }
}
