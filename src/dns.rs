use {
    std::{
        future::Future,
        net::{IpAddr, SocketAddr},
        pin::Pin,
        time::Duration,
    },
    trust_dns_client::{
        client::{AsyncClient, Client, SyncClient},
        error::ClientResult,
        op::{Edns, Message, MessageType, OpCode, Query, UpdateMessage},
        proto::{
            error::ProtoError,
            xfer::DnsExchangeSend,
        },
        rr::{
            dnssec::tsig::TSigner,
            DNSClass, Name, RData, Record, RecordType,
        },
        tcp::TcpClientConnection,
        udp::UdpClientConnection,
    },
    crate::config::Protocol,
};

/// `trust_dns_client::client::NewFutureObj` is not public
type NewFutureObj<H> = Pin<
    Box<
        dyn Future<
            Output = Result<
                (
                    H,
                    Box<dyn Future<Output = Result<(), ProtoError>> + 'static + Send + Unpin>,
                ),
                ProtoError,
            >,
        >
        + 'static
        + Send,
    >,
>;

/// Small wrapper to avoid callers needing to distinguish between TCP/UDP.
pub enum DnsClient {
    Tcp(SyncClient::<TcpClientConnection>),
    Udp(SyncClient::<UdpClientConnection>),
}

impl DnsClient {
    pub fn new(
        server: SocketAddr,
        protocol: Protocol,
        timeout: Duration,
        signer: TSigner,
    ) -> ClientResult<Self> {
        match protocol {
            Protocol::Tcp => {
                let client = TcpClientConnection::with_timeout(server, timeout)?;
                Ok(Self::Tcp(SyncClient::with_tsigner(client, signer)))
            }
            Protocol::Udp => {
                let client = UdpClientConnection::with_timeout(server, timeout)?;
                Ok(Self::Udp(SyncClient::with_tsigner(client, signer)))
            }
        }
    }
}

impl Client for DnsClient {
    type Response = DnsExchangeSend;
    type Handle = AsyncClient;

    fn new_future(&self) -> NewFutureObj<Self::Handle> {
        match self {
            Self::Tcp(c) => c.new_future(),
            Self::Udp(c) => c.new_future(),
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
    // trust_dns_client::client::AsyncClient::MAX_PAYLOAD_LEN is not public
    const MAX_PAYLOAD_LEN: u16 = 1232;

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
        let mut record = Record::with(name.clone(), rtype, 0);
        record.set_dns_class(DNSClass::ANY);
        message.add_update(record);
    }

    for addr in addrs {
        let rdata = match addr {
            IpAddr::V4(ip) => RData::A(*ip),
            IpAddr::V6(ip) => RData::AAAA(*ip),
        };

        message.add_update(Record::from_rdata(name.clone(), ttl, rdata));
    }

    message
        .extensions_mut()
        .get_or_insert_with(Edns::new)
        .set_max_payload(MAX_PAYLOAD_LEN)
        .set_version(0);

    message
}
