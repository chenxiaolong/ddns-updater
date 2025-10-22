use futures_util::stream::{Stream, StreamExt};
use hickory_client::{
    ClientError,
    client::Client,
    proto::{
        ProtoError, ProtoErrorKind,
        dnssec::tsig::TSigner,
        op::{Edns, Message, MessageType, OpCode, Query, UpdateMessage},
        rr::{DNSClass, Name, RData, Record, RecordType},
        runtime::TokioRuntimeProvider,
        tcp::TcpClientStream,
        udp::UdpClientStream,
        xfer::{DnsHandle, DnsResponse},
    },
};

use std::{
    future::Future,
    net::{IpAddr, SocketAddr},
    pin::Pin,
    sync::Arc,
    task::{Context, Poll, ready},
    time::Duration,
};

use crate::config::Protocol;

/// Copied from unexported hickory_client::client::client::ClientResponse.
#[must_use = "futures do nothing unless polled"]
pub struct ClientResponse<R>(pub(crate) R)
where
    R: Stream<Item = Result<DnsResponse, ProtoError>> + Send + Unpin + 'static;

impl<R> Future for ClientResponse<R>
where
    R: Stream<Item = Result<DnsResponse, ProtoError>> + Send + Unpin + 'static,
{
    type Output = Result<DnsResponse, ClientError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(
            match ready!(self.0.poll_next_unpin(cx)) {
                Some(r) => r,
                None => Err(ProtoError::from(ProtoErrorKind::Timeout)),
            }
            .map_err(ClientError::from),
        )
    }
}

/// Create a new TCP or UDP client and spawn the background task for performing
/// I/O operations.
pub async fn new_client(
    server: SocketAddr,
    protocol: Protocol,
    timeout: Duration,
    signer: TSigner,
) -> Result<Client, ProtoError> {
    let provider = TokioRuntimeProvider::default();
    let signer = Arc::new(signer);

    match protocol {
        Protocol::Tcp => {
            let (stream, sender) = TcpClientStream::new(server, None, Some(timeout), provider);

            let (client, bg) = Client::with_timeout(stream, sender, timeout, Some(signer)).await?;

            tokio::spawn(bg);

            Ok(client)
        }
        Protocol::Udp => {
            let stream = UdpClientStream::builder(server, provider)
                .with_timeout(Some(timeout))
                .with_signer(Some(signer))
                .build();

            let (client, bg) = Client::connect(stream).await?;

            tokio::spawn(bg);

            Ok(client)
        }
    }
}

/// Build a single DNS request that removes existing A/AAAA records and replaces
/// them with the specified addresses. With a supported servers, this operation
/// should be atomic.
pub fn replace_addrs_message(
    zone_origin: &Name,
    name: &Name,
    ttl: u32,
    addrs: &[IpAddr],
) -> Message {
    let mut zone = Query::new();
    zone.set_name(zone_origin.clone())
        .set_query_class(DNSClass::IN)
        .set_query_type(RecordType::SOA);

    let mut message = Message::new();
    message
        .set_id(rand::random())
        .set_message_type(MessageType::Query)
        .set_op_code(OpCode::Update)
        .set_recursion_desired(false);
    message.add_zone(zone);

    for rtype in [RecordType::A, RecordType::AAAA] {
        let mut record = Record::update0(name.clone(), 0, rtype);
        record.set_dns_class(DNSClass::ANY);
        message.add_update(record);
    }

    for addr in addrs {
        let rdata = match addr {
            IpAddr::V4(ip) => RData::A((*ip).into()),
            IpAddr::V6(ip) => RData::AAAA((*ip).into()),
        };

        message.add_update(Record::from_rdata(name.clone(), ttl, rdata));
    }

    message
        .extensions_mut()
        .get_or_insert_with(Edns::new)
        .set_max_payload(hickory_client::proto::op::update_message::MAX_PAYLOAD_LEN)
        .set_version(0);

    message
}

pub fn send_message(
    client: &Client,
    message: Message,
) -> ClientResponse<<Client as DnsHandle>::Response> {
    ClientResponse(client.send(message))
}
