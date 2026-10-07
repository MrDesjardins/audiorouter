//! Network audio: pairing keys, senders, receivers and their diagnostics.

use super::*;

/// The pairing key of a Network Send/Receive node: `None` when blank (not
/// paired). A key that is present but invalid refuses to start rather than
/// silently streaming unpaired (SEC-13). The key itself is never logged.
#[cfg(windows)]
pub(crate) fn network_pairing_key(
    node: &audiorouter_domain::Node,
) -> Result<Option<audiorouter_windows_audio::NetworkPairingKey>, ControlError> {
    match node.parameters.get("pairingKey") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(key)) if audiorouter_domain::valid_network_pairing_key(key) => {
            Ok(audiorouter_windows_audio::NetworkPairingKey::derive(key))
        }
        Some(_) => Err(ControlError::InvalidRequest(format!(
            "the pairing key of {} must be blank or {}-{} printable characters",
            node.name,
            audiorouter_domain::MIN_NETWORK_PAIRING_KEY_CHARS,
            audiorouter_domain::MAX_NETWORK_PAIRING_KEY_CHARS
        ))),
    }
}

/// Open the UDP sender of a Network Send node from its validated
/// parameters. Only an IP literal and port are accepted (no name lookup).
#[cfg(windows)]
pub(crate) fn start_network_sender(
    node: &audiorouter_domain::Node,
) -> Result<audiorouter_windows_audio::NetworkSender, ControlError> {
    let host = node
        .parameters
        .get("host")
        .and_then(Value::as_str)
        .unwrap_or("");
    let port = node
        .parameters
        .get("port")
        .and_then(Value::as_u64)
        .and_then(|port| u16::try_from(port).ok())
        .unwrap_or(audiorouter_domain::DEFAULT_NETWORK_AUDIO_PORT);
    let destination =
        audiorouter_windows_audio::network_socket_address(host, port).ok_or_else(|| {
            ControlError::InvalidRequest(format!(
                "enter the IP address of the receiving computer for {} in its Properties",
                node.name
            ))
        })?;
    let pairing_key = network_pairing_key(node)?;
    let paired = pairing_key.is_some();
    match audiorouter_windows_audio::NetworkSender::start_paired(destination, pairing_key) {
        Ok(sender) => {
            network_log::write(json!({
                "event": "sendStarted", "role": "send", "nodeId": node.id.as_str(),
                "destination": destination.to_string(), "paired": paired,
                "localAddress": sender.stats().local_address.map(|address| address.to_string()),
            }));
            Ok(sender)
        }
        Err(error) => {
            let code = os_error_code(&error.to_string());
            network_log::write(json!({
                "event": "sendFailed", "role": "send", "nodeId": node.id.as_str(),
                "destination": destination.to_string(), "errorCode": code,
                "hint": code.and_then(network_log::socket_error_hint),
            }));
            Err(ControlError::InvalidRequest(format!(
                "{} could not open its network socket: {error}",
                node.name
            )))
        }
    }
}

/// The Windows socket error code inside an I/O error message
/// ("… (os error 10048)"), for the network log; never the text itself.
pub(crate) fn os_error_code(message: &str) -> Option<i32> {
    let start = message.rfind("(os error ")? + "(os error ".len();
    message[start..]
        .strip_suffix(')')
        .and_then(|code| code.parse().ok())
}

/// Open the UDP receiver of a Network Receive node from its validated
/// parameters: the sending computer's IP literal, the port and the jitter
/// buffer.
#[cfg(windows)]
pub(crate) fn start_network_receiver(
    node: &audiorouter_domain::Node,
) -> Result<audiorouter_windows_audio::NetworkReceiver, ControlError> {
    let sender = node
        .parameters
        .get("sender")
        .and_then(Value::as_str)
        .filter(|address| audiorouter_domain::valid_network_address(address))
        .and_then(|address| address.parse::<std::net::IpAddr>().ok())
        .ok_or_else(|| {
            ControlError::InvalidRequest(format!(
                "enter the IP address of the sending computer for {} in its Properties",
                node.name
            ))
        })?;
    let port = node
        .parameters
        .get("port")
        .and_then(Value::as_u64)
        .and_then(|port| u16::try_from(port).ok())
        .filter(|port| *port != 0)
        .unwrap_or(audiorouter_domain::DEFAULT_NETWORK_AUDIO_PORT);
    let buffer_ms = node
        .parameters
        .get("bufferMs")
        .and_then(Value::as_f64)
        .unwrap_or(audiorouter_domain::DEFAULT_NETWORK_BUFFER_MS);
    // The address the sending computer must target, as routed from here.
    let local_toward_sender =
        audiorouter_windows_audio::local_address_toward(sender).map(|address| address.to_string());
    let pairing_key = network_pairing_key(node)?;
    let paired = pairing_key.is_some();
    match audiorouter_windows_audio::NetworkReceiver::start_paired(
        sender,
        port,
        buffer_ms,
        pairing_key,
    ) {
        Ok(receiver) => {
            network_log::write(json!({
                "event": "receiveStarted", "role": "receive", "nodeId": node.id.as_str(),
                "expectedSender": sender.to_string(), "port": port, "bufferMs": buffer_ms,
                "paired": paired,
                "listen": receiver.listen_address().to_string(),
                "localAddressTowardSender": local_toward_sender,
            }));
            Ok(receiver)
        }
        Err(error) => {
            let code = os_error_code(&error.to_string());
            network_log::write(json!({
                "event": "receiveFailed", "role": "receive", "nodeId": node.id.as_str(),
                "expectedSender": sender.to_string(), "port": port, "errorCode": code,
                "hint": code.and_then(network_log::socket_error_hint),
            }));
            Err(ControlError::InvalidRequest(format!(
                "{} could not listen on UDP port {port} (is another program using it?): {error}",
                node.name
            )))
        }
    }
}

