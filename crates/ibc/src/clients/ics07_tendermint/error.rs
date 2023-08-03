//! Defines the Tendermint light client's error type

use crate::prelude::*;

use crate::core::ics02_client::error::ClientError;
use crate::core::ics24_host::identifier::{ClientId, IdentifierError};
use crate::Height;

use core::time::Duration;

use displaydoc::Display;
use tendermint::{Error as TendermintError, Hash};
use tendermint_light_client_verifier::errors::VerificationErrorDetail as LightClientErrorDetail;
use tendermint_light_client_verifier::operations::VotingPowerTally;
use tendermint_light_client_verifier::Verdict;

/// The main error type
#[derive(Debug, Display)]
pub enum Error {
    /// invalid identifier: `{0}`
    InvalidIdentifier(IdentifierError),
    /// invalid header, failed basic validation: `{reason}`, error: `{error}`
    InvalidHeader {
        reason: String,
        error: TendermintError,
    },
    /// invalid client state trust threshold: `{reason}`
    InvalidTrustThreshold { reason: String },
    /// invalid tendermint client state trust threshold error: `{0}`
    InvalidTendermintTrustThreshold(TendermintError),
    /// invalid client state max clock drift: `{reason}`
    InvalidMaxClockDrift { reason: String },
    /// invalid client state latest height: `{reason}`
    InvalidLatestHeight { reason: String },
    /// missing signed header
    MissingSignedHeader,
    /// invalid header, failed basic validation: `{reason}`
    Validation { reason: String },
    /// invalid raw client state: `{reason}`
    InvalidRawClientState { reason: String },
    /// missing validator set
    MissingValidatorSet,
    /// missing trusted next validator set
    MissingTrustedNextValidatorSet,
    /// missing trusted height
    MissingTrustedHeight,
    /// missing trusting period
    MissingTrustingPeriod,
    /// missing unbonding period
    MissingUnbondingPeriod,
    /// negative max clock drift
    NegativeMaxClockDrift,
    /// missing latest height
    MissingLatestHeight,
    /// invalid raw header error: `{0}`
    InvalidRawHeader(TendermintError),
    /// invalid raw misbehaviour: `{reason}`
    InvalidRawMisbehaviour { reason: String },
    /// decode error: `{0}`
    Decode(prost::DecodeError),
    /// given other previous updates, header timestamp should be at most `{max}`, but was `{actual}`
    HeaderTimestampTooHigh { actual: String, max: String },
    /// given other previous updates, header timestamp should be at least `{min}`, but was `{actual}`
    HeaderTimestampTooLow { actual: String, min: String },
    /// header revision height = `{height}` is invalid
    InvalidHeaderHeight { height: u64 },
    /// Disallowed to create a new client with a frozen height
    FrozenHeightNotAllowed,
    /// the header's trusted revision number (`{trusted_revision}`) and the update's revision number (`{header_revision}`) should be the same
    MismatchHeightRevisions {
        trusted_revision: u64,
        header_revision: u64,
    },
    /// the given chain-id (`{given}`) does not match the chain-id of the client (`{expected}`)
    MismatchHeaderChainId { given: String, expected: String },
    /// not enough trust because insufficient validators overlap: `{reason}`
    NotEnoughTrustedValsSigned { reason: VotingPowerTally },
    /// verification failed: `{detail}`
    VerificationError { detail: LightClientErrorDetail },
    /// Processed time for the client `{client_id}` at height `{height}` not found
    ProcessedTimeNotFound { client_id: ClientId, height: Height },
    /// Processed height for the client `{client_id}` at height `{height}` not found
    ProcessedHeightNotFound { client_id: ClientId, height: Height },
    /// The given hash of the validators does not matches the given hash in the signed header. Expected: `{signed_header_validators_hash}`, got: `{validators_hash}`
    MismatchValidatorsHashes {
        validators_hash: Hash,
        signed_header_validators_hash: Hash,
    },
    /// current timestamp minus the latest consensus state timestamp is greater than or equal to the trusting period (`{duration_since_consensus_state:?}` >= `{trusting_period:?}`)
    ConsensusStateTimestampGteTrustingPeriod {
        duration_since_consensus_state: Duration,
        trusting_period: Duration,
    },
    /// headers block hashes are equal
    MisbehaviourHeadersBlockHashesEqual,
    /// headers are not at same height and are monotonically increasing
    MisbehaviourHeadersNotAtSameHeight,
}

#[cfg(feature = "std")]
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self {
            Self::InvalidIdentifier(e) => Some(e),
            Self::InvalidHeader { error: e, .. } => Some(e),
            Self::InvalidTendermintTrustThreshold(e) => Some(e),
            Self::InvalidRawHeader(e) => Some(e),
            Self::Decode(e) => Some(e),
            _ => None,
        }
    }
}

impl From<Error> for ClientError {
    fn from(e: Error) -> Self {
        Self::ClientSpecific {
            description: e.to_string(),
        }
    }
}

impl From<IdentifierError> for Error {
    fn from(e: IdentifierError) -> Self {
        Self::InvalidIdentifier(e)
    }
}

pub(crate) trait IntoResult<T, E> {
    fn into_result(self) -> Result<T, E>;
}

impl IntoResult<(), Error> for Verdict {
    fn into_result(self) -> Result<(), Error> {
        match self {
            Verdict::Success => Ok(()),
            Verdict::NotEnoughTrust(reason) => Err(Error::NotEnoughTrustedValsSigned { reason }),
            Verdict::Invalid(detail) => Err(Error::VerificationError { detail }),
        }
    }
}
