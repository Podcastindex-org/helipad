#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SendPaymentRequest {
    /// The identity pubkey of the payment recipient
    #[prost(bytes="vec", tag="1")]
    pub dest: ::prost::alloc::vec::Vec<u8>,
    ///
    ///Number of satoshis to send.
    ///
    ///The fields amt and amt_msat are mutually exclusive.
    #[prost(int64, tag="2")]
    pub amt: i64,
    /// The hash to use within the payment's HTLC
    #[prost(bytes="vec", tag="3")]
    pub payment_hash: ::prost::alloc::vec::Vec<u8>,
    ///
    ///The CLTV delta from the current height that should be used to set the
    ///timelock for the final hop.
    #[prost(int32, tag="4")]
    pub final_cltv_delta: i32,
    ///
    ///A bare-bones invoice for a payment within the Lightning Network.  With the
    ///details of the invoice, the sender has all the data necessary to send a
    ///payment to the recipient. The amount in the payment request may be zero. In
    ///that case it is required to set the amt field as well. If no payment request
    ///is specified, the following fields are required: dest, amt and payment_hash.
    #[prost(string, tag="5")]
    pub payment_request: ::prost::alloc::string::String,
    ///
    ///An optional limit, expressed in seconds, on the time to wait before
    ///attempting the first HTLC. Once HTLCs are in flight, the payment will
    ///not be aborted until the HTLCs are either settled or failed. If the field
    ///is not set or is explicitly set to zero, the default value of 60 seconds
    ///will be applied.
    #[prost(int32, tag="6")]
    pub timeout_seconds: i32,
    ///
    ///The maximum number of satoshis that will be paid as a fee of the payment.
    ///If this field is left to the default value of 0, only zero-fee routes will
    ///be considered. This usually means single hop routes connecting directly to
    ///the destination. To send the payment without a fee limit, use max int here.
    ///
    ///The fields fee_limit_sat and fee_limit_msat are mutually exclusive.
    #[prost(int64, tag="7")]
    pub fee_limit_sat: i64,
    ///
    ///Deprecated, use outgoing_chan_ids. The channel id of the channel that must
    ///be taken to the first hop. If zero, any channel may be used (unless
    ///outgoing_chan_ids are set).
    #[deprecated]
    #[prost(uint64, tag="8")]
    pub outgoing_chan_id: u64,
    ///
    ///An optional maximum total time lock for the route. This should not
    ///exceed lnd's `--max-cltv-expiry` setting. If zero, then the value of
    ///`--max-cltv-expiry` is enforced.
    #[prost(int32, tag="9")]
    pub cltv_limit: i32,
    ///
    ///Optional route hints to reach the destination through private channels.
    #[prost(message, repeated, tag="10")]
    pub route_hints: ::prost::alloc::vec::Vec<super::lnrpc::RouteHint>,
    ///
    ///An optional field that can be used to pass an arbitrary set of TLV records
    ///to a peer which understands the new records. This can be used to pass
    ///application specific data during the payment attempt. Record types are
    ///required to be in the custom range >= 65536. When using REST, the values
    ///must be encoded as base64.
    #[prost(map="uint64, bytes", tag="11")]
    pub dest_custom_records: ::std::collections::HashMap<u64, ::prost::alloc::vec::Vec<u8>>,
    ///
    ///Number of millisatoshis to send.
    ///
    ///The fields amt and amt_msat are mutually exclusive.
    #[prost(int64, tag="12")]
    pub amt_msat: i64,
    ///
    ///The maximum number of millisatoshis that will be paid as a fee of the
    ///payment. If this field is left to the default value of 0, only zero-fee
    ///routes will be considered. This usually means single hop routes connecting
    ///directly to the destination. To send the payment without a fee limit, use
    ///max int here.
    ///
    ///The fields fee_limit_sat and fee_limit_msat are mutually exclusive.
    #[prost(int64, tag="13")]
    pub fee_limit_msat: i64,
    ///
    ///The pubkey of the last hop of the route. If empty, any hop may be used.
    #[prost(bytes="vec", tag="14")]
    pub last_hop_pubkey: ::prost::alloc::vec::Vec<u8>,
    /// If set, circular payments to self are permitted.
    #[prost(bool, tag="15")]
    pub allow_self_payment: bool,
    ///
    ///Features assumed to be supported by the final node. All transitive feature
    ///dependencies must also be set properly. For a given feature bit pair, either
    ///optional or remote may be set, but not both. If this field is nil or empty,
    ///the router will try to load destination features from the graph as a
    ///fallback.
    #[prost(enumeration="super::lnrpc::FeatureBit", repeated, tag="16")]
    pub dest_features: ::prost::alloc::vec::Vec<i32>,
    ///
    ///The maximum number of partial payments that may be use to complete the full
    ///amount.
    #[prost(uint32, tag="17")]
    pub max_parts: u32,
    ///
    ///If set, only the final payment update is streamed back. Intermediate updates
    ///that show which htlcs are still in flight are suppressed.
    #[prost(bool, tag="18")]
    pub no_inflight_updates: bool,
    ///
    ///The channel ids of the channels are allowed for the first hop. If empty,
    ///any channel may be used.
    #[prost(uint64, repeated, tag="19")]
    pub outgoing_chan_ids: ::prost::alloc::vec::Vec<u64>,
    ///
    ///An optional payment addr to be included within the last hop of the route.
    ///This is also called payment secret in specifications (e.g. BOLT 11).
    #[prost(bytes="vec", tag="20")]
    pub payment_addr: ::prost::alloc::vec::Vec<u8>,
    ///
    ///The largest payment split that should be attempted when making a payment if
    ///splitting is necessary. Setting this value will effectively cause lnd to
    ///split more aggressively, vs only when it thinks it needs to. Note that this
    ///value is in milli-satoshis.
    #[prost(uint64, tag="21")]
    pub max_shard_size_msat: u64,
    ///
    ///If set, an AMP-payment will be attempted.
    #[prost(bool, tag="22")]
    pub amp: bool,
    ///
    ///The time preference for this payment. Set to -1 to optimize for fees
    ///only, to 1 to optimize for reliability only or a value inbetween for a mix.
    #[prost(double, tag="23")]
    pub time_pref: f64,
    ///
    ///If set, the payment loop can be interrupted by manually canceling the
    ///payment context, even before the payment timeout is reached. Note that the
    ///payment may still succeed after cancellation, as in-flight attempts can
    ///still settle afterwards. Canceling will only prevent further attempts from
    ///being sent.
    #[prost(bool, tag="24")]
    pub cancelable: bool,
    ///
    ///An optional field that can be used to pass an arbitrary set of TLV records
    ///to the first hop peer of this payment. This can be used to pass application
    ///specific data during the payment attempt. Record types are required to be in
    ///the custom range >= 65536. When using REST, the values must be encoded as
    ///base64.
    #[prost(map="uint64, bytes", tag="25")]
    pub first_hop_custom_records: ::std::collections::HashMap<u64, ::prost::alloc::vec::Vec<u8>>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct TrackPaymentRequest {
    /// The hash of the payment to look up.
    #[prost(bytes="vec", tag="1")]
    pub payment_hash: ::prost::alloc::vec::Vec<u8>,
    ///
    ///If set, only the final payment update is streamed back. Intermediate updates
    ///that show which htlcs are still in flight are suppressed.
    #[prost(bool, tag="2")]
    pub no_inflight_updates: bool,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct TrackPaymentsRequest {
    ///
    ///If set, only the final payment updates are streamed back. Intermediate
    ///updates that show which htlcs are still in flight are suppressed.
    #[prost(bool, tag="1")]
    pub no_inflight_updates: bool,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RouteFeeRequest {
    ///
    ///The destination one wishes to obtain a routing fee quote to. If set, this
    ///parameter requires the amt_sat parameter also to be set. This parameter
    ///combination triggers a graph based routing fee estimation as opposed to a
    ///payment probe based estimate in case a payment request is provided. The
    ///graph based estimation is an algorithm that is executed on the in memory
    ///graph. Hence its runtime is significantly shorter than a payment probe
    ///estimation that sends out actual payments to the network.
    #[prost(bytes="vec", tag="1")]
    pub dest: ::prost::alloc::vec::Vec<u8>,
    ///
    ///The amount one wishes to send to the target destination. It is only to be
    ///used in combination with the dest parameter.
    #[prost(int64, tag="2")]
    pub amt_sat: i64,
    ///
    ///A payment request of the target node that the route fee request is intended
    ///for. Its parameters are input to probe payments that estimate routing fees.
    ///The timeout parameter can be specified to set a maximum time on the probing
    ///attempt. Cannot be used in combination with dest and amt_sat.
    #[prost(string, tag="3")]
    pub payment_request: ::prost::alloc::string::String,
    ///
    ///A user preference of how long a probe payment should maximally be allowed to
    ///take, denoted in seconds. The probing payment loop is aborted if this
    ///timeout is reached. Note that the probing process itself can take longer
    ///than the timeout if the HTLC becomes delayed or stuck. Canceling the context
    ///of this call will not cancel the payment loop, the duration is only
    ///controlled by the timeout parameter.
    #[prost(uint32, tag="4")]
    pub timeout: u32,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RouteFeeResponse {
    ///
    ///A lower bound of the estimated fee to the target destination within the
    ///network, expressed in milli-satoshis.
    #[prost(int64, tag="1")]
    pub routing_fee_msat: i64,
    ///
    ///An estimate of the worst case time delay that can occur. Note that callers
    ///will still need to factor in the final CLTV delta of the last hop into this
    ///value.
    #[prost(int64, tag="2")]
    pub time_lock_delay: i64,
    ///
    ///An indication whether a probing payment succeeded or whether and why it
    ///failed. FAILURE_REASON_NONE indicates success.
    #[prost(enumeration="super::lnrpc::PaymentFailureReason", tag="5")]
    pub failure_reason: i32,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SendToRouteRequest {
    /// The payment hash to use for the HTLC.
    #[prost(bytes="vec", tag="1")]
    pub payment_hash: ::prost::alloc::vec::Vec<u8>,
    /// Route that should be used to attempt to complete the payment.
    #[prost(message, optional, tag="2")]
    pub route: ::core::option::Option<super::lnrpc::Route>,
    ///
    ///Whether the payment should be marked as failed when a temporary error is
    ///returned from the given route. Set it to true so the payment won't be
    ///failed unless a terminal error is occurred, such as payment timeout, no
    ///routes, incorrect payment details, or insufficient funds.
    #[prost(bool, tag="3")]
    pub skip_temp_err: bool,
    ///
    ///An optional field that can be used to pass an arbitrary set of TLV records
    ///to the first hop peer of this payment. This can be used to pass application
    ///specific data during the payment attempt. Record types are required to be in
    ///the custom range >= 65536. When using REST, the values must be encoded as
    ///base64.
    #[prost(map="uint64, bytes", tag="4")]
    pub first_hop_custom_records: ::std::collections::HashMap<u64, ::prost::alloc::vec::Vec<u8>>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SendToRouteResponse {
    /// The preimage obtained by making the payment.
    #[prost(bytes="vec", tag="1")]
    pub preimage: ::prost::alloc::vec::Vec<u8>,
    /// The failure message in case the payment failed.
    #[prost(message, optional, tag="2")]
    pub failure: ::core::option::Option<super::lnrpc::Failure>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ResetMissionControlRequest {
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ResetMissionControlResponse {
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct QueryMissionControlRequest {
}
/// QueryMissionControlResponse contains mission control state.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct QueryMissionControlResponse {
    /// Node pair-level mission control state.
    #[prost(message, repeated, tag="2")]
    pub pairs: ::prost::alloc::vec::Vec<PairHistory>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct XImportMissionControlRequest {
    /// Node pair-level mission control state to be imported.
    #[prost(message, repeated, tag="1")]
    pub pairs: ::prost::alloc::vec::Vec<PairHistory>,
    /// Whether to force override MC pair history. Note that even with force
    /// override the failure pair is imported before the success pair and both
    /// still clamp existing failure/success amounts.
    #[prost(bool, tag="2")]
    pub force: bool,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct XImportMissionControlResponse {
}
/// PairHistory contains the mission control state for a particular node pair.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PairHistory {
    /// The source node pubkey of the pair.
    #[prost(bytes="vec", tag="1")]
    pub node_from: ::prost::alloc::vec::Vec<u8>,
    /// The destination node pubkey of the pair.
    #[prost(bytes="vec", tag="2")]
    pub node_to: ::prost::alloc::vec::Vec<u8>,
    #[prost(message, optional, tag="7")]
    pub history: ::core::option::Option<PairData>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PairData {
    /// Time of last failure.
    #[prost(int64, tag="1")]
    pub fail_time: i64,
    ///
    ///Lowest amount that failed to forward rounded to whole sats. This may be
    ///set to zero if the failure is independent of amount.
    #[prost(int64, tag="2")]
    pub fail_amt_sat: i64,
    ///
    ///Lowest amount that failed to forward in millisats. This may be
    ///set to zero if the failure is independent of amount.
    #[prost(int64, tag="4")]
    pub fail_amt_msat: i64,
    /// Time of last success.
    #[prost(int64, tag="5")]
    pub success_time: i64,
    /// Highest amount that we could successfully forward rounded to whole sats.
    #[prost(int64, tag="6")]
    pub success_amt_sat: i64,
    /// Highest amount that we could successfully forward in millisats.
    #[prost(int64, tag="7")]
    pub success_amt_msat: i64,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetMissionControlConfigRequest {
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetMissionControlConfigResponse {
    ///
    ///Mission control's currently active config.
    #[prost(message, optional, tag="1")]
    pub config: ::core::option::Option<MissionControlConfig>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SetMissionControlConfigRequest {
    ///
    ///The config to set for mission control. Note that all values *must* be set,
    ///because the full config will be applied.
    #[prost(message, optional, tag="1")]
    pub config: ::core::option::Option<MissionControlConfig>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SetMissionControlConfigResponse {
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MissionControlConfig {
    ///
    ///Deprecated, use AprioriParameters. The amount of time mission control will
    ///take to restore a penalized node or channel back to 50% success probability,
    ///expressed in seconds. Setting this value to a higher value will penalize
    ///failures for longer, making mission control less likely to route through
    ///nodes and channels that we have previously recorded failures for.
    #[deprecated]
    #[prost(uint64, tag="1")]
    pub half_life_seconds: u64,
    ///
    ///Deprecated, use AprioriParameters. The probability of success mission
    ///control should assign to hop in a route where it has no other information
    ///available. Higher values will make mission control more willing to try hops
    ///that we have no information about, lower values will discourage trying these
    ///hops.
    #[deprecated]
    #[prost(float, tag="2")]
    pub hop_probability: f32,
    ///
    ///Deprecated, use AprioriParameters. The importance that mission control
    ///should place on historical results, expressed as a value in [0;1]. Setting
    ///this value to 1 will ignore all historical payments and just use the hop
    ///probability to assess the probability of success for each hop. A zero value
    ///ignores hop probability completely and relies entirely on historical
    ///results, unless none are available.
    #[deprecated]
    #[prost(float, tag="3")]
    pub weight: f32,
    ///
    ///The maximum number of payment results that mission control will store.
    #[prost(uint32, tag="4")]
    pub maximum_payment_results: u32,
    ///
    ///The minimum time that must have passed since the previously recorded failure
    ///before we raise the failure amount.
    #[prost(uint64, tag="5")]
    pub minimum_failure_relax_interval: u64,
    ///
    ///ProbabilityModel defines which probability estimator should be used in
    ///pathfinding. Note that the bimodal estimator is experimental.
    #[prost(enumeration="mission_control_config::ProbabilityModel", tag="6")]
    pub model: i32,
    ///
    ///EstimatorConfig is populated dependent on the estimator type.
    #[prost(oneof="mission_control_config::EstimatorConfig", tags="7, 8")]
    pub estimator_config: ::core::option::Option<mission_control_config::EstimatorConfig>,
}
/// Nested message and enum types in `MissionControlConfig`.
pub mod mission_control_config {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum ProbabilityModel {
        Apriori = 0,
        Bimodal = 1,
    }
    ///
    ///EstimatorConfig is populated dependent on the estimator type.
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum EstimatorConfig {
        #[prost(message, tag="7")]
        Apriori(super::AprioriParameters),
        #[prost(message, tag="8")]
        Bimodal(super::BimodalParameters),
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BimodalParameters {
    ///
    ///NodeWeight defines how strongly other previous forwardings on channels of a
    ///router should be taken into account when computing a channel's probability
    ///to route. The allowed values are in the range [0, 1], where a value of 0
    ///means that only direct information about a channel is taken into account.
    #[prost(double, tag="1")]
    pub node_weight: f64,
    ///
    ///ScaleMsat describes the scale over which channels statistically have some
    ///liquidity left. The value determines how quickly the bimodal distribution
    ///drops off from the edges of a channel. A larger value (compared to typical
    ///channel capacities) means that the drop off is slow and that channel
    ///balances are distributed more uniformly. A small value leads to the
    ///assumption of very unbalanced channels.
    #[prost(uint64, tag="2")]
    pub scale_msat: u64,
    ///
    ///DecayTime describes the information decay of knowledge about previous
    ///successes and failures in channels. The smaller the decay time, the quicker
    ///we forget about past forwardings.
    #[prost(uint64, tag="3")]
    pub decay_time: u64,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct AprioriParameters {
    ///
    ///The amount of time mission control will take to restore a penalized node
    ///or channel back to 50% success probability, expressed in seconds. Setting
    ///this value to a higher value will penalize failures for longer, making
    ///mission control less likely to route through nodes and channels that we
    ///have previously recorded failures for.
    #[prost(uint64, tag="1")]
    pub half_life_seconds: u64,
    ///
    ///The probability of success mission control should assign to hop in a route
    ///where it has no other information available. Higher values will make mission
    ///control more willing to try hops that we have no information about, lower
    ///values will discourage trying these hops.
    #[prost(double, tag="2")]
    pub hop_probability: f64,
    ///
    ///The importance that mission control should place on historical results,
    ///expressed as a value in [0;1]. Setting this value to 1 will ignore all
    ///historical payments and just use the hop probability to assess the
    ///probability of success for each hop. A zero value ignores hop probability
    ///completely and relies entirely on historical results, unless none are
    ///available.
    #[prost(double, tag="3")]
    pub weight: f64,
    ///
    ///The fraction of a channel's capacity that we consider to have liquidity. For
    ///amounts that come close to or exceed the fraction, an additional penalty is
    ///applied. A value of 1.0 disables the capacity factor. Allowed values are in
    ///[0.75, 1.0].
    #[prost(double, tag="4")]
    pub capacity_fraction: f64,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct QueryProbabilityRequest {
    /// The source node pubkey of the pair.
    #[prost(bytes="vec", tag="1")]
    pub from_node: ::prost::alloc::vec::Vec<u8>,
    /// The destination node pubkey of the pair.
    #[prost(bytes="vec", tag="2")]
    pub to_node: ::prost::alloc::vec::Vec<u8>,
    /// The amount for which to calculate a probability.
    #[prost(int64, tag="3")]
    pub amt_msat: i64,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct QueryProbabilityResponse {
    /// The success probability for the requested pair.
    #[prost(double, tag="1")]
    pub probability: f64,
    /// The historical data for the requested pair.
    #[prost(message, optional, tag="2")]
    pub history: ::core::option::Option<PairData>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BuildRouteRequest {
    ///
    ///The amount to send expressed in msat. If set to zero, the minimum routable
    ///amount is used.
    #[prost(int64, tag="1")]
    pub amt_msat: i64,
    ///
    ///CLTV delta from the current height that should be used for the timelock
    ///of the final hop
    #[prost(int32, tag="2")]
    pub final_cltv_delta: i32,
    ///
    ///The channel id of the channel that must be taken to the first hop. If zero,
    ///any channel may be used.
    #[prost(uint64, tag="3")]
    pub outgoing_chan_id: u64,
    ///
    ///A list of hops that defines the route. This does not include the source hop
    ///pubkey.
    #[prost(bytes="vec", repeated, tag="4")]
    pub hop_pubkeys: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
    ///
    ///An optional payment addr to be included within the last hop of the route.
    ///This is also called payment secret in specifications (e.g. BOLT 11).
    #[prost(bytes="vec", tag="5")]
    pub payment_addr: ::prost::alloc::vec::Vec<u8>,
    ///
    ///An optional field that can be used to pass an arbitrary set of TLV records
    ///to the first hop peer of this payment. This can be used to pass application
    ///specific data during the payment attempt. Record types are required to be in
    ///the custom range >= 65536. When using REST, the values must be encoded as
    ///base64.
    #[prost(map="uint64, bytes", tag="6")]
    pub first_hop_custom_records: ::std::collections::HashMap<u64, ::prost::alloc::vec::Vec<u8>>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BuildRouteResponse {
    ///
    ///Fully specified route that can be used to execute the payment.
    #[prost(message, optional, tag="1")]
    pub route: ::core::option::Option<super::lnrpc::Route>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SubscribeHtlcEventsRequest {
}
///
///HtlcEvent contains the htlc event that was processed. These are served on a
///best-effort basis; events are not persisted, delivery is not guaranteed
///(in the event of a crash in the switch, forward events may be lost) and
///some events may be replayed upon restart. Events consumed from this package
///should be de-duplicated by the htlc's unique combination of incoming and
///outgoing channel id and htlc id. [EXPERIMENTAL]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct HtlcEvent {
    ///
    ///The short channel id that the incoming htlc arrived at our node on. This
    ///value is zero for sends.
    #[prost(uint64, tag="1")]
    pub incoming_channel_id: u64,
    ///
    ///The short channel id that the outgoing htlc left our node on. This value
    ///is zero for receives.
    #[prost(uint64, tag="2")]
    pub outgoing_channel_id: u64,
    ///
    ///Incoming id is the index of the incoming htlc in the incoming channel.
    ///This value is zero for sends.
    #[prost(uint64, tag="3")]
    pub incoming_htlc_id: u64,
    ///
    ///Outgoing id is the index of the outgoing htlc in the outgoing channel.
    ///This value is zero for receives.
    #[prost(uint64, tag="4")]
    pub outgoing_htlc_id: u64,
    ///
    ///The time in unix nanoseconds that the event occurred.
    #[prost(uint64, tag="5")]
    pub timestamp_ns: u64,
    ///
    ///The event type indicates whether the htlc was part of a send, receive or
    ///forward.
    #[prost(enumeration="htlc_event::EventType", tag="6")]
    pub event_type: i32,
    #[prost(oneof="htlc_event::Event", tags="7, 8, 9, 10, 11, 12")]
    pub event: ::core::option::Option<htlc_event::Event>,
}
/// Nested message and enum types in `HtlcEvent`.
pub mod htlc_event {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum EventType {
        Unknown = 0,
        Send = 1,
        Receive = 2,
        Forward = 3,
    }
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Event {
        #[prost(message, tag="7")]
        ForwardEvent(super::ForwardEvent),
        #[prost(message, tag="8")]
        ForwardFailEvent(super::ForwardFailEvent),
        #[prost(message, tag="9")]
        SettleEvent(super::SettleEvent),
        #[prost(message, tag="10")]
        LinkFailEvent(super::LinkFailEvent),
        #[prost(message, tag="11")]
        SubscribedEvent(super::SubscribedEvent),
        #[prost(message, tag="12")]
        FinalHtlcEvent(super::FinalHtlcEvent),
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct HtlcInfo {
    /// The timelock on the incoming htlc.
    #[prost(uint32, tag="1")]
    pub incoming_timelock: u32,
    /// The timelock on the outgoing htlc.
    #[prost(uint32, tag="2")]
    pub outgoing_timelock: u32,
    /// The amount of the incoming htlc.
    #[prost(uint64, tag="3")]
    pub incoming_amt_msat: u64,
    /// The amount of the outgoing htlc.
    #[prost(uint64, tag="4")]
    pub outgoing_amt_msat: u64,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ForwardEvent {
    /// Info contains details about the htlc that was forwarded.
    #[prost(message, optional, tag="1")]
    pub info: ::core::option::Option<HtlcInfo>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ForwardFailEvent {
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SettleEvent {
    /// The revealed preimage.
    #[prost(bytes="vec", tag="1")]
    pub preimage: ::prost::alloc::vec::Vec<u8>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct FinalHtlcEvent {
    #[prost(bool, tag="1")]
    pub settled: bool,
    #[prost(bool, tag="2")]
    pub offchain: bool,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SubscribedEvent {
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct LinkFailEvent {
    /// Info contains details about the htlc that we failed.
    #[prost(message, optional, tag="1")]
    pub info: ::core::option::Option<HtlcInfo>,
    /// FailureCode is the BOLT error code for the failure.
    #[prost(enumeration="super::lnrpc::failure::FailureCode", tag="2")]
    pub wire_failure: i32,
    ///
    ///FailureDetail provides additional information about the reason for the
    ///failure. This detail enriches the information provided by the wire message
    ///and may be 'no detail' if the wire message requires no additional metadata.
    #[prost(enumeration="FailureDetail", tag="3")]
    pub failure_detail: i32,
    /// A string representation of the link failure.
    #[prost(string, tag="4")]
    pub failure_string: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PaymentStatus {
    /// Current state the payment is in.
    #[prost(enumeration="PaymentState", tag="1")]
    pub state: i32,
    ///
    ///The pre-image of the payment when state is SUCCEEDED.
    #[prost(bytes="vec", tag="2")]
    pub preimage: ::prost::alloc::vec::Vec<u8>,
    ///
    ///The HTLCs made in attempt to settle the payment [EXPERIMENTAL].
    #[prost(message, repeated, tag="4")]
    pub htlcs: ::prost::alloc::vec::Vec<super::lnrpc::HtlcAttempt>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CircuitKey {
    //// The id of the channel that the is part of this circuit.
    #[prost(uint64, tag="1")]
    pub chan_id: u64,
    //// The index of the incoming htlc in the incoming channel.
    #[prost(uint64, tag="2")]
    pub htlc_id: u64,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ForwardHtlcInterceptRequest {
    ///
    ///The key of this forwarded htlc. It defines the incoming channel id and
    ///the index in this channel.
    #[prost(message, optional, tag="1")]
    pub incoming_circuit_key: ::core::option::Option<CircuitKey>,
    /// The incoming htlc amount.
    #[prost(uint64, tag="5")]
    pub incoming_amount_msat: u64,
    /// The incoming htlc expiry.
    #[prost(uint32, tag="6")]
    pub incoming_expiry: u32,
    ///
    ///The htlc payment hash. This value is not guaranteed to be unique per
    ///request.
    #[prost(bytes="vec", tag="2")]
    pub payment_hash: ::prost::alloc::vec::Vec<u8>,
    /// The requested outgoing channel id for this forwarded htlc. Because of
    /// non-strict forwarding, this isn't necessarily the channel over which the
    /// packet will be forwarded eventually. A different channel to the same peer
    /// may be selected as well.
    #[prost(uint64, tag="7")]
    pub outgoing_requested_chan_id: u64,
    /// The outgoing htlc amount.
    #[prost(uint64, tag="3")]
    pub outgoing_amount_msat: u64,
    /// The outgoing htlc expiry.
    #[prost(uint32, tag="4")]
    pub outgoing_expiry: u32,
    /// Any custom records that were present in the payload.
    #[prost(map="uint64, bytes", tag="8")]
    pub custom_records: ::std::collections::HashMap<u64, ::prost::alloc::vec::Vec<u8>>,
    /// The onion blob for the next hop
    #[prost(bytes="vec", tag="9")]
    pub onion_blob: ::prost::alloc::vec::Vec<u8>,
    /// The block height at which this htlc will be auto-failed to prevent the
    /// channel from force-closing.
    #[prost(int32, tag="10")]
    pub auto_fail_height: i32,
    /// The custom records of the peer's incoming p2p wire message.
    #[prost(map="uint64, bytes", tag="11")]
    pub in_wire_custom_records: ::std::collections::HashMap<u64, ::prost::alloc::vec::Vec<u8>>,
}
///*
///ForwardHtlcInterceptResponse enables the caller to resolve a previously hold
///forward. The caller can choose either to:
///- `Resume`: Execute the default behavior (usually forward).
///- `ResumeModified`: Execute the default behavior (usually forward) with HTLC
///field modifications.
///- `Reject`: Fail the htlc backwards.
///- `Settle`: Settle this htlc with a given preimage.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ForwardHtlcInterceptResponse {
    ///*
    ///The key of this forwarded htlc. It defines the incoming channel id and
    ///the index in this channel.
    #[prost(message, optional, tag="1")]
    pub incoming_circuit_key: ::core::option::Option<CircuitKey>,
    /// The resolve action for this intercepted htlc.
    #[prost(enumeration="ResolveHoldForwardAction", tag="2")]
    pub action: i32,
    /// The preimage in case the resolve action is Settle.
    #[prost(bytes="vec", tag="3")]
    pub preimage: ::prost::alloc::vec::Vec<u8>,
    /// Encrypted failure message in case the resolve action is Fail.
    ///
    /// If failure_message is specified, the failure_code field must be set
    /// to zero.
    #[prost(bytes="vec", tag="4")]
    pub failure_message: ::prost::alloc::vec::Vec<u8>,
    /// Return the specified failure code in case the resolve action is Fail. The
    /// message data fields are populated automatically.
    ///
    /// If a non-zero failure_code is specified, failure_message must not be set.
    ///
    /// For backwards-compatibility reasons, TEMPORARY_CHANNEL_FAILURE is the
    /// default value for this field.
    #[prost(enumeration="super::lnrpc::failure::FailureCode", tag="5")]
    pub failure_code: i32,
    /// The amount that was set on the p2p wire message of the incoming HTLC.
    /// This field is ignored if the action is not RESUME_MODIFIED or the amount
    /// is zero.
    #[prost(uint64, tag="6")]
    pub in_amount_msat: u64,
    /// The amount to set on the p2p wire message of the resumed HTLC. This field
    /// is ignored if the action is not RESUME_MODIFIED or the amount is zero.
    #[prost(uint64, tag="7")]
    pub out_amount_msat: u64,
    /// Any custom records that should be set on the p2p wire message message of
    /// the resumed HTLC. This field is ignored if the action is not
    /// RESUME_MODIFIED.
    ///
    /// This map will merge with the existing set of custom records (if any),
    /// replacing any conflicting types. Note that there currently is no support
    /// for deleting existing custom records (they can only be replaced).
    #[prost(map="uint64, bytes", tag="8")]
    pub out_wire_custom_records: ::std::collections::HashMap<u64, ::prost::alloc::vec::Vec<u8>>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct UpdateChanStatusRequest {
    #[prost(message, optional, tag="1")]
    pub chan_point: ::core::option::Option<super::lnrpc::ChannelPoint>,
    #[prost(enumeration="ChanStatusAction", tag="2")]
    pub action: i32,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct UpdateChanStatusResponse {
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct AddAliasesRequest {
    #[prost(message, repeated, tag="1")]
    pub alias_maps: ::prost::alloc::vec::Vec<super::lnrpc::AliasMap>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct AddAliasesResponse {
    #[prost(message, repeated, tag="1")]
    pub alias_maps: ::prost::alloc::vec::Vec<super::lnrpc::AliasMap>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DeleteAliasesRequest {
    #[prost(message, repeated, tag="1")]
    pub alias_maps: ::prost::alloc::vec::Vec<super::lnrpc::AliasMap>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DeleteAliasesResponse {
    #[prost(message, repeated, tag="1")]
    pub alias_maps: ::prost::alloc::vec::Vec<super::lnrpc::AliasMap>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct FindBaseAliasRequest {
    /// The alias we want to look up the base scid for.
    #[prost(uint64, tag="1")]
    pub alias: u64,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct FindBaseAliasResponse {
    /// The base scid that resulted from the base scid look up.
    #[prost(uint64, tag="1")]
    pub base: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum FailureDetail {
    Unknown = 0,
    NoDetail = 1,
    OnionDecode = 2,
    LinkNotEligible = 3,
    OnChainTimeout = 4,
    HtlcExceedsMax = 5,
    InsufficientBalance = 6,
    IncompleteForward = 7,
    HtlcAddFailed = 8,
    ForwardsDisabled = 9,
    InvoiceCanceled = 10,
    InvoiceUnderpaid = 11,
    InvoiceExpiryTooSoon = 12,
    InvoiceNotOpen = 13,
    MppInvoiceTimeout = 14,
    AddressMismatch = 15,
    SetTotalMismatch = 16,
    SetTotalTooLow = 17,
    SetOverpaid = 18,
    UnknownInvoice = 19,
    InvalidKeysend = 20,
    MppInProgress = 21,
    CircularRoute = 22,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum PaymentState {
    ///
    ///Payment is still in flight.
    InFlight = 0,
    ///
    ///Payment completed successfully.
    Succeeded = 1,
    ///
    ///There are more routes to try, but the payment timeout was exceeded.
    FailedTimeout = 2,
    ///
    ///All possible routes were tried and failed permanently. Or were no
    ///routes to the destination at all.
    FailedNoRoute = 3,
    ///
    ///A non-recoverable error has occurred.
    FailedError = 4,
    ///
    ///Payment details incorrect (unknown hash, invalid amt or
    ///invalid final cltv delta)
    FailedIncorrectPaymentDetails = 5,
    ///
    ///Insufficient local balance.
    FailedInsufficientBalance = 6,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum ResolveHoldForwardAction {
    /// SETTLE is an action that is used to settle an HTLC instead of forwarding
    /// it.
    Settle = 0,
    /// FAIL is an action that is used to fail an HTLC backwards.
    Fail = 1,
    /// RESUME is an action that is used to resume a forward HTLC.
    Resume = 2,
    /// RESUME_MODIFIED is an action that is used to resume a hold forward HTLC
    /// with modifications specified during interception.
    ResumeModified = 3,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum ChanStatusAction {
    Enable = 0,
    Disable = 1,
    Auto = 2,
}
# [doc = r" Generated client implementations."] pub mod router_client { # ! [allow (unused_variables , dead_code , missing_docs , clippy :: let_unit_value ,)] use tonic :: codegen :: * ; # [doc = " Router is a service that offers advanced interaction with the router"] # [doc = " subsystem of the daemon."] # [derive (Debug , Clone)] pub struct RouterClient < T > { inner : tonic :: client :: Grpc < T > , } impl RouterClient < tonic :: transport :: Channel > { # [doc = r" Attempt to create a new client by connecting to a given endpoint."] pub async fn connect < D > (dst : D) -> Result < Self , tonic :: transport :: Error > where D : std :: convert :: TryInto < tonic :: transport :: Endpoint > , D :: Error : Into < StdError > , { let conn = tonic :: transport :: Endpoint :: new (dst) ? . connect () . await ? ; Ok (Self :: new (conn)) } } impl < T > RouterClient < T > where T : tonic :: client :: GrpcService < tonic :: body :: BoxBody > , T :: ResponseBody : Body + Send + Sync + 'static , T :: Error : Into < StdError > , < T :: ResponseBody as Body > :: Error : Into < StdError > + Send , { pub fn new (inner : T) -> Self { let inner = tonic :: client :: Grpc :: new (inner) ; Self { inner } } pub fn with_interceptor < F > (inner : T , interceptor : F) -> RouterClient < InterceptedService < T , F >> where F : tonic :: service :: Interceptor , T : tonic :: codegen :: Service < http :: Request < tonic :: body :: BoxBody > , Response = http :: Response << T as tonic :: client :: GrpcService < tonic :: body :: BoxBody >> :: ResponseBody > > , < T as tonic :: codegen :: Service < http :: Request < tonic :: body :: BoxBody >> > :: Error : Into < StdError > + Send + Sync , { RouterClient :: new (InterceptedService :: new (inner , interceptor)) } # [doc = r" Compress requests with `gzip`."] # [doc = r""] # [doc = r" This requires the server to support it otherwise it might respond with an"] # [doc = r" error."] pub fn send_gzip (mut self) -> Self { self . inner = self . inner . send_gzip () ; self } # [doc = r" Enable decompressing responses with `gzip`."] pub fn accept_gzip (mut self) -> Self { self . inner = self . inner . accept_gzip () ; self } # [doc = ""] # [doc = "SendPaymentV2 attempts to route a payment described by the passed"] # [doc = "PaymentRequest to the final destination. The call returns a stream of"] # [doc = "payment updates. When using this RPC, make sure to set a fee limit, as the"] # [doc = "default routing fee limit is 0 sats. Without a non-zero fee limit only"] # [doc = "routes without fees will be attempted which often fails with"] # [doc = "FAILURE_REASON_NO_ROUTE."] pub async fn send_payment_v2 (& mut self , request : impl tonic :: IntoRequest < super :: SendPaymentRequest > ,) -> Result < tonic :: Response < tonic :: codec :: Streaming < super :: super :: lnrpc :: Payment >> , tonic :: Status > { self . inner . ready () . await . map_err (| e | { tonic :: Status :: new (tonic :: Code :: Unknown , format ! ("Service was not ready: {}" , e . into ())) }) ? ; let codec = tonic :: codec :: ProstCodec :: default () ; let path = http :: uri :: PathAndQuery :: from_static ("/routerrpc.Router/SendPaymentV2") ; self . inner . server_streaming (request . into_request () , path , codec) . await } # [doc = " lncli: `trackpayment`"] # [doc = "TrackPaymentV2 returns an update stream for the payment identified by the"] # [doc = "payment hash."] pub async fn track_payment_v2 (& mut self , request : impl tonic :: IntoRequest < super :: TrackPaymentRequest > ,) -> Result < tonic :: Response < tonic :: codec :: Streaming < super :: super :: lnrpc :: Payment >> , tonic :: Status > { self . inner . ready () . await . map_err (| e | { tonic :: Status :: new (tonic :: Code :: Unknown , format ! ("Service was not ready: {}" , e . into ())) }) ? ; let codec = tonic :: codec :: ProstCodec :: default () ; let path = http :: uri :: PathAndQuery :: from_static ("/routerrpc.Router/TrackPaymentV2") ; self . inner . server_streaming (request . into_request () , path , codec) . await } # [doc = ""] # [doc = "TrackPayments returns an update stream for every payment that is not in a"] # [doc = "terminal state. Note that if payments are in-flight while starting a new"] # [doc = "subscription, the start of the payment stream could produce out-of-order"] # [doc = "and/or duplicate events. In order to get updates for every in-flight"] # [doc = "payment attempt make sure to subscribe to this method before initiating any"] # [doc = "payments."] pub async fn track_payments (& mut self , request : impl tonic :: IntoRequest < super :: TrackPaymentsRequest > ,) -> Result < tonic :: Response < tonic :: codec :: Streaming < super :: super :: lnrpc :: Payment >> , tonic :: Status > { self . inner . ready () . await . map_err (| e | { tonic :: Status :: new (tonic :: Code :: Unknown , format ! ("Service was not ready: {}" , e . into ())) }) ? ; let codec = tonic :: codec :: ProstCodec :: default () ; let path = http :: uri :: PathAndQuery :: from_static ("/routerrpc.Router/TrackPayments") ; self . inner . server_streaming (request . into_request () , path , codec) . await } # [doc = ""] # [doc = "EstimateRouteFee allows callers to obtain a lower bound w.r.t how much it"] # [doc = "may cost to send an HTLC to the target end destination."] pub async fn estimate_route_fee (& mut self , request : impl tonic :: IntoRequest < super :: RouteFeeRequest > ,) -> Result < tonic :: Response < super :: RouteFeeResponse > , tonic :: Status > { self . inner . ready () . await . map_err (| e | { tonic :: Status :: new (tonic :: Code :: Unknown , format ! ("Service was not ready: {}" , e . into ())) }) ? ; let codec = tonic :: codec :: ProstCodec :: default () ; let path = http :: uri :: PathAndQuery :: from_static ("/routerrpc.Router/EstimateRouteFee") ; self . inner . unary (request . into_request () , path , codec) . await } # [doc = ""] # [doc = "Deprecated, use SendToRouteV2. SendToRoute attempts to make a payment via"] # [doc = "the specified route. This method differs from SendPayment in that it"] # [doc = "allows users to specify a full route manually. This can be used for"] # [doc = "things like rebalancing, and atomic swaps. It differs from the newer"] # [doc = "SendToRouteV2 in that it doesn't return the full HTLC information."] pub async fn send_to_route (& mut self , request : impl tonic :: IntoRequest < super :: SendToRouteRequest > ,) -> Result < tonic :: Response < super :: SendToRouteResponse > , tonic :: Status > { self . inner . ready () . await . map_err (| e | { tonic :: Status :: new (tonic :: Code :: Unknown , format ! ("Service was not ready: {}" , e . into ())) }) ? ; let codec = tonic :: codec :: ProstCodec :: default () ; let path = http :: uri :: PathAndQuery :: from_static ("/routerrpc.Router/SendToRoute") ; self . inner . unary (request . into_request () , path , codec) . await } # [doc = ""] # [doc = "SendToRouteV2 attempts to make a payment via the specified route. This"] # [doc = "method differs from SendPayment in that it allows users to specify a full"] # [doc = "route manually. This can be used for things like rebalancing, and atomic"] # [doc = "swaps."] pub async fn send_to_route_v2 (& mut self , request : impl tonic :: IntoRequest < super :: SendToRouteRequest > ,) -> Result < tonic :: Response < super :: super :: lnrpc :: HtlcAttempt > , tonic :: Status > { self . inner . ready () . await . map_err (| e | { tonic :: Status :: new (tonic :: Code :: Unknown , format ! ("Service was not ready: {}" , e . into ())) }) ? ; let codec = tonic :: codec :: ProstCodec :: default () ; let path = http :: uri :: PathAndQuery :: from_static ("/routerrpc.Router/SendToRouteV2") ; self . inner . unary (request . into_request () , path , codec) . await } # [doc = " lncli: `resetmc`"] # [doc = "ResetMissionControl clears all mission control state and starts with a clean"] # [doc = "slate."] pub async fn reset_mission_control (& mut self , request : impl tonic :: IntoRequest < super :: ResetMissionControlRequest > ,) -> Result < tonic :: Response < super :: ResetMissionControlResponse > , tonic :: Status > { self . inner . ready () . await . map_err (| e | { tonic :: Status :: new (tonic :: Code :: Unknown , format ! ("Service was not ready: {}" , e . into ())) }) ? ; let codec = tonic :: codec :: ProstCodec :: default () ; let path = http :: uri :: PathAndQuery :: from_static ("/routerrpc.Router/ResetMissionControl") ; self . inner . unary (request . into_request () , path , codec) . await } # [doc = " lncli: `querymc`"] # [doc = "QueryMissionControl exposes the internal mission control state to callers."] # [doc = "It is a development feature."] pub async fn query_mission_control (& mut self , request : impl tonic :: IntoRequest < super :: QueryMissionControlRequest > ,) -> Result < tonic :: Response < super :: QueryMissionControlResponse > , tonic :: Status > { self . inner . ready () . await . map_err (| e | { tonic :: Status :: new (tonic :: Code :: Unknown , format ! ("Service was not ready: {}" , e . into ())) }) ? ; let codec = tonic :: codec :: ProstCodec :: default () ; let path = http :: uri :: PathAndQuery :: from_static ("/routerrpc.Router/QueryMissionControl") ; self . inner . unary (request . into_request () , path , codec) . await } # [doc = " lncli: `importmc`"] # [doc = "XImportMissionControl is an experimental API that imports the state provided"] # [doc = "to the internal mission control's state, using all results which are more"] # [doc = "recent than our existing values. These values will only be imported"] # [doc = "in-memory, and will not be persisted across restarts."] pub async fn x_import_mission_control (& mut self , request : impl tonic :: IntoRequest < super :: XImportMissionControlRequest > ,) -> Result < tonic :: Response < super :: XImportMissionControlResponse > , tonic :: Status > { self . inner . ready () . await . map_err (| e | { tonic :: Status :: new (tonic :: Code :: Unknown , format ! ("Service was not ready: {}" , e . into ())) }) ? ; let codec = tonic :: codec :: ProstCodec :: default () ; let path = http :: uri :: PathAndQuery :: from_static ("/routerrpc.Router/XImportMissionControl") ; self . inner . unary (request . into_request () , path , codec) . await } # [doc = " lncli: `getmccfg`"] # [doc = "GetMissionControlConfig returns mission control's current config."] pub async fn get_mission_control_config (& mut self , request : impl tonic :: IntoRequest < super :: GetMissionControlConfigRequest > ,) -> Result < tonic :: Response < super :: GetMissionControlConfigResponse > , tonic :: Status > { self . inner . ready () . await . map_err (| e | { tonic :: Status :: new (tonic :: Code :: Unknown , format ! ("Service was not ready: {}" , e . into ())) }) ? ; let codec = tonic :: codec :: ProstCodec :: default () ; let path = http :: uri :: PathAndQuery :: from_static ("/routerrpc.Router/GetMissionControlConfig") ; self . inner . unary (request . into_request () , path , codec) . await } # [doc = " lncli: `setmccfg`"] # [doc = "SetMissionControlConfig will set mission control's config, if the config"] # [doc = "provided is valid."] pub async fn set_mission_control_config (& mut self , request : impl tonic :: IntoRequest < super :: SetMissionControlConfigRequest > ,) -> Result < tonic :: Response < super :: SetMissionControlConfigResponse > , tonic :: Status > { self . inner . ready () . await . map_err (| e | { tonic :: Status :: new (tonic :: Code :: Unknown , format ! ("Service was not ready: {}" , e . into ())) }) ? ; let codec = tonic :: codec :: ProstCodec :: default () ; let path = http :: uri :: PathAndQuery :: from_static ("/routerrpc.Router/SetMissionControlConfig") ; self . inner . unary (request . into_request () , path , codec) . await } # [doc = " lncli: `queryprob`"] # [doc = "Deprecated. QueryProbability returns the current success probability"] # [doc = "estimate for a given node pair and amount. The call returns a zero success"] # [doc = "probability if no channel is available or if the amount violates min/max"] # [doc = "HTLC constraints."] pub async fn query_probability (& mut self , request : impl tonic :: IntoRequest < super :: QueryProbabilityRequest > ,) -> Result < tonic :: Response < super :: QueryProbabilityResponse > , tonic :: Status > { self . inner . ready () . await . map_err (| e | { tonic :: Status :: new (tonic :: Code :: Unknown , format ! ("Service was not ready: {}" , e . into ())) }) ? ; let codec = tonic :: codec :: ProstCodec :: default () ; let path = http :: uri :: PathAndQuery :: from_static ("/routerrpc.Router/QueryProbability") ; self . inner . unary (request . into_request () , path , codec) . await } # [doc = " lncli: `buildroute`"] # [doc = "BuildRoute builds a fully specified route based on a list of hop public"] # [doc = "keys. It retrieves the relevant channel policies from the graph in order to"] # [doc = "calculate the correct fees and time locks."] # [doc = "Note that LND will use its default final_cltv_delta if no value is supplied."] # [doc = "Make sure to add the correct final_cltv_delta depending on the invoice"] # [doc = "restriction. Moreover the caller has to make sure to provide the"] # [doc = "payment_addr if the route is paying an invoice which signaled it."] pub async fn build_route (& mut self , request : impl tonic :: IntoRequest < super :: BuildRouteRequest > ,) -> Result < tonic :: Response < super :: BuildRouteResponse > , tonic :: Status > { self . inner . ready () . await . map_err (| e | { tonic :: Status :: new (tonic :: Code :: Unknown , format ! ("Service was not ready: {}" , e . into ())) }) ? ; let codec = tonic :: codec :: ProstCodec :: default () ; let path = http :: uri :: PathAndQuery :: from_static ("/routerrpc.Router/BuildRoute") ; self . inner . unary (request . into_request () , path , codec) . await } # [doc = ""] # [doc = "SubscribeHtlcEvents creates a uni-directional stream from the server to"] # [doc = "the client which delivers a stream of htlc events."] pub async fn subscribe_htlc_events (& mut self , request : impl tonic :: IntoRequest < super :: SubscribeHtlcEventsRequest > ,) -> Result < tonic :: Response < tonic :: codec :: Streaming < super :: HtlcEvent >> , tonic :: Status > { self . inner . ready () . await . map_err (| e | { tonic :: Status :: new (tonic :: Code :: Unknown , format ! ("Service was not ready: {}" , e . into ())) }) ? ; let codec = tonic :: codec :: ProstCodec :: default () ; let path = http :: uri :: PathAndQuery :: from_static ("/routerrpc.Router/SubscribeHtlcEvents") ; self . inner . server_streaming (request . into_request () , path , codec) . await } # [doc = ""] # [doc = "Deprecated, use SendPaymentV2. SendPayment attempts to route a payment"] # [doc = "described by the passed PaymentRequest to the final destination. The call"] # [doc = "returns a stream of payment status updates."] pub async fn send_payment (& mut self , request : impl tonic :: IntoRequest < super :: SendPaymentRequest > ,) -> Result < tonic :: Response < tonic :: codec :: Streaming < super :: PaymentStatus >> , tonic :: Status > { self . inner . ready () . await . map_err (| e | { tonic :: Status :: new (tonic :: Code :: Unknown , format ! ("Service was not ready: {}" , e . into ())) }) ? ; let codec = tonic :: codec :: ProstCodec :: default () ; let path = http :: uri :: PathAndQuery :: from_static ("/routerrpc.Router/SendPayment") ; self . inner . server_streaming (request . into_request () , path , codec) . await } # [doc = ""] # [doc = "Deprecated, use TrackPaymentV2. TrackPayment returns an update stream for"] # [doc = "the payment identified by the payment hash."] pub async fn track_payment (& mut self , request : impl tonic :: IntoRequest < super :: TrackPaymentRequest > ,) -> Result < tonic :: Response < tonic :: codec :: Streaming < super :: PaymentStatus >> , tonic :: Status > { self . inner . ready () . await . map_err (| e | { tonic :: Status :: new (tonic :: Code :: Unknown , format ! ("Service was not ready: {}" , e . into ())) }) ? ; let codec = tonic :: codec :: ProstCodec :: default () ; let path = http :: uri :: PathAndQuery :: from_static ("/routerrpc.Router/TrackPayment") ; self . inner . server_streaming (request . into_request () , path , codec) . await } # [doc = "*"] # [doc = "HtlcInterceptor dispatches a bi-directional streaming RPC in which"] # [doc = "Forwarded HTLC requests are sent to the client and the client responds with"] # [doc = "a boolean that tells LND if this htlc should be intercepted."] # [doc = "In case of interception, the htlc can be either settled, cancelled or"] # [doc = "resumed later by using the ResolveHoldForward endpoint."] pub async fn htlc_interceptor (& mut self , request : impl tonic :: IntoStreamingRequest < Message = super :: ForwardHtlcInterceptResponse >) -> Result < tonic :: Response < tonic :: codec :: Streaming < super :: ForwardHtlcInterceptRequest >> , tonic :: Status > { self . inner . ready () . await . map_err (| e | { tonic :: Status :: new (tonic :: Code :: Unknown , format ! ("Service was not ready: {}" , e . into ())) }) ? ; let codec = tonic :: codec :: ProstCodec :: default () ; let path = http :: uri :: PathAndQuery :: from_static ("/routerrpc.Router/HtlcInterceptor") ; self . inner . streaming (request . into_streaming_request () , path , codec) . await } # [doc = " lncli: `updatechanstatus`"] # [doc = "UpdateChanStatus attempts to manually set the state of a channel"] # [doc = "(enabled, disabled, or auto). A manual \"disable\" request will cause the"] # [doc = "channel to stay disabled until a subsequent manual request of either"] # [doc = "\"enable\" or \"auto\"."] pub async fn update_chan_status (& mut self , request : impl tonic :: IntoRequest < super :: UpdateChanStatusRequest > ,) -> Result < tonic :: Response < super :: UpdateChanStatusResponse > , tonic :: Status > { self . inner . ready () . await . map_err (| e | { tonic :: Status :: new (tonic :: Code :: Unknown , format ! ("Service was not ready: {}" , e . into ())) }) ? ; let codec = tonic :: codec :: ProstCodec :: default () ; let path = http :: uri :: PathAndQuery :: from_static ("/routerrpc.Router/UpdateChanStatus") ; self . inner . unary (request . into_request () , path , codec) . await } # [doc = ""] # [doc = "XAddLocalChanAliases is an experimental API that creates a set of new"] # [doc = "channel SCID alias mappings. The final total set of aliases in the manager"] # [doc = "after the add operation is returned. This is only a locally stored alias,"] # [doc = "and will not be communicated to the channel peer via any message. Therefore,"] # [doc = "routing over such an alias will only work if the peer also calls this same"] # [doc = "RPC on their end. If an alias already exists, an error is returned"] pub async fn x_add_local_chan_aliases (& mut self , request : impl tonic :: IntoRequest < super :: AddAliasesRequest > ,) -> Result < tonic :: Response < super :: AddAliasesResponse > , tonic :: Status > { self . inner . ready () . await . map_err (| e | { tonic :: Status :: new (tonic :: Code :: Unknown , format ! ("Service was not ready: {}" , e . into ())) }) ? ; let codec = tonic :: codec :: ProstCodec :: default () ; let path = http :: uri :: PathAndQuery :: from_static ("/routerrpc.Router/XAddLocalChanAliases") ; self . inner . unary (request . into_request () , path , codec) . await } # [doc = ""] # [doc = "XDeleteLocalChanAliases is an experimental API that deletes a set of alias"] # [doc = "mappings. The final total set of aliases in the manager after the delete"] # [doc = "operation is returned. The deletion will not be communicated to the channel"] # [doc = "peer via any message."] pub async fn x_delete_local_chan_aliases (& mut self , request : impl tonic :: IntoRequest < super :: DeleteAliasesRequest > ,) -> Result < tonic :: Response < super :: DeleteAliasesResponse > , tonic :: Status > { self . inner . ready () . await . map_err (| e | { tonic :: Status :: new (tonic :: Code :: Unknown , format ! ("Service was not ready: {}" , e . into ())) }) ? ; let codec = tonic :: codec :: ProstCodec :: default () ; let path = http :: uri :: PathAndQuery :: from_static ("/routerrpc.Router/XDeleteLocalChanAliases") ; self . inner . unary (request . into_request () , path , codec) . await } # [doc = ""] # [doc = "XFindBaseLocalChanAlias is an experimental API that looks up the base scid"] # [doc = "for a local chan alias that was registered during the current runtime."] pub async fn x_find_base_local_chan_alias (& mut self , request : impl tonic :: IntoRequest < super :: FindBaseAliasRequest > ,) -> Result < tonic :: Response < super :: FindBaseAliasResponse > , tonic :: Status > { self . inner . ready () . await . map_err (| e | { tonic :: Status :: new (tonic :: Code :: Unknown , format ! ("Service was not ready: {}" , e . into ())) }) ? ; let codec = tonic :: codec :: ProstCodec :: default () ; let path = http :: uri :: PathAndQuery :: from_static ("/routerrpc.Router/XFindBaseLocalChanAlias") ; self . inner . unary (request . into_request () , path , codec) . await } } }