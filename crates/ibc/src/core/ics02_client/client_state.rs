//! Defines `ClientState`, the core type to be implemented by light clients

use core::fmt::Debug;
use core::marker::{Send, Sync};
use core::time::Duration;

use ibc_proto::google::protobuf::Any;

use crate::core::ics02_client::client_type::ClientType;
use crate::core::ics02_client::error::ClientError;
use crate::core::ics02_client::ClientExecutionContext;
use crate::core::ics23_commitment::commitment::{
    CommitmentPrefix, CommitmentProofBytes, CommitmentRoot,
};
use crate::core::ics24_host::identifier::ClientId;
use crate::core::ics24_host::path::Path;
use crate::prelude::*;
use crate::Height;

/// `UpdateKind` represents the 2 ways that a client can be updated
/// in IBC: either through a `MsgUpdateClient`, or a `MsgSubmitMisbehaviour`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UpdateKind {
    /// this is the typical scenario where a new header is submitted to the client
    /// to update the client. Note that light clients are free to define the type
    /// of the object used to update them (e.g. could be a list of headers).
    UpdateClient,
    /// this is the scenario where misbehaviour is submitted to the client
    /// (e.g 2 headers with the same height in Tendermint)
    SubmitMisbehaviour,
}

/// `ClientState` methods needed in both validation and execution.
///
/// They do not require access to a client `ValidationContext` nor
/// `ExecutionContext`.
pub trait ClientStateCommon {
    /// Performs basic validation on the `consensus_state`.
    ///
    /// Notably, an implementation should verify that it can properly
    /// deserialize the object into the expected format.
    fn verify_consensus_state(&self, consensus_state: Any) -> Result<(), ClientError>;

    /// Type of client associated with this state (eg. Tendermint)
    fn client_type(&self) -> ClientType;

    /// Latest height the client was updated to
    fn latest_height(&self) -> Height;

    /// Validate that the client is at a sufficient height
    fn validate_proof_height(&self, proof_height: Height) -> Result<(), ClientError>;

    /// Assert that the client is not frozen
    fn confirm_not_frozen(&self) -> Result<(), ClientError>;

    /// Check if the state is expired when `elapsed` time has passed since the latest consensus
    /// state timestamp
    fn expired(&self, elapsed: Duration) -> bool;

    /// Verify the upgraded client and consensus states and validate proofs
    /// against the given root.
    ///
    /// NOTE: proof heights are not included as upgrade to a new revision is
    /// expected to pass only on the last height committed by the current
    /// revision. Clients are responsible for ensuring that the planned last
    /// height of the current revision is somehow encoded in the proof
    /// verification process. This is to ensure that no premature upgrades
    /// occur, since upgrade plans committed to by the counterparty may be
    /// cancelled or modified before the last planned height.
    fn verify_upgrade_client(
        &self,
        upgraded_client_state: Any,
        upgraded_consensus_state: Any,
        proof_upgrade_client: CommitmentProofBytes,
        proof_upgrade_consensus_state: CommitmentProofBytes,
        root: &CommitmentRoot,
    ) -> Result<(), ClientError>;

    // Verify_membership is a generic proof verification method which verifies a
    // proof of the existence of a value at a given Path.
    fn verify_membership(
        &self,
        prefix: &CommitmentPrefix,
        proof: &CommitmentProofBytes,
        root: &CommitmentRoot,
        path: Path,
        value: Vec<u8>,
    ) -> Result<(), ClientError>;

    // Verify_non_membership is a generic proof verification method which
    // verifies the absence of a given commitment.
    fn verify_non_membership(
        &self,
        prefix: &CommitmentPrefix,
        proof: &CommitmentProofBytes,
        root: &CommitmentRoot,
        path: Path,
    ) -> Result<(), ClientError>;
}

/// `ClientState` methods which require access to the client's
/// `ValidationContext`.
///
/// The `ClientValidationContext` enables the light client implementation to
/// define its own `ValidationContext` trait and use it in its implementation.
///
/// ```ignore
/// impl<ClientValidationContext> ClientStateValidation<ClientValidationContext> for MyClientState
/// where
///     ClientValidationContext: MyValidationContext,
/// {
///   // `MyValidationContext` methods available
/// }
///
/// trait MyValidationContext {
///   // My Context methods
/// }
/// ```
pub trait ClientStateValidation<ClientValidationContext> {
    /// verify_client_message must verify a client_message. A client_message
    /// could be a Header, Misbehaviour. It must handle each type of
    /// client_message appropriately. Calls to check_for_misbehaviour,
    /// update_state, and update_state_on_misbehaviour will assume that the
    /// content of the client_message has been verified and can be trusted. An
    /// error should be returned if the client_message fails to verify.
    fn verify_client_message(
        &self,
        ctx: &ClientValidationContext,
        client_id: &ClientId,
        client_message: Any,
        update_kind: &UpdateKind,
    ) -> Result<(), ClientError>;

