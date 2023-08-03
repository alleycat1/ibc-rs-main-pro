//! Defines core commitment types

use crate::core::ics23_commitment::error::CommitmentError;
use crate::prelude::*;

use core::{convert::TryFrom, fmt};
use ibc_proto::ibc::core::commitment::v1::MerkleProof as RawMerkleProof;
use subtle_encoding::{Encoding, Hex};

use super::merkle::MerkleProof;

/// Encodes a commitment root; most often a Merkle tree root hash.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
#[derive(Clone, PartialEq, Eq)]
pub struct CommitmentRoot {
    #[cfg_attr(
        feature = "serde",
        serde(serialize_with = "crate::serializers::ser_hex_upper")
    )]
    bytes: Vec<u8>,
}

impl fmt::Debug for CommitmentRoot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let hex = Hex::upper_case()
            .encode_to_string(&self.bytes)
            .map_err(|_| fmt::Error)?;
        f.debug_tuple("CommitmentRoot").field(&hex).finish()
    }
}

impl CommitmentRoot {
    pub fn from_bytes(bytes: &[u8]) -> Self {
        Self {
            bytes: Vec::from(bytes),
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn into_vec(self) -> Vec<u8> {
        self.bytes
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }
}

impl From<Vec<u8>> for CommitmentRoot {
    fn from(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }
}

/// Demonstrates membership or non-membership for an element or set of elements,
/// verifiable in conjunction with a known commitment root.
///
/// For example, in the case of a proof of membership in a Merkle tree,
/// this encodes a Merkle proof.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
#[derive(Clone, PartialEq, Eq)]
pub struct CommitmentProofBytes {
    #[cfg_attr(
        feature = "serde",
        serde(serialize_with = "crate::serializers::ser_hex_upper")
    )]
    bytes: Vec<u8>,
}

impl fmt::Debug for CommitmentProofBytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let hex = Hex::upper_case()
            .encode_to_string(&self.bytes)
            .map_err(|_| fmt::Error)?;
        f.debug_tuple("CommitmentProof").field(&hex).finish()
    }
}

impl TryFrom<Vec<u8>> for CommitmentProofBytes {
    type Error = CommitmentError;

    fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
        if bytes.is_empty() {
            Err(Self::Error::EmptyMerkleProof)
        } else {
            Ok(Self { bytes })
        }
    }
}

impl From<CommitmentProofBytes> for Vec<u8> {
    fn from(p: CommitmentProofBytes) -> Vec<u8> {
        p.bytes
    }
}

impl TryFrom<RawMerkleProof> for CommitmentProofBytes {
    type Error = CommitmentError;

    fn try_from(proof: RawMerkleProof) -> Result<Self, Self::Error> {
        let mut buf = Vec::new();
        prost::Message::encode(&proof, &mut buf)
            .map_err(|e| Self::Error::EncodingFailure(e.to_string()))?;
        buf.try_into()
    }
}

impl TryFrom<MerkleProof> for CommitmentProofBytes {
    type Error = CommitmentError;

    fn try_from(value: MerkleProof) -> Result<Self, Self::Error> {
        Self::try_from(RawMerkleProof::from(value))
    }
}

impl TryFrom<CommitmentProofBytes> for RawMerkleProof {
    type Error = CommitmentError;

    fn try_from(value: CommitmentProofBytes) -> Result<Self, Self::Error> {
        let value: Vec<u8> = value.into();
        let res: RawMerkleProof = prost::Message::decode(value.as_ref())
            .map_err(CommitmentError::InvalidRawMerkleProof)?;
        Ok(res)
    }
}

/// Defines a store prefix of the commitment proof.
///
/// See [spec](https://github.com/cosmos/ibc/blob/main/spec/core/ics-023-vector-commitments/README.md#prefix).
#[cfg_attr(
    feature = "parity-scale-codec",
    derive(
        parity_scale_codec::Encode,
        parity_scale_codec::Decode,
        scale_info::TypeInfo
    )
)]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
#[derive(Clone, PartialEq, Eq, Hash, Default)]
pub struct CommitmentPrefix {
    bytes: Vec<u8>,
}

impl CommitmentPrefix {
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn into_vec(self) -> Vec<u8> {
        self.bytes
    }
}

impl TryFrom<Vec<u8>> for CommitmentPrefix {
    type Error = CommitmentError;

    fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
        if bytes.is_empty() {
            Err(Self::Error::EmptyCommitmentPrefix)
        } else {
            Ok(Self { bytes })
        }
    }
}

impl fmt::Debug for CommitmentPrefix {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let converted = core::str::from_utf8(self.as_bytes());
        match converted {
            Ok(s) => write!(f, "{s}"),
            Err(_e) => write!(f, "<not valid UTF8: {:?}>", self.as_bytes()),
        }
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for CommitmentPrefix {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        format!("{self:?}").serialize(serializer)
    }
}

#[cfg(test)]
pub mod test_util {
    use super::CommitmentProofBytes;
    use crate::prelude::*;
    use ibc_proto::ibc::core::commitment::v1::MerkleProof as RawMerkleProof;
    use ibc_proto::ics23::CommitmentProof;

    /// Returns a dummy `CommitmentProofBytes`, for testing only!
    pub fn get_dummy_commitment_proof_bytes() -> CommitmentProofBytes {
        let parsed = CommitmentProof { proof: None };
        let mproofs: Vec<CommitmentProof> = vec![parsed];
        let raw_mp = RawMerkleProof { proofs: mproofs };
        raw_mp.try_into().unwrap()
    }
}
