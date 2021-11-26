use solana_program_test::*;
use solana_sdk::{signature::Keypair, signer::Signer, transport::TransportError};

use program_test::*;

mod program_test;

#[allow(unaligned_references)]
#[tokio::test]
async fn test_basic() -> Result<(), TransportError> {
    let context = TestContext::new().await;

    let payer = &context.users[0].key;
    let realm_authority = Keypair::new();
    let realm = context
        .governance
        .create_realm(
            "testrealm",
            realm_authority.pubkey(),
            &context.mints[0],
            &payer,
            &context.addin.program_id,
        )
        .await;

    let voter_authority = &context.users[1].key;
    let token_owner_record = realm
        .create_token_owner_record(voter_authority.pubkey(), &payer)
        .await;

    let registrar = context.addin.create_registrar(&realm, payer).await;
    let voter = context
        .addin
        .create_voter(&registrar, &voter_authority, &payer)
        .await;

    context
        .addin
        .deposit(
            &registrar,
            &voter,
            &voter_authority,
            context.users[1].token_accounts[0],
            10000,
        )
        .await?;

    // Must advance slots because withdrawing in the same slot as the deposit is forbidden
    context.solana.advance_clock_by_slots(2).await;

    context
        .addin
        .withdraw(
            &registrar,
            &voter,
            &token_owner_record,
            &voter_authority,
            context.users[1].token_accounts[0],
            10000,
        )
        .await?;

    Ok(())
}
