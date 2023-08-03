//! Protocol logic specific to ICS4 messages of type `MsgChannelOpenTry`.

use crate::prelude::*;
use ibc_proto::protobuf::Protobuf;

use crate::core::events::{IbcEvent, MessageEvent};
use crate::core::ics02_client::client_state::ClientStateCommon;
use crate::core::ics02_client::consensus_state::ConsensusState;
use crate::core::ics03_connection::connection::State as ConnectionState;
use crate::core::ics04_channel::channel::State;
use crate::core::ics04_channel::channel::{ChannelEnd, Counterparty, State as ChannelState};
use crate::core::ics04_channel::error::ChannelError;
use crate::core::ics04_channel::events::OpenTry;
use crate::core::ics04_channel::msgs::chan_open_try::MsgChannelOpenTry;
use crate::core::ics24_host::identifier::ChannelId;
use crate::core::ics24_host::path::Path;
use crate::core::ics24_host::path::{ChannelEndPath, ClientConsensusStatePath};
use crate::core::ics24_host::path::{SeqAckPath, SeqRecvPath, SeqSendPath};
use crate::core::router::ModuleId;
use crate::core::{ContextError, ExecutionContext, ValidationContext};

pub(crate) fn chan_open_try_validate<ValCtx>(
    ctx_b: &ValCtx,
    module_id: ModuleId,
    msg: MsgChannelOpenTry,
) -> Result<(), ContextError>
where
    ValCtx: ValidationContext,
{
    validate(ctx_b, &msg)?;

    let chan_id_on_b = ChannelId::new(ctx_b.channel_counter()?);

    let module = ctx_b
        .get_route(&module_id)
        .ok_or(ChannelError::RouteNotFound)?;
    module.on_chan_open_try_validate(
        msg.ordering,
        &msg.connection_hops_on_b,
        &msg.port_id_on_b,
        &chan_id_on_b,
        &Counterparty::new(msg.port_id_on_a.clone(), Some(msg.chan_id_on_a.clone())),
        &msg.version_supported_on_a,
    )?;

    Ok(())
}

pub(crate) fn chan_open_try_execute<ExecCtx>(
    ctx_b: &mut ExecCtx,
    module_id: ModuleId,
    msg: MsgChannelOpenTry,
) -> Result<(), ContextError>
where
    ExecCtx: ExecutionContext,
{
    let chan_id_on_b = ChannelId::new(ctx_b.channel_counter()?);
    let module = ctx_b
        .get_route_mut(&module_id)
        .ok_or(ChannelError::RouteNotFound)?;

    let (extras, version) = module.on_chan_open_try_execute(
        msg.ordering,
        &msg.connection_hops_on_b,
        &msg.port_id_on_b,
        &chan_id_on_b,
        &Counterparty::new(msg.port_id_on_a.clone(), Some(msg.chan_id_on_a.clone())),
        &msg.version_supported_on_a,
    )?;

    let conn_id_on_b = msg.connection_hops_on_b[0].clone();

    // state changes
    {
        let chan_end_on_b = ChannelEnd::new(
            State::TryOpen,
            msg.ordering,
            Counterparty::new(msg.port_id_on_a.clone(), Some(msg.chan_id_on_a.clone())),
            msg.connection_hops_on_b.clone(),
            version.clone(),
        )?;

        let chan_end_path_on_b = ChannelEndPath::new(&msg.port_id_on_b, &chan_id_on_b);
        ctx_b.store_channel(&chan_end_path_on_b, chan_end_on_b)?;
        ctx_b.increase_channel_counter();

        // Initialize send, recv, and ack sequence numbers.
        let seq_send_path = SeqSendPath::new(&msg.port_id_on_b, &chan_id_on_b);
        ctx_b.store_next_sequence_send(&seq_send_path, 1.into())?;

        let seq_recv_path = SeqRecvPath::new(&msg.port_id_on_b, &chan_id_on_b);
        ctx_b.store_next_sequence_recv(&seq_recv_path, 1.into())?;

        let seq_ack_path = SeqAckPath::new(&msg.port_id_on_b, &chan_id_on_b);
        ctx_b.store_next_sequence_ack(&seq_ack_path, 1.into())?;
    }

    // emit events and logs
    {
        ctx_b.log_message(format!(
            "success: channel open try with channel identifier: {chan_id_on_b}"
        ));

        let core_event = IbcEvent::OpenTryChannel(OpenTry::new(
            msg.port_id_on_b.clone(),
            chan_id_on_b.clone(),
            msg.port_id_on_a.clone(),
            msg.chan_id_on_a.clone(),
            conn_id_on_b,
            version,
        ));
        ctx_b.emit_ibc_event(IbcEvent::Message(MessageEvent::Channel));
        ctx_b.emit_ibc_event(core_event);

        for module_event in extras.events {
            ctx_b.emit_ibc_event(IbcEvent::Module(module_event));
        }

        for log_message in extras.log {
            ctx_b.log_message(log_message);
        }
    }

    Ok(())
}

