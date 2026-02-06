//! WebRTC data channel transport for cross-network sync.
//!
//! Uses `datachannel-rs` (libdatachannel FFI) for WebRTC peer connections.
//! Bridges datachannel's synchronous callbacks to tokio async via mpsc channels,
//! producing `DataChannelStream` and `DataChannelSink` types that satisfy the
//! same `Stream`/`Sink` bounds as `run_sync_loop`'s generic parameters.
//!
//! The negotiation flow:
//! 1. Both peers connect to the signaling relay
//! 2. Exchange SPAKE2 messages through signaling
//! 3. Derive SessionEncryption from SPAKE2 shared secret
//! 4. Exchange encrypted SDP offer/answer + ICE candidates via signaling
//! 5. Wait for data channel to open
//! 6. Return (Sink, Stream, encryption) for reuse by `run_sync_loop`

use async_tungstenite::tungstenite;
use datachannel::{
    ConnectionState, DataChannelHandler, DataChannelInfo, DataChannelInit, GatheringState,
    IceCandidate, PeerConnectionHandler, RtcConfig, RtcDataChannel, RtcPeerConnection,
    SessionDescription, SdpType,
};
use futures_util::{Sink, Stream};
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use std::sync::{Arc, Mutex as StdMutex};
use std::task::{Context, Poll, Waker};
use tokio::sync::mpsc;

use crate::sync::encryption::SessionEncryption;
use crate::sync::pairing::{PairingCreator, PairingJoiner};
use crate::sync::signaling::{SignalingClient, SignalingEvent};

// ---------------------------------------------------------------------------
// Signaling payload types (exchanged through relay, encrypted after SPAKE2)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
enum SignalingPayload {
    #[serde(rename = "spake2")]
    Spake2 { data: Vec<u8> },
    #[serde(rename = "sdp")]
    Sdp { sdp: String, sdp_type: String },
    #[serde(rename = "ice")]
    Ice { candidate: String, mid: String },
    #[serde(rename = "ice_done")]
    IceDone,
}

// ---------------------------------------------------------------------------
// DataChannel → async adapters
// ---------------------------------------------------------------------------

/// Shared state between datachannel callbacks and async adapters.
struct DcShared {
    /// Incoming messages from the data channel.
    msg_tx: mpsc::UnboundedSender<Vec<u8>>,
    /// Notified when the data channel opens.
    open_tx: Option<tokio::sync::oneshot::Sender<()>>,
    /// ICE candidates gathered by the peer connection.
    ice_tx: mpsc::UnboundedSender<IceCandidate>,
    /// Notified when ICE gathering is complete.
    gathering_done_tx: Option<tokio::sync::oneshot::Sender<()>>,
    /// Current connection state.
    connection_state: ConnectionState,
    /// Waker for the Stream impl.
    waker: Option<Waker>,
}

/// PeerConnection event handler — forwards events to async channels.
struct PcHandler {
    shared: Arc<StdMutex<DcShared>>,
}

impl PeerConnectionHandler for PcHandler {
    type DCH = DcHandler;

    fn data_channel_handler(&mut self, _info: DataChannelInfo) -> Self::DCH {
        DcHandler {
            shared: self.shared.clone(),
        }
    }

    fn on_candidate(&mut self, candidate: IceCandidate) {
        let shared = self.shared.lock().unwrap();
        let _ = shared.ice_tx.send(candidate);
    }

    fn on_gathering_state_change(&mut self, state: GatheringState) {
        if state == GatheringState::Complete {
            let mut shared = self.shared.lock().unwrap();
            if let Some(tx) = shared.gathering_done_tx.take() {
                let _ = tx.send(());
            }
        }
    }

    fn on_connection_state_change(&mut self, state: ConnectionState) {
        let mut shared = self.shared.lock().unwrap();
        shared.connection_state = state;
        if let Some(w) = shared.waker.take() {
            w.wake();
        }
    }
}

/// DataChannel event handler — forwards messages and open events.
struct DcHandler {
    shared: Arc<StdMutex<DcShared>>,
}

impl DataChannelHandler for DcHandler {
    fn on_open(&mut self) {
        let mut shared = self.shared.lock().unwrap();
        if let Some(tx) = shared.open_tx.take() {
            let _ = tx.send(());
        }
    }

    fn on_message(&mut self, msg: &[u8]) {
        let shared = self.shared.lock().unwrap();
        let _ = shared.msg_tx.send(msg.to_vec());
        if let Some(w) = shared.waker.as_ref() {
            w.wake_by_ref();
        }
    }

    fn on_closed(&mut self) {
        // Channel closed — dropping msg_tx will signal the Stream
    }