    /// Checks for evidence of a misbehaviour in Header or Misbehaviour type. It
    /// assumes the client_message has already been verified.
    fn check_for_misbehaviour(
        &self,
        ctx: &ClientValidationContext,
        client_id: &ClientId,
        client_message: Any,
        update_kind: &UpdateKind,
    ) -> Result<bool, ClientError>;
}

/// `ClientState` methods which require access to the client's
/// `ExecutionContext`.
///
/// A client can define its own `ExecutionContext` in a manner analogous to how
/// it can define a `ValidationContext` in [`ClientStateValidation`]. The one
/// difference is every client's `ExecutionContext` must have
/// [`ClientExecutionContext`] as a supertrait, which provides a set of common
/// methods to store a client state and consensus state.
pub trait ClientStateExecution<E>
where
    E: ClientExecutionContext,
{
    /// Initialises the client with the initial client and consensus states.
    ///
    /// Most clients will want to call `E::store_client_state` and
    /// `E::store_consensus_state`.
    fn initialise(
        &self,
        ctx: &mut E,
        client_id: &ClientId,
        consensus_state: Any,
    ) -> Result<(), ClientError>;

    /// Updates and stores as necessary any associated information for an IBC
    /// client, such as the ClientState and corresponding ConsensusState. Upon
    /// successful update, a list of consensus heights is returned. It assumes
    /// the client_message has already been verified.
    ///
    /// Note that `header` is the field associated with `UpdateKind::UpdateClient`.
    ///
    /// Post-condition: on success, the return value MUST contain at least one
    /// height.
    fn update_state(
        &self,
        ctx: &mut E,
        client_id: &ClientId,
        header: Any,
    ) -> Result<Vec<Height>, ClientError>;

    /// update_state_on_misbehaviour should perform appropriate state changes on
    /// a client state given that misbehaviour has been detected and verified
    fn update_state_on_misbehaviour(
        &self,
        ctx: &mut E,
        client_id: &ClientId,
        client_message: Any,
        update_kind: &UpdateKind,
    ) -> Result<(), ClientError>;

    // Update the client state and consensus state in the store with the upgraded ones.
    fn update_state_on_upgrade(
        &self,
        ctx: &mut E,
        client_id: &ClientId,
        upgraded_client_state: Any,
        upgraded_consensus_state: Any,
    ) -> Result<Height, ClientError>;
}

/// Derive macro that implements [`ClientState`] for enums containing variants
/// that implement [`ClientState`].
///
/// The macro expects the attribute `#[generics(ClientValidationContext = <...>,
/// ClientExecutionContext = <...>)]` which specifies [`ClientState`]'s generic
/// arguments to be defined.
///
/// The macro does not support generic types for `ClientValidationContext` and
/// `ClientExecutionContext` (e.g. `MyType<T>` would not be supported).
pub use ibc_derive::ClientState;

/// Primary client trait. Defines all the methods that clients must implement.
///
/// `ClientState` is broken up in 3 separate traits to avoid needing to use
/// fully qualified syntax for every method call (see ADR 7 for more details).
/// One only needs to implement [`ClientStateCommon`], [`ClientStateValidation`]
/// and [`ClientStateExecution`]; a blanket implementation will automatically
/// implement `ClientState`.
///
/// Refer to [`ClientStateValidation`] and [`ClientStateExecution`] to learn
/// more about what both generic parameters represent.
pub trait ClientState<ClientValidationContext, E: ClientExecutionContext>:
    Send
    + Sync
    + ClientStateCommon
    + ClientStateValidation<ClientValidationContext>
    + ClientStateExecution<E>
{
}

impl<ClientValidationContext, E: ClientExecutionContext, T> ClientState<ClientValidationContext, E>
    for T
where
    T: Send
        + Sync
        + ClientStateCommon
        + ClientStateValidation<ClientValidationContext>
        + ClientStateExecution<E>,
{
}