fn validate<Ctx>(ctx_b: &Ctx, msg: &MsgChannelOpenTry) -> Result<(), ContextError>
where
    Ctx: ValidationContext,
{
    ctx_b.validate_message_signer(&msg.signer)?;

    msg.verify_connection_hops_length()?;

    let conn_end_on_b = ctx_b.connection_end(&msg.connection_hops_on_b[0])?;

    conn_end_on_b.verify_state_matches(&ConnectionState::Open)?;

    let conn_version = conn_end_on_b.versions();

    conn_version[0].verify_feature_supported(msg.ordering.to_string())?;

    // Verify proofs
    {
        let client_id_on_b = conn_end_on_b.client_id();
        let client_state_of_a_on_b = ctx_b.client_state(client_id_on_b)?;

        client_state_of_a_on_b.confirm_not_frozen()?;
        client_state_of_a_on_b.validate_proof_height(msg.proof_height_on_a)?;

        let client_cons_state_path_on_b =
            ClientConsensusStatePath::new(client_id_on_b, &msg.proof_height_on_a);
        let consensus_state_of_a_on_b = ctx_b.consensus_state(&client_cons_state_path_on_b)?;
        let prefix_on_a = conn_end_on_b.counterparty().prefix();
        let port_id_on_a = msg.port_id_on_a.clone();
        let chan_id_on_a = msg.chan_id_on_a.clone();
        let conn_id_on_a = conn_end_on_b.counterparty().connection_id().ok_or(
            ChannelError::UndefinedConnectionCounterparty {
                connection_id: msg.connection_hops_on_b[0].clone(),
            },
        )?;

        let expected_chan_end_on_a = ChannelEnd::new(
            ChannelState::Init,
            msg.ordering,
            Counterparty::new(msg.port_id_on_b.clone(), None),
            vec![conn_id_on_a.clone()],
            msg.version_supported_on_a.clone(),
        )?;
        let chan_end_path_on_a = ChannelEndPath::new(&port_id_on_a, &chan_id_on_a);

        // Verify the proof for the channel state against the expected channel end.
        // A counterparty channel id of None in not possible, and is checked by validate_basic in msg.
        client_state_of_a_on_b
            .verify_membership(
                prefix_on_a,
                &msg.proof_chan_end_on_a,
                consensus_state_of_a_on_b.root(),
                Path::ChannelEnd(chan_end_path_on_a),
                expected_chan_end_on_a.encode_vec(),
            )
            .map_err(ChannelError::VerifyChannelFailed)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;
    use test_log::test;

    use crate::core::ics03_connection::connection::ConnectionEnd;
    use crate::core::ics03_connection::connection::Counterparty as ConnectionCounterparty;
    use crate::core::ics03_connection::connection::State as ConnectionState;
    use crate::core::ics03_connection::msgs::test_util::get_dummy_raw_counterparty;
    use crate::core::ics03_connection::version::get_compatible_versions;
    use crate::core::ics04_channel::msgs::chan_open_try::test_util::get_dummy_raw_msg_chan_open_try;
    use crate::core::ics04_channel::msgs::chan_open_try::MsgChannelOpenTry;
    use crate::core::ics24_host::identifier::{ClientId, ConnectionId};
    use crate::core::timestamp::ZERO_DURATION;
    use crate::Height;

    use crate::applications::transfer::MODULE_ID_STR;
    use crate::mock::client_state::client_type as mock_client_type;
    use crate::mock::context::MockContext;
    use crate::test_utils::DummyTransferModule;

    pub struct Fixture {
        pub ctx: MockContext,
        pub module_id: ModuleId,
        pub msg: MsgChannelOpenTry,
        pub client_id_on_b: ClientId,
        pub conn_id_on_b: ConnectionId,
        pub conn_end_on_b: ConnectionEnd,
        pub proof_height: u64,
    }

    #[fixture]
    fn fixture() -> Fixture {
        let proof_height = 10;
        let conn_id_on_b = ConnectionId::new(2);
        let client_id_on_b = ClientId::new(mock_client_type(), 45).unwrap();

        // This is the connection underlying the channel we're trying to open.
        let conn_end_on_b = ConnectionEnd::new(
            ConnectionState::Open,
            client_id_on_b.clone(),
            ConnectionCounterparty::try_from(get_dummy_raw_counterparty(Some(0))).unwrap(),
            get_compatible_versions(),
            ZERO_DURATION,
        )
        .unwrap();

        // We're going to test message processing against this message.
        // Note: we make the counterparty's channel_id `None`.
        let mut msg =
            MsgChannelOpenTry::try_from(get_dummy_raw_msg_chan_open_try(proof_height)).unwrap();

        let hops = vec![conn_id_on_b.clone()];
        msg.connection_hops_on_b = hops;

        let mut ctx = MockContext::default();
        let module = DummyTransferModule::new();
        let module_id: ModuleId = ModuleId::new(MODULE_ID_STR.to_string());
        ctx.add_route(module_id.clone(), module).unwrap();

        Fixture {
            ctx,
            module_id,
            msg,
            client_id_on_b,
            conn_id_on_b,
            conn_end_on_b,
            proof_height,
        }
    }

    #[rstest]
    fn chan_open_try_fail_no_connection(fixture: Fixture) {
        let Fixture { ctx, msg, .. } = fixture;

        let res = validate(&ctx, &msg);

        assert!(
            res.is_err(),
            "Validation fails because no connection exists in the context"
        )
    }

    #[rstest]
    fn chan_open_try_fail_no_client_state(fixture: Fixture) {
        let Fixture {
            ctx,
            msg,
            conn_id_on_b,
            conn_end_on_b,
            ..
        } = fixture;
        let ctx = ctx.with_connection(conn_id_on_b, conn_end_on_b);

        let res = validate(&ctx, &msg);

        assert!(
            res.is_err(),
            "Validation fails because the context has no client state"
        )
    }

    #[rstest]
    fn chan_open_try_validate_happy_path(fixture: Fixture) {
        let Fixture {
            ctx,
            msg,
            client_id_on_b,
            conn_id_on_b,
            conn_end_on_b,
            proof_height,
            ..
        } = fixture;

        let ctx = ctx
            .with_client(&client_id_on_b, Height::new(0, proof_height).unwrap())
            .with_connection(conn_id_on_b, conn_end_on_b);

        let res = validate(&ctx, &msg);

        assert!(res.is_ok(), "Validation success: happy path")
    }

    #[rstest]
    fn chan_open_try_execute_happy_path(fixture: Fixture) {
        let Fixture {
            ctx,
            module_id,
            msg,
            client_id_on_b,
            conn_id_on_b,
            conn_end_on_b,
            proof_height,
            ..
        } = fixture;

        let mut ctx = ctx
            .with_client(&client_id_on_b, Height::new(0, proof_height).unwrap())
            .with_connection(conn_id_on_b, conn_end_on_b);

        let res = chan_open_try_execute(&mut ctx, module_id, msg);

        assert!(res.is_ok(), "Execution success: happy path");

        assert_eq!(ctx.events.len(), 2);
        assert!(matches!(
            ctx.events[0],
            IbcEvent::Message(MessageEvent::Channel)
        ));
        assert!(matches!(ctx.events[1], IbcEvent::OpenTryChannel(_)));
    }
}