impl ControlPlane {
    /// Pump every running prepared native worker once, from the backend-owned
    /// audio service thread. The calls are the same bounded, non-waiting
    /// pumps the `native*.pump` methods expose, so endpoint invalidation,
    /// lease heartbeats and application maintenance keep their semantics.
    /// Errors are left for the UI/diagnostic pump paths to report; the
    /// service only skips a worker whose session generation is not running.
    /// Returns the number of workers serviced.
    /// Summarize playing Network Send/Receive nodes into `network.jsonl`
    /// (paced by the sampler), and write their last summaries on stop.
    #[cfg(windows)]
    pub(crate) fn sample_network_diagnostics(
        &mut self,
        now: std::time::Instant,
        playing: Option<EntityId>,
    ) {
        let Some(session_id) = playing else {
            for record in self.network_log.finish() {
                network_log::write(record);
            }
            return;
        };
        if !self.network_log.due(now) {
            return;
        }
        // The worker borrows the plane; the sampler is moved out meanwhile.
        let mut sampler = std::mem::take(&mut self.network_log);
        let _ = self.write_network_summaries(&session_id, &mut sampler, now);
        self.network_log = sampler;
    }

    #[cfg(windows)]
    pub(crate) fn write_network_summaries(
        &self,
        session_id: &EntityId,
        sampler: &mut network_log::Sampler,
        now: std::time::Instant,
    ) -> Vec<Value> {
        let Some(worker) = self.native_multi_input_worker.as_ref() else {
            return Vec::new();
        };
        let Ok(session) = self.get_session(session_id) else {
            return Vec::new();
        };
        let seconds_playing = sampler.seconds_playing(now);
        let parameter = |node_id: &EntityId, name: &str| {
            session
                .nodes
                .iter()
                .find(|node| node.id == *node_id)
                .and_then(|node| node.parameters.get(name).cloned())
        };
        let mut records = Vec::new();
        for (index, node_id) in worker.input_node_ids().iter().enumerate() {
            let Some(stats) = worker.network_receive_stats(index) else {
                continue;
            };
            let expected = parameter(node_id, "sender")
                .and_then(|value| value.as_str().map(str::to_owned))
                .unwrap_or_default();
            let port = parameter(node_id, "port")
                .and_then(|value| value.as_u64())
                .and_then(|port| u16::try_from(port).ok())
                .unwrap_or(audiorouter_domain::DEFAULT_NETWORK_AUDIO_PORT);
            let underruns_since_last =
                sampler.underruns_since_last(node_id.as_str(), stats.underruns);
            let summary = network_log::ReceiveSummary {
                node_id: node_id.as_str().to_owned(),
                local_address_toward_sender: expected
                    .parse()
                    .ok()
                    .and_then(audiorouter_windows_audio::local_address_toward)
                    .map(|address| address.to_string()),
                expected_sender: expected,
                port,
                seconds_playing,
                received_packets: stats.received_packets,
                lost_packets: stats.lost_packets,
                late_packets: stats.late_packets,
                rejected_datagrams: stats.rejected_datagrams,
                last_rejected_sender: stats
                    .last_rejected_sender
                    .map(|address| address.to_string()),
                underruns: stats.underruns,
                underruns_since_last,
                overflow_packets: stats.overflow_packets,
                receive_errors: stats.receive_errors,
                last_error_code: stats.last_error_code,
                paired: stats.paired,
                auth_failures: stats.auth_failures,
                auth_problem: stats.last_auth_failure.map(|failure| failure.as_str()),
                replayed_packets: stats.replayed_packets,
            };
            records.push((node_id.as_str().to_owned(), summary.to_record("summary")));
        }
        for (index, node_id) in worker.output_node_ids().iter().enumerate() {
            let (Some(stats), Some(sender)) = (
                worker.network_send_stats(index),
                worker.network_sender(index),
            ) else {
                continue;
            };
            let summary = network_log::SendSummary {
                node_id: node_id.as_str().to_owned(),
                destination: sender.destination().to_string(),
                local_address: stats.local_address.map(|address| address.to_string()),
                seconds_playing,
                sent_packets: stats.sent_packets,
                dropped_packets: stats.dropped_packets,
                send_errors: stats.send_errors,
                last_error_code: stats.last_error_code,
                paired: stats.paired,
            };
            records.push((node_id.as_str().to_owned(), summary.to_record("summary")));
        }
        let mut written = Vec::with_capacity(records.len());
        for (node_id, record) in records {
            network_log::write(record.clone());
            sampler.remember(&node_id, record.clone());
            written.push(record);
        }
        written
    }