    fn on_error(&mut self, err: &str) {
        tracing::error!("DataChannel error: {}", err);
    }
}

// ---------------------------------------------------------------------------
// DataChannelStream — implements Stream<Item = Result<tungstenite::Message>>
// ---------------------------------------------------------------------------

/// Async stream adapter for datachannel-rs incoming messages.
/// Implements the same `Stream` trait bound that `run_sync_loop` expects.
pub struct DataChannelStream {
    rx: mpsc::UnboundedReceiver<Vec<u8>>,
}

impl Stream for DataChannelStream {
    type Item = Result<tungstenite::Message, tungstenite::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.rx.poll_recv(cx) {
            Poll::Ready(Some(bytes)) => {
                // Convert binary data to a tungstenite Text message containing
                // the UTF-8 string, matching the wire format used by run_sync_loop.
                // The sync loop expects JSON text messages.
                match String::from_utf8(bytes) {
                    Ok(text) => Poll::Ready(Some(Ok(tungstenite::Message::Text(text)))),
                    Err(e) => {
                        // If not valid UTF-8, treat as binary
                        Poll::Ready(Some(Ok(tungstenite::Message::Binary(
                            e.into_bytes().into(),
                        ))))
                    }
                }
            }
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

// ---------------------------------------------------------------------------
// DataChannelSink — implements Sink<tungstenite::Message>
// ---------------------------------------------------------------------------

/// Async sink adapter for datachannel-rs outgoing messages.
/// Implements the same `Sink` trait bound that `run_sync_loop` expects.
pub struct DataChannelSink {
    dc: Box<RtcDataChannel<DcHandler>>,
}

impl Sink<tungstenite::Message> for DataChannelSink {
    type Error = tungstenite::Error;

    fn poll_ready(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        // datachannel-rs send is synchronous and doesn't need readiness polling
        Poll::Ready(Ok(()))
    }

    fn start_send(mut self: Pin<&mut Self>, item: tungstenite::Message) -> Result<(), Self::Error> {
        let bytes = match item {
            tungstenite::Message::Text(t) => t.into_bytes(),
            tungstenite::Message::Binary(b) => b.to_vec(),
            tungstenite::Message::Close(_) => return Ok(()),
            _ => return Ok(()),
        };
        self.dc
            .send(&bytes)
            .map_err(|e| tungstenite::Error::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("DataChannel send failed: {}", e),
            )))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        // datachannel-rs send is synchronous — no buffering to flush
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
}

// ---------------------------------------------------------------------------
// WebRTC session (kept alive for the duration of sync)
// ---------------------------------------------------------------------------

/// Holds the WebRTC peer connection and data channel alive.
/// When dropped, the connection is closed.
pub struct WebRtcSession {
    _pc: Box<RtcPeerConnection<PcHandler>>,
}

// ---------------------------------------------------------------------------
// Main entry point
// ---------------------------------------------------------------------------

/// Establish a WebRTC data channel transport through the signaling server.
///
/// Returns a (Sink, Stream, Encryption, Session) tuple ready for `run_sync_loop`.
/// The `WebRtcSession` must be held alive for the duration of the sync.
pub async fn establish_webrtc_transport(
    signaling_url: &str,
    pairing_code: &str,
    device_id: &str,
    is_creator: bool,
) -> Result<(DataChannelSink, DataChannelStream, SessionEncryption, WebRtcSession), String> {
    // 1. Connect to signaling server
    let mut signaling = SignalingClient::connect(signaling_url, pairing_code, device_id).await?;
    tracing::info!("WebRTC: connected to signaling server");

    // 2. Wait for peer if creator; if joiner, peer should already be there
    if is_creator {
        tracing::info!("WebRTC: waiting for peer on signaling server...");
        let peer_id = signaling.wait_for_peer().await?;
        tracing::info!("WebRTC: peer joined: {}", peer_id);
    }

    // 3. SPAKE2 key exchange through signaling
    let encryption = if is_creator {
        let creator = PairingCreator::start(pairing_code);
        // Send our SPAKE2 message
        let spake_payload = SignalingPayload::Spake2 {
            data: creator.outbound_msg.clone(),
        };
        let json = serde_json::to_string(&spake_payload).map_err(|e| e.to_string())?;
        signaling.send(json.as_bytes()).await?;

        // Receive joiner's SPAKE2 message
        let joiner_spake = recv_payload(&mut signaling).await?;
        match serde_json::from_slice::<SignalingPayload>(&joiner_spake)
            .map_err(|e| format!("Invalid SPAKE2 response: {}", e))?
        {
            SignalingPayload::Spake2 { data } => creator.finish(&data)?,
            _ => return Err("Expected SPAKE2 message from peer".to_string()),
        }
    } else {
        let joiner = PairingJoiner::start(pairing_code);
        // Receive creator's SPAKE2 message
        let creator_spake = recv_payload(&mut signaling).await?;
        match serde_json::from_slice::<SignalingPayload>(&creator_spake)
            .map_err(|e| format!("Invalid SPAKE2 message: {}", e))?
        {
            SignalingPayload::Spake2 { data } => {
                // Send our SPAKE2 message
                let spake_payload = SignalingPayload::Spake2 {
                    data: joiner.outbound_msg.clone(),
                };
                let json = serde_json::to_string(&spake_payload).map_err(|e| e.to_string())?;
                signaling.send(json.as_bytes()).await?;

                joiner.finish(&data)?
            }
            _ => return Err("Expected SPAKE2 message from peer".to_string()),
        }
    };
    tracing::info!("WebRTC: SPAKE2 key exchange complete");

    // 4. Create WebRTC peer connection
    let (msg_tx, msg_rx) = mpsc::unbounded_channel();
    let (ice_tx, mut ice_rx) = mpsc::unbounded_channel();
    let (open_tx, open_rx) = tokio::sync::oneshot::channel();
    let (gathering_done_tx, gathering_done_rx) = tokio::sync::oneshot::channel();

    let shared = Arc::new(StdMutex::new(DcShared {
        msg_tx,
        open_tx: Some(open_tx),
        ice_tx,
        gathering_done_tx: Some(gathering_done_tx),
        connection_state: ConnectionState::New,
        waker: None,
    }));

    let config = RtcConfig::new(&["stun:stun.l.google.com:19302"]);

    let pc_handler = PcHandler {
        shared: shared.clone(),
    };
    let mut pc = RtcPeerConnection::new(&config, pc_handler)
        .map_err(|e| format!("Failed to create RTCPeerConnection: {}", e))?;

    // 5. Creator creates data channel and generates offer; joiner waits for offer
    let dc = if is_creator {
        // Create data channel
        let dc_handler = DcHandler {
            shared: shared.clone(),
        };
        let dc = pc
            .create_data_channel_ex("aurus-sync", dc_handler, &DataChannelInit::default())
            .map_err(|e| format!("Failed to create data channel: {}", e))?;

        // Gather local description (offer)
        let local_desc = pc
            .local_description()
            .ok_or("No local description after creating data channel")?;

        // Send encrypted SDP offer
        let sdp_payload = SignalingPayload::Sdp {
            sdp: local_desc.sdp.to_string(),
            sdp_type: sd_type_to_string(&local_desc.sdp_type),
        };
        let json = serde_json::to_string(&sdp_payload).map_err(|e| e.to_string())?;
        let envelope = encryption.encrypt(json.as_bytes())?;
        let envelope_json = serde_json::to_string(&envelope).map_err(|e| e.to_string())?;
        signaling.send(envelope_json.as_bytes()).await?;

        // Send ICE candidates as they're gathered
        let signaling_ice = send_ice_candidates(&mut ice_rx, &encryption, &signaling).await;

        // Wait for gathering to complete, then signal done
        let _ = gathering_done_rx.await;
        let done_payload = SignalingPayload::IceDone;
        let json = serde_json::to_string(&done_payload).map_err(|e| e.to_string())?;
        let envelope = encryption.encrypt(json.as_bytes())?;
        let envelope_json = serde_json::to_string(&envelope).map_err(|e| e.to_string())?;
        signaling.send(envelope_json.as_bytes()).await?;

        // Receive answer
        let answer_bytes = recv_payload(&mut signaling).await?;
        let answer_envelope: crate::sync::encryption::EncryptedEnvelope =
            serde_json::from_slice(&answer_bytes)
                .map_err(|e| format!("Invalid SDP answer envelope: {}", e))?;
        let answer_json = encryption.decrypt(&answer_envelope)?;
        let answer: SignalingPayload = serde_json::from_slice(&answer_json)
            .map_err(|e| format!("Invalid SDP answer: {}", e))?;

        match answer {
            SignalingPayload::Sdp { sdp, sdp_type } => {
                let sdp_session = webrtc_sdp::parse_sdp(&sdp, false)
                    .map_err(|e| format!("Failed to parse SDP answer: {:?}", e))?;
                let sd = SessionDescription {
                    sdp: sdp_session,
                    sdp_type: string_to_sd_type(&sdp_type),
                };
                pc.set_remote_description(&sd)
                    .map_err(|e| format!("Failed to set remote description: {}", e))?;
            }
            _ => return Err("Expected SDP answer".to_string()),
        }

        // Receive remote ICE candidates
        receive_ice_candidates(&mut signaling, &encryption, &mut pc).await?;

        // Wait for any remaining local ICE candidates
        let _ = signaling_ice;

        dc
    } else {
        // Joiner: receive SDP offer
        let offer_bytes = recv_payload(&mut signaling).await?;
        let offer_envelope: crate::sync::encryption::EncryptedEnvelope =
            serde_json::from_slice(&offer_bytes)
                .map_err(|e| format!("Invalid SDP offer envelope: {}", e))?;
        let offer_json = encryption.decrypt(&offer_envelope)?;
        let offer: SignalingPayload = serde_json::from_slice(&offer_json)
            .map_err(|e| format!("Invalid SDP offer: {}", e))?;

        match offer {
            SignalingPayload::Sdp { sdp, sdp_type } => {
                let sdp_session = webrtc_sdp::parse_sdp(&sdp, false)
                    .map_err(|e| format!("Failed to parse SDP offer: {:?}", e))?;
                let sd = SessionDescription {
                    sdp: sdp_session,
                    sdp_type: string_to_sd_type(&sdp_type),
                };
                pc.set_remote_description(&sd)
                    .map_err(|e| format!("Failed to set remote description: {}", e))?;
            }
            _ => return Err("Expected SDP offer".to_string()),
        }

        // Receive remote ICE candidates
        receive_ice_candidates(&mut signaling, &encryption, &mut pc).await?;

        // The data channel handler will be called when the creator's channel arrives.
        // We need a reference to it — datachannel-rs creates it via the handler callback.
        // For the joiner, we need to wait for the on_open event which signals the DC is ready.

        // Get local description (answer)
        let local_desc = pc
            .local_description()
            .ok_or("No local description after setting remote description")?;

        // Send encrypted SDP answer
        let sdp_payload = SignalingPayload::Sdp {
            sdp: local_desc.sdp.to_string(),
            sdp_type: sd_type_to_string(&local_desc.sdp_type),
        };
        let json = serde_json::to_string(&sdp_payload).map_err(|e| e.to_string())?;
        let envelope = encryption.encrypt(json.as_bytes())?;
        let envelope_json = serde_json::to_string(&envelope).map_err(|e| e.to_string())?;
        signaling.send(envelope_json.as_bytes()).await?;

        // Send local ICE candidates
        let _signaling_ice = send_ice_candidates(&mut ice_rx, &encryption, &signaling).await;

        // Wait for gathering, then signal done
        let _ = gathering_done_rx.await;
        let done_payload = SignalingPayload::IceDone;
        let json = serde_json::to_string(&done_payload).map_err(|e| e.to_string())?;
        let envelope = encryption.encrypt(json.as_bytes())?;
        let envelope_json = serde_json::to_string(&envelope).map_err(|e| e.to_string())?;
        signaling.send(envelope_json.as_bytes()).await?;

        // Joiner doesn't directly hold the DC — the handler created it.
        // We'll create a "dummy" DC reference. Actually, for the joiner the DC
        // is created by the peer connection handler when the remote DC arrives.
        // We need a different approach: create our own DC too so we have a send handle.
        let dc_handler = DcHandler {
            shared: shared.clone(),
        };
        pc.create_data_channel_ex("aurus-sync", dc_handler, &DataChannelInit::default())
            .map_err(|e| format!("Failed to create data channel: {}", e))?
    };

    // 6. Wait for data channel to open (with timeout)
    tracing::info!("WebRTC: waiting for data channel to open...");
    tokio::time::timeout(std::time::Duration::from_secs(30), open_rx)
        .await
        .map_err(|_| "Data channel open timeout (30s)".to_string())?
        .map_err(|_| "Data channel open signal dropped".to_string())?;

    tracing::info!("WebRTC: data channel open — transport ready");

    // 7. Close signaling (no longer needed)
    signaling.close();

    let stream = DataChannelStream { rx: msg_rx };
    let sink = DataChannelSink { dc };
    let session = WebRtcSession { _pc: pc };

    Ok((sink, stream, encryption, session))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Receive the next Payload event from signaling (skipping non-payload events).
async fn recv_payload(signaling: &mut SignalingClient) -> Result<Vec<u8>, String> {
    loop {
        match signaling.recv().await? {
            SignalingEvent::Payload(bytes) => return Ok(bytes),
            SignalingEvent::PeerJoined(id) => {
                tracing::info!("WebRTC signaling: peer joined: {}", id);
            }
            SignalingEvent::PeerLeft(id) => {
                return Err(format!("Peer {} left during negotiation", id));
            }
        }
    }
}

/// Send ICE candidates through signaling as they're gathered.
/// Returns a handle that should be kept alive until gathering is done.
async fn send_ice_candidates(
    ice_rx: &mut mpsc::UnboundedReceiver<IceCandidate>,
    encryption: &SessionEncryption,
    signaling: &SignalingClient,
) -> () {
    // Drain any already-gathered candidates
    while let Ok(candidate) = ice_rx.try_recv() {
        let payload = SignalingPayload::Ice {
            candidate: candidate.candidate,
            mid: candidate.mid,
        };
        if let Ok(json) = serde_json::to_string(&payload) {
            if let Ok(envelope) = encryption.encrypt(json.as_bytes()) {
                if let Ok(envelope_json) = serde_json::to_string(&envelope) {
                    let _ = signaling.send(envelope_json.as_bytes()).await;
                }
            }
        }
    }
}

/// Receive ICE candidates from signaling and add them to the peer connection.
async fn receive_ice_candidates(
    signaling: &mut SignalingClient,
    encryption: &SessionEncryption,
    pc: &mut Box<RtcPeerConnection<PcHandler>>,
) -> Result<(), String> {
    loop {
        let bytes = recv_payload(signaling).await?;
        let envelope: crate::sync::encryption::EncryptedEnvelope =
            serde_json::from_slice(&bytes)
                .map_err(|e| format!("Invalid ICE envelope: {}", e))?;
        let json = encryption.decrypt(&envelope)?;
        let payload: SignalingPayload =
            serde_json::from_slice(&json).map_err(|e| format!("Invalid ICE payload: {}", e))?;

        match payload {
            SignalingPayload::Ice { candidate, mid } => {
                let ice = IceCandidate { candidate, mid };
                pc.add_remote_candidate(&ice)
                    .map_err(|e| format!("Failed to add ICE candidate: {}", e))?;
            }
            SignalingPayload::IceDone => {
                tracing::info!("WebRTC: received all remote ICE candidates");
                break;
            }
            _ => {
                tracing::warn!("WebRTC: unexpected payload during ICE gathering");
            }
        }
    }
    Ok(())
}

fn sd_type_to_string(t: &SdpType) -> String {
    match t {
        SdpType::Offer => "offer".to_string(),
        SdpType::Answer => "answer".to_string(),
        _ => "unknown".to_string(),
    }
}

fn string_to_sd_type(s: &str) -> SdpType {
    match s {
        "offer" => SdpType::Offer,
        "answer" => SdpType::Answer,
        _ => SdpType::Offer,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signaling_payload_spake2_serde() {
        let payload = SignalingPayload::Spake2 {
            data: vec![1, 2, 3],
        };
        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("\"type\":\"spake2\""));
        let parsed: SignalingPayload = serde_json::from_str(&json).unwrap();
        match parsed {
            SignalingPayload::Spake2 { data } => assert_eq!(data, vec![1, 2, 3]),
            _ => panic!("Expected Spake2"),
        }
    }

    #[test]
    fn test_signaling_payload_sdp_serde() {
        let payload = SignalingPayload::Sdp {
            sdp: "v=0\r\n...".to_string(),
            sdp_type: "offer".to_string(),
        };
        let json = serde_json::to_string(&payload).unwrap();
        let parsed: SignalingPayload = serde_json::from_str(&json).unwrap();
        match parsed {
            SignalingPayload::Sdp { sdp, sdp_type } => {
                assert_eq!(sdp, "v=0\r\n...");
                assert_eq!(sdp_type, "offer");
            }
            _ => panic!("Expected Sdp"),
        }
    }

    #[test]
    fn test_signaling_payload_ice_serde() {
        let payload = SignalingPayload::Ice {
            candidate: "candidate:1 1 UDP ...".to_string(),
            mid: "0".to_string(),
        };
        let json = serde_json::to_string(&payload).unwrap();
        let parsed: SignalingPayload = serde_json::from_str(&json).unwrap();
        match parsed {
            SignalingPayload::Ice { candidate, mid } => {
                assert!(candidate.starts_with("candidate:"));
                assert_eq!(mid, "0");
            }
            _ => panic!("Expected Ice"),
        }
    }

    #[test]
    fn test_sd_type_round_trip() {
        assert_eq!(
            sd_type_to_string(&string_to_sd_type("offer")),
            "offer"
        );
        assert_eq!(
            sd_type_to_string(&string_to_sd_type("answer")),
            "answer"
        );
    }
}
