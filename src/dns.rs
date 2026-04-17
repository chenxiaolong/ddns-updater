use futures_util::stream::{Stream, StreamExt};
use hickory_net::{
    NetError,
    client::Client,
    proto::{
        op::{
            DEFAULT_MAX_PAYLOAD_LEN, DnsRequest, DnsResponse, Edns, Message, OpCode, Query,
            UpdateMessage,
        },
        rr::{DNSClass, Name, RData, Record, RecordType, TSigner},
    },
    runtime::TokioRuntimeProvider,
    tcp::TcpClientStream,
    udp::UdpClientStream,
    xfer::{DnsHandle, DnsMultiplexer},
};

use std::{
    future::Future,
    net::{IpAddr, SocketAddr},
    pin::Pin,
    task::{Context, Poll, ready},
    time::Duration,
};

use crate::config::Protocol;

/// Copied from unexported hickory_net::client::ClientResponse.
#[must_use = "futures do nothing unless polled"]
pub struct ClientResponse<R>(pub(crate) R)
where
    R: Stream<Item = Result<DnsResponse, NetError>> + Send + Unpin + 'static;

impl<R> Future for ClientResponse<R>
where
    R: Stream<Item = Result<DnsResponse, NetError>> + Send + Unpin + 'static,
{
    type Output = Result<DnsResponse, NetError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(match ready!(self.0.poll_next_unpin(cx)) {
            Some(r) => r,
            None => Err(NetError::Timeout),
        })
    }
}

/// Create a new TCP or UDP client and spawn the background task for performing
/// I/O operations.
pub async fn new_client(
    server: SocketAddr,
    protocol: Protocol,
    timeout: Duration,
    signer: TSigner,
) -> Result<Client<TokioRuntimeProvider>, NetError> {
    let provider = TokioRuntimeProvider::default();

    match protocol {
        Protocol::Tcp => {
            let (stream, sender) = TcpClientStream::new(server, None, Some(timeout), provider);
            let stream = stream.await?;
            let multiplexer = DnsMultiplexer::new(stream, sender)
                .with_timeout(timeout)
                .with_signer(signer);

            let (client, bg) = Client::from_sender(multiplexer);

            tokio::spawn(bg);

            Ok(client)
        }
        Protocol::Udp => {
            let stream = UdpClientStream::builder(server, provider)
                .with_timeout(Some(timeout))
                .with_signer(Some(signer))
                .build();

            let (client, bg) = Client::from_sender(stream);

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

    let mut message = Message::query();
    message.metadata.op_code = OpCode::Update;
    message.metadata.recursion_desired = false;
    message.add_zone(zone);

    for rtype in [RecordType::A, RecordType::AAAA] {
        let mut record = Record::update0(name.clone(), 0, rtype);
        record.dns_class = DNSClass::ANY;
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
        .edns
        .get_or_insert_with(Edns::new)
        .set_max_payload(DEFAULT_MAX_PAYLOAD_LEN)
        .set_version(0);

    message
}

pub fn send_message(
    client: &Client<TokioRuntimeProvider>,
    message: Message,
) -> ClientResponse<<Client<TokioRuntimeProvider> as DnsHandle>::Response> {
    ClientResponse(client.send(DnsRequest::from(message)))
}