    /// Recompile a running session's saved graph into its attached native
    /// adapter so parameter edits are heard immediately. Returns the adapter
    /// kind that was updated, `None` when no native adapter is attached, or an
    /// error when the change needs a stop and fresh preparation.
    /// Apply saved Network Send/Receive setting changes to a playing
    /// multi-path worker. Their sockets live outside the compiled graph, so
    /// a recompile alone would report success while still using the old
    /// address. Sender address/port and receiver address/buffer change in
    /// place; a receiver port or IP family change opens a new receiver.
    #[cfg(windows)]
    pub(crate) fn reconfigure_running_network_nodes(
        &mut self,
        session_id: &EntityId,
        previous: &Session,
    ) -> Result<usize, ControlError> {
        if self.native_multi_input_worker_session.as_ref() != Some(session_id) {
            return Ok(0);
        }
        let session = self.get_session(session_id)?.clone();
        let mut changed = 0;
        for node in session
            .nodes
            .iter()
            .filter(|node| matches!(node.kind, NodeKind::NetworkSend | NodeKind::NetworkReceive))
        {
            let Some(old) = previous.nodes.iter().find(|old| old.id == node.id) else {
                continue;
            };
            if old.parameters == node.parameters {
                continue;
            }
            let Some(worker) = self.native_multi_input_worker.as_mut() else {
                break;
            };
            // A new pairing key applies in place on both sides (a receiver's
            // port stays bound, so it cannot be reopened beside itself).
            let pairing_changed =
                old.parameters.get("pairingKey") != node.parameters.get("pairingKey");
            let pairing_key = network_pairing_key(node)?;
            if node.kind == NodeKind::NetworkSend {
                let Some(index) = worker
                    .output_node_ids()
                    .iter()
                    .position(|id| *id == node.id)
                else {
                    continue;
                };
                let host = node
                    .parameters
                    .get("host")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let port = node
                    .parameters
                    .get("port")
                    .and_then(Value::as_u64)
                    .and_then(|port| u16::try_from(port).ok())
                    .unwrap_or(audiorouter_domain::DEFAULT_NETWORK_AUDIO_PORT);
                let destination = audiorouter_windows_audio::network_socket_address(host, port).ok_or_else(|| {
                    ControlError::InvalidRequest(format!("enter the IP address of the receiving computer for {} in its Properties", node.name))
                })?;
                if let Some(sender) = worker.network_sender(index) {
                    sender.retarget(destination);
                    if pairing_changed {
                        sender.set_pairing_key(pairing_key);
                    }
                    changed += 1;
                }
                continue;
            }
            let Some(index) = worker.input_node_ids().iter().position(|id| *id == node.id) else {
                continue;
            };
            let sender = node
                .parameters
                .get("sender")
                .and_then(Value::as_str)
                .and_then(|address| address.parse::<std::net::IpAddr>().ok());
            let buffer_ms = node
                .parameters
                .get("bufferMs")
                .and_then(Value::as_f64)
                .unwrap_or(audiorouter_domain::DEFAULT_NETWORK_BUFFER_MS);
            let port_changed = old.parameters.get("port") != node.parameters.get("port");
            let in_place = !port_changed
                && sender.is_some_and(|sender| {
                    worker.network_receiver(index).is_some_and(|receiver| {
                        let reconfigured = receiver.reconfigure(sender, buffer_ms);
                        if reconfigured && pairing_changed {
                            receiver.set_pairing_key(pairing_key.clone());
                        }
                        reconfigured
                    })
                });
            if !in_place {
                let receiver = start_network_receiver(node)?;
                worker
                    .replace_capture(
                        index,
                        audiorouter_windows_audio::MultiInputCaptureSource::Network(receiver),
                    )
                    .map_err(|error| {
                        ControlError::InvalidRequest(format!(
                            "{} could not switch to its new settings: {error:?}",
                            node.name
                        ))
                    })?;
            }
            changed += 1;
        }
        Ok(changed)
    }
}
