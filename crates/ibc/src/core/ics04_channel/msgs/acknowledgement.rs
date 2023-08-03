use crate::prelude::*;

use ibc_proto::ibc::core::channel::v1::MsgAcknowledgement as RawMsgAcknowledgement;
use ibc_proto::protobuf::Protobuf;

use crate::core::ics04_channel::acknowledgement::Acknowledgement;
use crate::core::ics04_channel::error::PacketError;
use crate::core::ics04_channel::packet::Packet;
use crate::core::ics23_commitment::commitment::CommitmentProofBytes;
use crate::core::Msg;
use crate::signer::Signer;
use crate::Height;

pub(crate) const TYPE_URL: &str = "/ibc.core.channel.v1.MsgAcknowledgement";

///
/// Message definition for packet acknowledgements.
///
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MsgAcknowledgement {
    pub packet: Packet,
    pub acknowledgement: Acknowledgement,
    /// Proof of packet acknowledgement on the receiving chain
    pub proof_acked_on_b: CommitmentProofBytes,
    /// Height at which the commitment proof in this message were taken
    pub proof_height_on_b: Height,
    pub signer: Signer,
}

impl Msg for MsgAcknowledgement {
    type Raw = RawMsgAcknowledgement;

    fn type_url(&self) -> String {
        TYPE_URL.to_string()
    }
}

impl Protobuf<RawMsgAcknowledgement> for MsgAcknowledgement {}

impl TryFrom<RawMsgAcknowledgement> for MsgAcknowledgement {
    type Error = PacketError;

    fn try_from(raw_msg: RawMsgAcknowledgement) -> Result<Self, Self::Error> {
        Ok(MsgAcknowledgement {
            packet: raw_msg
                .packet
                .ok_or(PacketError::MissingPacket)?
                .try_into()?,
            acknowledgement: raw_msg.acknowledgement.try_into()?,
            proof_acked_on_b: raw_msg
                .proof_acked
                .try_into()
                .map_err(|_| PacketError::InvalidProof)?,
            proof_height_on_b: raw_msg
                .proof_height
                .and_then(|raw_height| raw_height.try_into().ok())
                .ok_or(PacketError::MissingHeight)?,
            signer: raw_msg.signer.into(),
        })
    }
}

impl From<MsgAcknowledgement> for RawMsgAcknowledgement {
    fn from(domain_msg: MsgAcknowledgement) -> Self {
        RawMsgAcknowledgement {
            packet: Some(domain_msg.packet.into()),
            acknowledgement: domain_msg.acknowledgement.into(),
            signer: domain_msg.signer.to_string(),
            proof_height: Some(domain_msg.proof_height_on_b.into()),
            proof_acked: domain_msg.proof_acked_on_b.into(),
        }
    }
}

#[cfg(test)]
pub mod test_util {
    use ibc_proto::ibc::core::channel::v1::MsgAcknowledgement as RawMsgAcknowledgement;
    use ibc_proto::ibc::core::channel::v1::Packet as RawPacket;
    use ibc_proto::ibc::core::client::v1::Height as RawHeight;

    use crate::core::ics04_channel::packet::test_utils::get_dummy_raw_packet;
    use crate::test_utils::{get_dummy_bech32_account, get_dummy_proof};

    /// Returns a dummy `RawMsgAcknowledgement`, for testing only!
    /// The `height` parametrizes both the proof height as well as the timeout height.
    pub fn get_dummy_raw_msg_acknowledgement(height: u64) -> RawMsgAcknowledgement {
        get_dummy_raw_msg_ack_with_packet(get_dummy_raw_packet(height, 1), height)
    }

    pub fn get_dummy_raw_msg_ack_with_packet(
        packet: RawPacket,
        height: u64,
    ) -> RawMsgAcknowledgement {
        RawMsgAcknowledgement {
            packet: Some(packet),
            acknowledgement: get_dummy_proof(),
            proof_acked: get_dummy_proof(),
            proof_height: Some(RawHeight {
                revision_number: 0,
                revision_height: height,
            }),
            signer: get_dummy_bech32_account(),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::prelude::*;

    use test_log::test;

    use ibc_proto::ibc::core::channel::v1::MsgAcknowledgement as RawMsgAcknowledgement;

    use crate::core::ics04_channel::error::PacketError;
    use crate::core::ics04_channel::msgs::acknowledgement::test_util::get_dummy_raw_msg_acknowledgement;
    use crate::core::ics04_channel::msgs::acknowledgement::MsgAcknowledgement;
    use crate::test_utils::get_dummy_bech32_account;

    #[test]
    fn msg_acknowledgment_try_from_raw() {
        struct Test {
            name: String,
            raw: RawMsgAcknowledgement,
            want_pass: bool,
        }

        let height = 50;
        let default_raw_msg = get_dummy_raw_msg_acknowledgement(height);

        let tests: Vec<Test> = vec![
            Test {
                name: "Good parameters".to_string(),
                raw: default_raw_msg.clone(),
                want_pass: true,
            },
            Test {
                name: "Missing packet".to_string(),
                raw: RawMsgAcknowledgement {
                    packet: None,
                    ..default_raw_msg.clone()
                },
                want_pass: false,
            },
            Test {
                name: "Missing proof height".to_string(),
                raw: RawMsgAcknowledgement {
                    proof_height: None,
                    ..default_raw_msg.clone()
                },
                want_pass: false,
            },
            Test {
                name: "Empty signer".to_string(),
                raw: RawMsgAcknowledgement {
                    signer: get_dummy_bech32_account(),
                    ..default_raw_msg.clone()
                },
                want_pass: true,
            },
            Test {
                name: "Empty proof acked".to_string(),
                raw: RawMsgAcknowledgement {
                    proof_acked: Vec::new(),
                    ..default_raw_msg
                },
                want_pass: false,
            },
        ];

        for test in tests {
            let res_msg: Result<MsgAcknowledgement, PacketError> = test.raw.clone().try_into();

            assert_eq!(
                res_msg.is_ok(),
                test.want_pass,
                "MsgAcknowledgement::try_from failed for test {} \nraw message: {:?} with error: {:?}",
                test.name,
                test.raw,
                res_msg.err()
            );
        }
    }
}
