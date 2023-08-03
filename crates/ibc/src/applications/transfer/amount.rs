//! Contains the `Amount` type, which represents amounts of tokens transferred.

use core::{ops::Deref, str::FromStr};
use derive_more::{Display, From, Into};

use super::error::TokenTransferError;
use primitive_types::U256;

/// A type for representing token transfer amounts.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord, Display, From, Into)]
pub struct Amount(U256);

#[cfg(feature = "parity-scale-codec")]
impl parity_scale_codec::WrapperTypeDecode for Amount {
    type Wrapped = [u64; 4];
}

#[cfg(feature = "parity-scale-codec")]
impl parity_scale_codec::WrapperTypeEncode for Amount {}

impl Deref for Amount {
    type Target = [u64; 4];

    fn deref(&self) -> &Self::Target {
        &self.0 .0
    }
}

impl From<[u64; 4]> for Amount {
    fn from(value: [u64; 4]) -> Self {
        Self(U256(value))
    }
}

impl Amount {
    pub fn checked_add(self, rhs: Self) -> Option<Self> {
        self.0.checked_add(rhs.0).map(Self)
    }

    pub fn checked_sub(self, rhs: Self) -> Option<Self> {
        self.0.checked_sub(rhs.0).map(Self)
    }
}

impl AsRef<U256> for Amount {
    fn as_ref(&self) -> &U256 {
        &self.0
    }
}

impl FromStr for Amount {
    type Err = TokenTransferError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let amount = U256::from_dec_str(s).map_err(TokenTransferError::InvalidAmount)?;
        Ok(Self(amount))
    }
}

impl From<u64> for Amount {
    fn from(v: u64) -> Self {
        Self(v.into())
    }
}
