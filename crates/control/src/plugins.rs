//! Plug-ins: worker location, bridges, chains, inventory, scanning, state and editors.

use super::*;

pub(crate) fn packaged_plugin_worker_path(
    shell_executable: &std::path::Path,
) -> Option<std::path::PathBuf> {
    let directory = shell_executable.parent()?;
    packaged_plugin_worker_candidates(directory)
        .into_iter()
        .find(|candidate| candidate.is_file())
}

pub(crate) fn packaged_plugin_worker_candidates(
    directory: &std::path::Path,
) -> [std::path::PathBuf; 2] {
    let worker = if cfg!(windows) {
        "audiorouter-plugin-worker.exe"
    } else {
        "audiorouter-plugin-worker"
    };
    [
        directory.join(worker),
        directory.join("resources").join(worker),
    ]
}

pub(crate) fn plugin_worker_unavailable_message(
    shell_executable: Option<&std::path::Path>,
) -> String {
    let worker_name = if cfg!(windows) {
        "audiorouter-plugin-worker.exe"
    } else {
        "audiorouter-plugin-worker"
    };
    let locations = shell_executable
        .and_then(std::path::Path::parent)
        .map(|directory| {
            packaged_plugin_worker_candidates(directory)
                .into_iter()
                .map(|path| format!("`{}`", path.display()))
                .collect::<Vec<_>>()
                .join(" or ")
        })
        .unwrap_or_else(|| "the shell's install directory or its `resources` folder".into());
    format!(
        "Plugin worker executable `{worker_name}` was not found. Checked {locations}. Install or rebuild the complete AudioRouter package so the worker is beside the shell or in `resources`, or set `AUDIOROUTER_PLUGIN_WORKER_PATH` to the worker's full path."
    )
}

pub(crate) fn plugin_worker_path() -> Option<std::path::PathBuf> {
    std::env::var_os("AUDIOROUTER_PLUGIN_WORKER_PATH")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::current_exe()
                .ok()
                .and_then(|executable| packaged_plugin_worker_path(&executable))
        })
}

/// Issuer of native-editor parent capabilities. The key never leaves this
/// process; it is derived once per backend run from process-local entropy.
pub(crate) fn editor_authorization_issuer(
) -> &'static audiorouter_plugin_host::EditorParentAuthorizationIssuer {
    static ISSUER: std::sync::OnceLock<audiorouter_plugin_host::EditorParentAuthorizationIssuer> =
        std::sync::OnceLock::new();
    ISSUER.get_or_init(|| {
        use sha2::Digest;
        let mut digest = sha2::Sha256::new();
        digest.update(std::process::id().to_le_bytes());
        digest.update(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |elapsed| elapsed.as_nanos())
                .to_le_bytes(),
        );
        digest.update((&ISSUER as *const _ as usize).to_le_bytes());
        audiorouter_plugin_host::EditorParentAuthorizationIssuer::from_key(digest.finalize().into())
    })
}

/// What `ControlPlane::prepare_plugin_stages` gathers for one enabled plugin
/// node before the slow part runs off the control thread.
pub(crate) struct PluginStageRequest {
    node_id: EntityId,
    /// The binary path exactly as scanned under `configured_root`.
    scanned_path: std::path::PathBuf,
    configured_root: std::path::PathBuf,
    fingerprint: String,
    channels: usize,
    state: Option<audiorouter_plugin_host::PluginStateAsset>,
    parameters: Vec<audiorouter_plugin_host::ParameterEvent>,
}

/// Re-hash each plugin binary, start its worker process (one per exclusive
/// VST2 chain), restore saved state and start the runtime bridges. Needs no
/// control-plane state, so it can run while audio keeps being serviced. On
/// error the bridges started so far are dropped, which stops their workers.
pub(crate) fn start_plugin_bridges(
    session: &Session,
    worker_executable: &std::path::Path,
    requests: Vec<PluginStageRequest>,
    sample_rate_hz: u32,
) -> Result<Vec<(EntityId, Arc<audiorouter_plugin_host::PluginRuntimeBridge>)>, ControlError> {
    let mut prepared = HashMap::new();
    for request in requests {
        let verified = audiorouter_plugin_host::inspect_binary(
            &request.scanned_path,
            std::slice::from_ref(&request.configured_root),
        )
        .map_err(|error| {
            ControlError::InvalidRequest(format!("plugin revalidation failed: {error:?}"))
        })?;
        if verified.sha256 != request.fingerprint {
            return Err(ControlError::InvalidRequest(
                "plugin fingerprint changed since scan".into(),
            ));
        }
        prepared.insert(
            request.node_id,
            (
                verified,
                request.configured_root,
                request.channels,
                request.state,
                request.parameters,
            ),
        );
    }
    let eligible = prepared
        .iter()
        .filter(|(id, (identity, _, _, _, _))| {
            identity.format == audiorouter_plugin_host::PluginFormat::Vst2
                && session
                    .nodes
                    .iter()
                    .any(|node| &node.id == *id && !node.bypass)
        })
        .map(|(id, (_, _, channels, _, _))| (id.clone(), *channels))
        .collect();
    let mut started = Vec::new();
    for group in plugin_chain_groups(session, &eligible) {
        let members = group
            .iter()
            .map(|id| prepared.get(id).expect("prepared plugin group"))
            .collect::<Vec<_>>();
        let (verified, configured_root, channels, _, _) = members[0];
        let mut worker = if members.len() > 1 {
            let plugins = members.iter().map(|(identity, root, _, _, _)| audiorouter_plugin_host::WorkerChainPlugin {
                path: identity.path.clone(), sha256: identity.sha256.clone(), configured_roots: vec![root.clone()],
            }).collect::<Vec<_>>();
            audiorouter_plugin_host::SupervisedWorkerProcess::spawn_verified_chain(
                worker_executable, &plugins, *channels as u16, sample_rate_hz, Instant::now())
        } else if verified.format == audiorouter_plugin_host::PluginFormat::Vst3 {
            #[cfg(windows)]
            {
                audiorouter_plugin_host::SupervisedWorkerProcess::spawn_verified_native_vst3_with_sample_rate(
                    worker_executable,
                    verified,
                    std::slice::from_ref(configured_root),
                    *channels as u16,
                    sample_rate_hz,
                    Instant::now(),
                )
            }
            #[cfg(not(windows))]
            {
                return Err(ControlError::InvalidRequest(
                    "VST3 worker activation requires Windows".into(),
                ));
            }
        } else {
            audiorouter_plugin_host::SupervisedWorkerProcess::spawn_verified_with_sample_rate(
                worker_executable,
                verified,
                std::slice::from_ref(configured_root),
                *channels as u16,
                sample_rate_hz,
                Instant::now(),
            )
        }
        .map_err(|error| ControlError::InvalidRequest(format!("plugin worker launch failed: {error:?}")))?;
        for (index, (_, _, _, state, _)) in members.iter().enumerate() {
            if let Some(asset) = state {
                if members.len() > 1 {
                    worker
                        .select_instance(index, Instant::now())
                        .map_err(|error| {
                            ControlError::InvalidRequest(format!(
                                "plugin state instance selection failed: {error:?}"
                            ))
                        })?;
                }
                worker
                    .restore_state(asset.clone(), Instant::now())
                    .map_err(|error| {
                        ControlError::InvalidRequest(format!(
                            "plugin state restore failed: {error:?}"
                        ))
                    })?;
            }
        }
        let bridges = if members.len() > 1 {
            audiorouter_plugin_host::PluginRuntimeBridge::start_chain(
                worker,
                *channels,
                audiorouter_engine::PROCESSING_QUANTUM_FRAMES,
                8,
                members.len(),
            )
        } else {
            audiorouter_plugin_host::PluginRuntimeBridge::start(
                worker,
                *channels,
                audiorouter_engine::PROCESSING_QUANTUM_FRAMES,
                8,
            )
            .map(|bridge| vec![bridge])
        }
        .map_err(|error| {
            ControlError::InvalidRequest(format!("plugin runtime bridge failed: {error:?}"))
        })?;
        for ((id, member), bridge) in group.iter().zip(members).zip(bridges) {
            bridge.set_parameters(member.4.clone()).map_err(|error| {
                ControlError::InvalidRequest(format!(
                    "plugin parameter template rejected: {error:?}"
                ))
            })?;
            started.push((id.clone(), bridge));
        }
    }
    Ok(started)
}

/// Group only exclusive identity edges; absorbing a branch or matrix would
/// change which signal an intermediate graph node receives.
pub(crate) fn plugin_chain_groups(
    session: &Session,
    eligible: &HashMap<EntityId, usize>,
) -> Vec<Vec<EntityId>> {
    let mut next = HashMap::new();
    for edge in session.edges.iter().filter(|edge| edge.enabled) {
        let Some(&channels) = eligible.get(&edge.source_node) else {
            continue;
        };
        if eligible.get(&edge.destination_node) != Some(&channels) {
            continue;
        }
        let identity = edge.matrix.len() == channels * channels
            && edge.matrix.iter().enumerate().all(|(index, gain)| {
                *gain
                    == if index / channels == index % channels {
                        1.0
                    } else {
                        0.0
                    }
            });
        let ports_match = session
            .nodes
            .iter()
            .filter(|node| node.id == edge.source_node || node.id == edge.destination_node)
            .all(|node| {
                node.ports
                    .iter()
                    .all(|port| usize::from(port.channels) == channels)
            });
        if identity
            && ports_match
            && session
                .edges
                .iter()
                .filter(|other| other.enabled && other.source_node == edge.source_node)
                .count()
                == 1
            && session
                .edges
                .iter()
                .filter(|other| other.enabled && other.destination_node == edge.destination_node)
                .count()
                == 1
        {
            next.insert(edge.source_node.clone(), edge.destination_node.clone());
        }
    }
    let plugins = session
        .nodes
        .iter()
        .filter(|node| node.enabled && node.kind == NodeKind::Plugin)
        .collect::<Vec<_>>();
    let mut visited = std::collections::HashSet::new();
    let mut groups = Vec::new();
    // Start at heads, independent of saved node order. The second pass covers
    // isolated nodes and defensively bounds invalid cycles before compilation.
    for head in plugins
        .iter()
        .filter(|node| !next.values().any(|id| id == &node.id))
        .chain(plugins.iter())
    {
        if visited.contains(&head.id) {
            continue;
        }
        let mut id = head.id.clone();
        let mut group = Vec::new();
        loop {
            if !visited.insert(id.clone()) {
                break;
            }
            group.push(id.clone());
            if group.len() == audiorouter_plugin_host::MAX_PLUGIN_CHAIN_MEMBERS {
                groups.push(std::mem::take(&mut group));
            }
            let Some(successor) = next.get(&id) else {
                break;
            };
            id = successor.clone();
        }
        if !group.is_empty() {
            groups.push(group);
        }
    }
    groups
}

#[cfg(test)]
mod plugin_chain_group_tests {
    use super::*;
    use audiorouter_domain::{Edge, Node, Port};
    fn graph() -> Session {
        let nodes = ["c", "a", "b", "d"]
            .into_iter()
            .map(|id| Node {
                id: EntityId::new(id),
                name: id.into(),
                kind: NodeKind::Plugin,
                type_version: 1,
                enabled: true,
                bypass: false,
                parameters: Default::default(),
                ports: vec![
                    Port {
                        name: "in".into(),
                        direction: PortDirection::Input,
                        channels: 1,
                    },
                    Port {
                        name: "out".into(),
                        direction: PortDirection::Output,
                        channels: 1,
                    },
                ],
            })
            .collect();
        let edges = [("a", "b"), ("b", "c"), ("c", "d")]
            .into_iter()
            .map(|(source, destination)| Edge {
                id: EntityId::new(format!("{source}-{destination}")),
                source_node: EntityId::new(source),
                source_port: "out".into(),
                destination_node: EntityId::new(destination),
                destination_port: "in".into(),
                matrix: vec![1.0],
                enabled: true,
            })
            .collect();
        Session {
            id: EntityId::new("chain"),
            name: "chain".into(),
            schema_version: 1,
            revision: 0,
            nodes,
            edges,
        }
    }
    fn groups(session: &Session) -> Vec<Vec<String>> {
        let eligible = session
            .nodes
            .iter()
            .filter(|node| node.enabled && !node.bypass && node.kind == NodeKind::Plugin)
            .map(|node| (node.id.clone(), usize::from(node.ports[0].channels)))
            .collect();
        plugin_chain_groups(session, &eligible)
            .into_iter()
            .map(|group| group.into_iter().map(|id| id.as_str().to_owned()).collect())
            .collect()
    }
    #[test]
    fn shared_plugin_groups_follow_signal_order_and_keep_boundaries() {
        let mut session = graph();
        assert_eq!(groups(&session), vec![vec!["a", "b", "c", "d"]]);
        session.edges[1].matrix[0] = 0.5;
        let result = groups(&session);
        assert!(result.contains(&vec!["a".into(), "b".into()]));
        assert!(result.contains(&vec!["c".into(), "d".into()]));
        session.edges[1].matrix[0] = 1.0;
        session.nodes[0].bypass = true;
        assert!(groups(&session).contains(&vec!["a".into(), "b".into()]));
        session.nodes[0].bypass = false;
        session.nodes[0].kind = NodeKind::Gain;
        assert!(groups(&session).contains(&vec!["a".into(), "b".into()]));
    }
    #[test]
    fn shared_plugin_groups_do_not_absorb_fanout_or_channel_conversion() {
        let mut session = graph();
        let mut branch = session.edges[0].clone();
        branch.id = EntityId::new("branch");
        branch.destination_node = EntityId::new("d");
        session.edges.push(branch);
        assert!(groups(&session).contains(&vec!["a".into()]));
        assert!(groups(&session).contains(&vec!["b".into(), "c".into()]));
        session.edges.pop();
        session.nodes[0]
            .ports
            .iter_mut()
            .for_each(|port| port.channels = 2);
        assert!(groups(&session).contains(&vec!["a".into(), "b".into()]));
        assert!(groups(&session).contains(&vec!["c".into()]));
    }
    #[test]
    fn shared_plugin_groups_bound_members_even_for_large_or_cyclic_graphs() {
        let mut session = graph();
        session.nodes.clear();
        session.edges.clear();
        for index in 0..19 {
            let mut node = graph().nodes[0].clone();
            node.id = EntityId::new(format!("p{index}"));
            session.nodes.push(node);
            if index > 0 {
                let mut edge = graph().edges[0].clone();
                edge.id = EntityId::new(format!("e{index}"));
                edge.source_node = EntityId::new(format!("p{}", index - 1));
                edge.destination_node = EntityId::new(format!("p{index}"));
                session.edges.push(edge);
            }
        }
        assert_eq!(
            groups(&session).iter().map(Vec::len).collect::<Vec<_>>(),
            vec![8, 8, 3]
        );
        let mut edge = graph().edges[0].clone();
        edge.source_node = EntityId::new("p18");
        edge.destination_node = EntityId::new("p0");
        session.edges.push(edge);
        assert_eq!(groups(&session).iter().map(Vec::len).sum::<usize>(), 19);
    }
}

/// Whether a remembered scan entry is the plugin at `path`. A node may carry
/// either the scanned path or the canonical binary path (`\\?\C:\...`), so
/// both are accepted without the verbatim prefix and without case. The binary
/// fingerprint is still checked by the caller.
pub(crate) fn scan_entry_matches_path(entry: &Value, path: &str) -> bool {
    fn normalized(path: &str) -> String {
        path.strip_prefix(r"\\?\")
            .unwrap_or(path)
            .to_ascii_lowercase()
    }
    let wanted = normalized(path);
    [
        entry.get("path").and_then(Value::as_str),
        entry
            .get("identity")
            .and_then(|identity| identity.get("binaryPath"))
            .and_then(Value::as_str),
    ]
    .into_iter()
    .flatten()
    .any(|candidate| normalized(candidate) == wanted)
}

#[cfg(test)]
mod plugin_inventory_tests {
    use super::*;

    #[test]
    fn remembered_plugin_scans_survive_a_backend_restart() {
        let unique = format!("{}-{:?}", std::process::id(), std::thread::current().id())
            .replace(['(', ')'], "");
        let database =
            std::env::temp_dir().join(format!("audiorouter-plugin-inventory-{unique}.sqlite"));
        let folder =
            std::env::temp_dir().join(format!("audiorouter-plugin-inventory-folder-{unique}"));
        let _ = std::fs::remove_file(&database);
        std::fs::create_dir_all(&folder).unwrap();
        let directory = folder.to_string_lossy().into_owned();
        {
            let mut plane = ControlPlane::with_storage("first", Storage::open(&database).unwrap());
            plane
                .dispatch_plugins_scan(Some(json!({ "directory": directory })))
                .unwrap();
        }
        let plane = ControlPlane::with_storage("second", Storage::open(&database).unwrap());
        let remembered = plane.remembered_plugin_inventories();
        assert_eq!(remembered.as_array().unwrap().len(), 1);
        assert_eq!(remembered[0]["directory"], json!(directory));
        assert!(remembered[0]["entries"].is_array());
        drop(plane);
        let _ = std::fs::remove_file(&database);
        let _ = std::fs::remove_dir_all(&folder);
    }
}

#[cfg(test)]
mod scan_entry_path_tests {
    use super::*;

    #[test]
    fn scan_entries_match_the_scanned_or_canonical_binary_path() {
        let entry = json!({
            "path": r"C:\Program Files\VSTPlugins\ReaPlugs\reaeq-standalone.dll",
            "identity": { "binaryPath": r"\\?\C:\Program Files\VSTPlugins\ReaPlugs\reaeq-standalone.dll" }
        });
        // The scanned path, the canonical binary path a node may carry, and a
        // different letter case all identify the same plugin.
        assert!(scan_entry_matches_path(
            &entry,
            r"C:\Program Files\VSTPlugins\ReaPlugs\reaeq-standalone.dll"
        ));
        assert!(scan_entry_matches_path(
            &entry,
            r"\\?\C:\Program Files\VSTPlugins\ReaPlugs\reaeq-standalone.dll"
        ));
        assert!(scan_entry_matches_path(
            &entry,
            r"c:\program files\vstplugins\reaplugs\REAEQ-STANDALONE.dll"
        ));
        assert!(!scan_entry_matches_path(
            &entry,
            r"C:\Program Files\VSTPlugins\ReaPlugs\reacomp-standalone.dll"
        ));
    }
}

impl ControlPlane {
    pub(crate) fn remember_plugin_inventory(&mut self, directory: String, result: Value) {
        self.insert_plugin_inventory(directory, result);
        // Best effort: a failed save only means the next restart rescans.
        if let Some(storage) = &self.storage {
            let inventories = self
                .plugin_inventory_order
                .iter()
                .filter_map(|directory| self.plugin_inventories.get(directory).cloned())
                .collect::<Vec<_>>();
            let _ = storage.save_plugin_inventories(&Value::Array(inventories));
        }
    }

    pub(crate) fn insert_plugin_inventory(&mut self, directory: String, result: Value) {
        if !self.plugin_inventories.contains_key(&directory) {
            self.plugin_inventory_order.push_back(directory.clone());
        }
        self.plugin_inventories.insert(directory, result);
        while self.plugin_inventory_order.len() > MAX_PLUGIN_INVENTORY_ROOTS {
            if let Some(oldest) = self.plugin_inventory_order.pop_front() {
                self.plugin_inventories.remove(&oldest);
            }
        }
    }

    /// Reload remembered scan results at startup (metadata only; nothing is
    /// loaded or executed, and each add still re-verifies the binary hash).
    pub(crate) fn restore_plugin_inventories(&mut self) {
        let Some(storage) = &self.storage else { return };
        let Ok(inventories) = storage.load_plugin_inventories() else {
            return;
        };
        for inventory in inventories.into_iter().take(MAX_PLUGIN_INVENTORY_ROOTS) {
            let Some(directory) = inventory
                .get("directory")
                .and_then(Value::as_str)
                .filter(|directory| std::path::Path::new(directory).is_absolute())
                .map(str::to_owned)
            else {
                continue;
            };
            if inventory.get("entries").is_some_and(Value::is_array) {
                self.insert_plugin_inventory(directory, inventory);
            }
        }
    }

    /// Every remembered plugin scan, newest folder last (`plugins.list` with
    /// no directory).
    pub(crate) fn remembered_plugin_inventories(&self) -> Value {
        Value::Array(
            self.plugin_inventory_order
                .iter()
                .filter_map(|directory| self.plugin_inventories.get(directory).cloned())
                .collect(),
        )
    }

    /// Require plugin placeholders to originate from a current explicit scan.
    /// The cached identity is discovery evidence, not permission to execute;
    /// worker launch will revalidate the binary again at its own boundary.
    pub(crate) fn validate_plugin_placeholders(
        &self,
        session: &Session,
    ) -> Result<(), ControlError> {
        for node in session
            .nodes
            .iter()
            .filter(|node| node.kind == NodeKind::Plugin)
        {
            let path = node
                .parameters
                .get("path")
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    ControlError::InvalidRequest("plugin placeholder path is missing".into())
                })?;
            let format = node
                .parameters
                .get("format")
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    ControlError::InvalidRequest("plugin placeholder format is missing".into())
                })?;
            let fingerprint = node
                .parameters
                .get("fingerprint")
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    ControlError::InvalidRequest("plugin placeholder fingerprint is missing".into())
                })?;
            let verified = self.plugin_inventories.values().any(|inventory| {
                inventory
                    .get("entries")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .any(|entry| {
                        scan_entry_matches_path(entry, path)
                            && entry
                                .get("identity")
                                .and_then(|identity| identity.get("format"))
                                .and_then(Value::as_str)
                                == Some(format)
                            && entry
                                .get("identity")
                                .and_then(|identity| identity.get("sha256"))
                                .and_then(Value::as_str)
                                == Some(fingerprint)
                    })
            });
            if !verified {
                return Err(ControlError::InvalidRequest(
                    "plugin placeholder requires a current explicit scan result".into(),
                ));
            }
        }
        Ok(())
    }

    /// Resolve and launch only plugin identities present in the current scan
    /// inventory. The scan lookup and saved state are read here; re-hashing
    /// each binary, starting its worker process and its runtime bridge run on
    /// a helper thread while routes that are already running keep being
    /// serviced (the route being prepared is not published until the caller
    /// compiles it from the returned stages). The bridges are published for
    /// node controls only once every group started.
    pub(crate) fn prepare_plugin_stages(
        &mut self,
        session: &Session,
        sample_rate_hz: u32,
    ) -> Result<HashMap<EntityId, Arc<dyn RealtimePluginProcessor>>, ControlError> {
        let session = audiorouter_engine::prune_unfed_upstream(session);
        let session = session.as_ref();
        if !session
            .nodes
            .iter()
            .any(|node| node.enabled && node.kind == NodeKind::Plugin)
        {
            return Ok(HashMap::new());
        }
        let worker_executable = plugin_worker_path().ok_or_else(|| {
            ControlError::InvalidRequest(plugin_worker_unavailable_message(
                std::env::current_exe().ok().as_deref(),
            ))
        })?;
        let mut requests = Vec::new();
        for node in session
            .nodes
            .iter()
            .filter(|node| node.enabled && node.kind == NodeKind::Plugin)
        {
            let path = node
                .parameters
                .get("path")
                .and_then(Value::as_str)
                .ok_or_else(|| ControlError::InvalidRequest("plugin path is missing".into()))?;
            let fingerprint = node
                .parameters
                .get("fingerprint")
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    ControlError::InvalidRequest("plugin fingerprint is missing".into())
                })?;
            let (entry, root) = self
                .plugin_inventories
                .iter()
                .filter_map(|(root, inventory)| {
                    let entry = inventory
                        .get("entries")
                        .and_then(Value::as_array)?
                        .iter()
                        .find(|entry| scan_entry_matches_path(entry, path))?;
                    let entry_fingerprint = entry
                        .get("identity")
                        .and_then(|identity| identity.get("sha256"))
                        .and_then(Value::as_str)?;
                    (entry_fingerprint == fingerprint).then_some((entry, root))
                })
                .next()
                .ok_or_else(|| {
                    ControlError::InvalidRequest(
                        "plugin worker requires a current matching scan identity".into(),
                    )
                })?;
            // Re-inspect using the path exactly as scanned under its root.
            let scanned_path = entry.get("path").and_then(Value::as_str).unwrap_or(path);
            let channels = node
                .ports
                .iter()
                .find(|port| port.direction == audiorouter_domain::PortDirection::Input)
                .map(|port| usize::from(port.channels))
                .unwrap_or(1);
            // The node's latest capture (editor close, Save state, Stop) wins;
            // a capture from a different plugin binary is ignored. Otherwise
            // the state saved with the route (or imported with it) applies.
            let latest = self
                .storage
                .as_ref()
                .and_then(|storage| {
                    storage
                        .plugin_node_state(session.id.as_str(), node.id.as_str())
                        .ok()
                        .flatten()
                })
                .and_then(|id| self.load_plugin_state_asset(&id, fingerprint).ok());
            let state = match latest {
                Some(state) => Some(state),
                None => node
                    .parameters
                    .get("stateId")
                    .and_then(Value::as_str)
                    .map(|id| self.load_plugin_state_asset(id, fingerprint))
                    .transpose()?,
            };
            let mut parameters = node
                .parameters
                .iter()
                .filter_map(|(name, value)| {
                    Some(audiorouter_plugin_host::ParameterEvent {
                        parameter_id: name.strip_prefix("pluginParameter:")?.parse().ok()?,
                        normalized_value: value.as_f64()? as f32,
                        sample_offset: 0,
                    })
                })
                .collect::<Vec<_>>();
            parameters.sort_by_key(|event| event.parameter_id);
            requests.push(PluginStageRequest {
                node_id: node.id.clone(),
                scanned_path: std::path::PathBuf::from(scanned_path),
                configured_root: std::path::PathBuf::from(root),
                fingerprint: fingerprint.to_owned(),
                channels,
                state,
                parameters,
            });
        }
        let started = self.while_servicing_audio(|| {
            start_plugin_bridges(session, &worker_executable, requests, sample_rate_hz)
        })?;
        // Keep controls for the existing graph intact if any new group fails
        // preparation. Publish per-node handles only once all groups exist.
        if let Ok(mut bridges) = self.plugin_bridges.lock() {
            bridges.retain(|_, bridge| bridge.strong_count() > 0);
            for (id, bridge) in &started {
                bridges.insert((session.id.clone(), id.clone()), Arc::downgrade(bridge));
            }
        }
        Ok(started
            .into_iter()
            .map(|(id, bridge)| (id, bridge as Arc<dyn RealtimePluginProcessor>))
            .collect())
    }

    /// The live runtime bridge of a plugin node in a playing route.
    pub(crate) fn plugin_bridge(
        &self,
        session_id: &EntityId,
        node_id: &EntityId,
    ) -> Result<Arc<audiorouter_plugin_host::PluginRuntimeBridge>, ControlError> {
        self.plugin_bridges
            .lock()
            .ok()
            .and_then(|bridges| {
                bridges
                    .get(&(session_id.clone(), node_id.clone()))
                    .and_then(std::sync::Weak::upgrade)
            })
            .ok_or_else(|| {
                ControlError::InvalidRequest(
                    "the plugin is available while its route is playing".into(),
                )
            })
    }

    /// Read and verify a stored plugin state for the plugin binary it was
    /// captured from.
    pub(crate) fn load_plugin_state_asset(
        &self,
        state_id: &str,
        plugin_sha256: &str,
    ) -> Result<audiorouter_plugin_host::PluginStateAsset, ControlError> {
        let storage = self.storage.as_ref().ok_or_else(|| {
            ControlError::InvalidRequest("plugin state storage is unavailable".into())
        })?;
        let root = storage.plugin_state_directory().ok_or_else(|| {
            ControlError::InvalidRequest("plugin state storage is unavailable".into())
        })?;
        let record = storage
            .list_plugin_states(Some(plugin_sha256))
            .map_err(storage_error)?
            .into_iter()
            .find(|record| record.id == state_id)
            .ok_or_else(|| {
                ControlError::InvalidRequest(
                    "the saved plugin state is missing or belongs to a different plugin binary"
                        .into(),
                )
            })?;
        audiorouter_plugin_host::read_state_asset(
            &root,
            std::path::Path::new(&record.path),
            record.version,
            &record.state_sha256,
        )
        .map_err(|error| {
            ControlError::InvalidRequest(format!("saved plugin state is unreadable: {error:?}"))
        })
    }

    pub(crate) fn dispatch_plugins_scan(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let directory = params
            .as_ref()
            .and_then(|value| value.get("directory"))
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("directory is required".into()))?;
        let root = std::path::Path::new(directory);
        if !root.is_absolute() {
            return Err(ControlError::InvalidRequest(
                "directory path must be absolute".into(),
            ));
        }
        let entries = self
            .while_servicing_audio(|| audiorouter_plugin_host::scan_directory(root))
            .map_err(ControlError::PluginScan)?;
        let result = json!({
            "directory": directory,
            "entries": entries.into_iter().map(|entry| {
                let identity = entry.identity.map(|identity| json!({
                    "path": identity.path,
                    "binaryPath": identity.binary_path,
                    "format": match identity.format {
                        audiorouter_plugin_host::PluginFormat::Vst3 => "vst3",
                        audiorouter_plugin_host::PluginFormat::Vst2 => "vst2",
                        audiorouter_plugin_host::PluginFormat::Unknown => "unknown",
                    },
                    "architecture": match identity.architecture {
                        audiorouter_plugin_host::PeArchitecture::X64 => "x64",
                        audiorouter_plugin_host::PeArchitecture::X86 => "x86",
                        audiorouter_plugin_host::PeArchitecture::Arm64 => "arm64",
                        audiorouter_plugin_host::PeArchitecture::Unknown => "unknown",
                    },
                    "fileBytes": identity.file_bytes,
                    "sha256": identity.sha256,
                    "vendor": identity.metadata.vendor,
                    "version": identity.metadata.version,
                    "classIds": identity.metadata.class_ids,
                    "compatibility": match identity.compatibility() {
                        audiorouter_plugin_host::PluginCompatibility::SupportedVst3X64 => "supportedVst3X64",
                        audiorouter_plugin_host::PluginCompatibility::SupportedVst2X64Gated => "supportedVst2X64Gated",
                        audiorouter_plugin_host::PluginCompatibility::UnsupportedFormat => "unsupportedFormat",
                    }
                }));
                json!({
                    "path": entry.path,
                    "identity": identity,
                    "error": entry.error.as_ref().map(|error| format!("{error:?}")),
                    "errorCode": entry.error.as_ref().map(|error| error.code())
                })
            }).collect::<Vec<_>>()
        });
        self.remember_plugin_inventory(directory.to_owned(), result.clone());
        Ok(result)
    }

    pub(crate) fn dispatch_plugins_list(
        &self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let directory = params
            .as_ref()
            .and_then(|value| value.get("directory"))
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("directory is required".into()))?;
        if !std::path::Path::new(directory).is_absolute() {
            return Err(ControlError::InvalidRequest(
                "directory path must be absolute".into(),
            ));
        }
        Ok(self
            .plugin_inventories
            .get(directory)
            .cloned()
            .unwrap_or_else(|| json!({ "directory": directory, "entries": [] })))
    }

    pub(crate) fn dispatch_plugins_retry(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let params = params.unwrap_or_else(|| json!({}));
        let directory = params
            .get("directory")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("directory is required".into()))?;
        let key = params
            .get("idempotencyKey")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("idempotencyKey is required".into()))?;
        let scoped_key = self.scoped_idempotency_key("plugins.retry", key);
        let hash = Self::request_hash(&json!({ "directory": directory }));
        if let Some(result) = self.lookup_idempotent_result(&scoped_key, &hash)? {
            return Ok(result);
        }
        let result = self.dispatch_plugins_scan(Some(json!({ "directory": directory })))?;
        self.journal_idempotent_result(&scoped_key, "plugins.retry", &hash, &result)?;
        Ok(result)
    }

    pub(crate) fn dispatch_plugins_inspect(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let path = params
            .as_ref()
            .and_then(|value| value.get("path"))
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("path is required".into()))?;
        let candidate = std::path::Path::new(path);
        if !candidate.is_absolute() {
            return Err(ControlError::InvalidRequest("path must be absolute".into()));
        }
        let root = candidate.parent().ok_or_else(|| {
            ControlError::InvalidRequest("path must have a parent directory".into())
        })?;
        let roots = [root.to_path_buf()];
        let inspected = self
            .while_servicing_audio(|| audiorouter_plugin_host::inspect_binary(candidate, &roots));
        let result = match inspected {
            Ok(identity) => json!({
                "path": path,
                "identity": {
                    "path": identity.path,
                    "binaryPath": identity.binary_path,
                    "format": match identity.format {
                        audiorouter_plugin_host::PluginFormat::Vst3 => "vst3",
                        audiorouter_plugin_host::PluginFormat::Vst2 => "vst2",
                        audiorouter_plugin_host::PluginFormat::Unknown => "unknown",
                    },
                    "architecture": match identity.architecture {
                        audiorouter_plugin_host::PeArchitecture::X64 => "x64",
                        audiorouter_plugin_host::PeArchitecture::X86 => "x86",
                        audiorouter_plugin_host::PeArchitecture::Arm64 => "arm64",
                        audiorouter_plugin_host::PeArchitecture::Unknown => "unknown",
                    },
                    "fileBytes": identity.file_bytes,
                    "sha256": identity.sha256,
                    "vendor": identity.metadata.vendor,
                    "version": identity.metadata.version,
                    "classIds": identity.metadata.class_ids,
                    "compatibility": match identity.compatibility() {
                        audiorouter_plugin_host::PluginCompatibility::SupportedVst3X64 => "supportedVst3X64",
                        audiorouter_plugin_host::PluginCompatibility::SupportedVst2X64Gated => "supportedVst2X64Gated",
                        audiorouter_plugin_host::PluginCompatibility::UnsupportedFormat => "unsupportedFormat",
                    }
                },
                "error": null,
                "errorCode": null
            }),
            Err(error) => json!({
                "path": path,
                "identity": null,
                "error": format!("{error:?}"),
                "errorCode": error.code()
            }),
        };
        Ok(result)
    }

    pub(crate) fn scanned_plugin_identity(
        &self,
        path: &str,
    ) -> Result<(audiorouter_plugin_host::PluginIdentity, std::path::PathBuf), ControlError> {
        let (root, fingerprint, scanned_path) = self
            .plugin_inventories
            .iter()
            .filter_map(|(root, inventory)| {
                let entry = inventory
                    .get("entries")
                    .and_then(Value::as_array)?
                    .iter()
                    .find(|entry| scan_entry_matches_path(entry, path))?;
                let fingerprint = entry
                    .get("identity")
                    .and_then(|identity| identity.get("sha256"))
                    .and_then(Value::as_str)?;
                let scanned_path = entry.get("path").and_then(Value::as_str)?;
                Some((root, fingerprint, scanned_path))
            })
            .next()
            .ok_or_else(|| {
                ControlError::InvalidRequest(
                    "plugin parameters require a current explicit scan result".into(),
                )
            })?;
        let root = std::path::PathBuf::from(root);
        // Re-inspect using the path exactly as scanned under its root.
        let identity = audiorouter_plugin_host::inspect_binary(
            std::path::Path::new(scanned_path),
            std::slice::from_ref(&root),
        )
        .map_err(|error| {
            ControlError::InvalidRequest(format!("plugin revalidation failed: {error:?}"))
        })?;
        if identity.sha256 != fingerprint {
            return Err(ControlError::InvalidRequest(
                "plugin fingerprint changed since scan".into(),
            ));
        }
        if identity.compatibility()
            == audiorouter_plugin_host::PluginCompatibility::UnsupportedFormat
        {
            return Err(ControlError::InvalidRequest(
                "plugin format or architecture is unsupported".into(),
            ));
        }
        Ok((identity, root))
    }

    pub(crate) fn plugin_node_request(
        params: &Option<Value>,
    ) -> Result<(EntityId, EntityId), ControlError> {
        let text = |name: &str| {
            params
                .as_ref()
                .and_then(|params| params.get(name))
                .and_then(Value::as_str)
                .filter(|value| {
                    !value.is_empty() && value.len() <= audiorouter_domain::MAX_ENTITY_ID_BYTES
                })
                .map(EntityId::new)
                .ok_or_else(|| ControlError::InvalidRequest(format!("{name} is required")))
        };
        Ok((text("sessionId")?, text("nodeId")?))
    }

    /// Capture a playing plugin's state from its processing instance and
    /// store it (PLUG-04: versioned, size-limited, hashed). The caller sets
    /// the returned `stateId` on the node so the next start restores it.
    pub(crate) fn dispatch_plugins_save_state(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let (session_id, node_id) = Self::plugin_node_request(&params)?;
        let (state_id, size_bytes) = self.capture_plugin_state(&session_id, &node_id, false)?;
        Ok(
            json!({ "sessionId": session_id, "nodeId": node_id, "stateId": state_id, "sizeBytes": size_bytes }),
        )
    }

    /// Capture every playing plugin of a session (before Stop) so the next
    /// Play restores what the user set in the plugins' editors. Best effort:
    /// a plugin that cannot report its state keeps its previous capture.
    pub(crate) fn capture_playing_plugin_states(&mut self, session_id: &EntityId) {
        let nodes = self
            .plugin_bridges
            .lock()
            .map(|bridges| {
                bridges
                    .iter()
                    .filter(|((session, _), bridge)| {
                        session == session_id && bridge.strong_count() > 0
                    })
                    .map(|((_, node), _)| node.clone())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        for node_id in nodes {
            if let Err(error) = self.capture_plugin_state(session_id, &node_id, true) {
                eprintln!(
                    "AudioRouter plugin state capture skipped for {}: {error:?}",
                    node_id.as_str()
                );
            }
        }
    }

    /// Store a playing plugin's current state (PLUG-04: versioned,
    /// size-limited, hashed) and record it as the node's latest state, which
    /// the next Play restores. An `automatic` capture replaces the node's
    /// previous automatic capture so they never accumulate; states from an
    /// explicit Save state are kept.
    pub(crate) fn capture_plugin_state(
        &mut self,
        session_id: &EntityId,
        node_id: &EntityId,
        automatic: bool,
    ) -> Result<(String, usize), ControlError> {
        let (session_id, node_id) = (session_id.clone(), node_id.clone());
        let fingerprint = self
            .get_session(&session_id)?
            .nodes
            .iter()
            .find(|node| node.id == node_id && node.kind == NodeKind::Plugin)
            .and_then(|node| {
                node.parameters
                    .get("fingerprint")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            })
            .ok_or_else(|| {
                ControlError::InvalidRequest("nodeId is not a plugin node in this session".into())
            })?;
        let asset = self
            .plugin_bridge(&session_id, &node_id)?
            .save_state()
            .map_err(|error| {
                ControlError::InvalidRequest(format!("plugin state capture failed: {error}"))
            })?;
        let storage = self.storage.as_ref().ok_or_else(|| {
            ControlError::InvalidRequest("plugin state storage is unavailable".into())
        })?;
        let root = storage.plugin_state_directory().ok_or_else(|| {
            ControlError::InvalidRequest("plugin state storage is unavailable".into())
        })?;
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_nanos());
        let prefix = if automatic {
            AUTOMATIC_PLUGIN_STATE_PREFIX
        } else {
            "plugin-state-"
        };
        let state_id = format!("{prefix}{}-{nanos}", &asset.sha256[..16]);
        let path = audiorouter_plugin_host::write_state_asset(&root, &state_id, &asset).map_err(
            |error| ControlError::InvalidRequest(format!("plugin state write failed: {error:?}")),
        )?;
        storage
            .save_plugin_state(&audiorouter_storage::PluginStateRecord {
                id: state_id.clone(),
                plugin_id: fingerprint.clone(),
                plugin_sha256: fingerprint,
                version: asset.version,
                path: path.to_string_lossy().into_owned(),
                state_sha256: asset.sha256.clone(),
                size_bytes: asset.bytes.len() as u64,
            })
            .map_err(storage_error)?;
        let replaced = storage
            .set_plugin_node_state(session_id.as_str(), node_id.as_str(), &state_id)
            .map_err(storage_error)?;
        // Drop the automatic capture this one replaces, unless another node
        // (for example a duplicated session) still restores it.
        if let Some(previous) =
            replaced.filter(|previous| previous.starts_with(AUTOMATIC_PLUGIN_STATE_PREFIX))
        {
            if !storage
                .plugin_node_state_in_use(&previous)
                .map_err(storage_error)?
            {
                let _ = storage.remove_plugin_state(&previous);
                let _ = std::fs::remove_file(root.join(format!("{previous}.bin")));
            }
        }
        Ok((state_id, asset.bytes.len()))
    }

    /// Open or close a playing plugin's native editor. The parent window must
    /// exist and belong to `ownerProcessId` (the worker checks it again).
    pub(crate) fn dispatch_plugins_editor(
        &mut self,
        params: Option<Value>,
        open: bool,
    ) -> Result<Value, ControlError> {
        let (session_id, node_id) = Self::plugin_node_request(&params)?;
        let bridge = self.plugin_bridge(&session_id, &node_id)?;
        if open {
            let number = |name: &str| {
                params
                    .as_ref()
                    .and_then(|params| params.get(name))
                    .and_then(Value::as_u64)
                    .filter(|value| *value > 0)
            };
            let parent_window = number("parentWindow")
                .ok_or_else(|| ControlError::InvalidRequest("parentWindow is required".into()))?;
            let owner_process_id = number("ownerProcessId")
                .and_then(|value| u32::try_from(value).ok())
                .ok_or_else(|| ControlError::InvalidRequest("ownerProcessId is required".into()))?;
            let authorization = editor_authorization_issuer()
                .issue(parent_window, owner_process_id)
                .map_err(|error| {
                    ControlError::InvalidRequest(format!("editor authorization failed: {error:?}"))
                })?;
            bridge.open_editor(authorization).map_err(|error| {
                ControlError::InvalidRequest(if error.contains("editorUnavailable") || error.contains("UnsupportedFeature") {
                    "this plugin has no editor window AudioRouter can open (VST3 editors are not supported yet); use its parameters in Properties".into()
                } else {
                    format!("plugin editor failed to open: {error}")
                })
            })?;
        } else {
            bridge.close_editor().map_err(|error| {
                ControlError::InvalidRequest(format!("plugin editor failed to close: {error}"))
            })?;
            drop(bridge);
            // Keep what the user set in the editor for the next Play.
            if let Err(error) = self.capture_plugin_state(&session_id, &node_id, true) {
                eprintln!("AudioRouter plugin state capture after editor close failed: {error:?}");
            }
        }
        Ok(
            json!({ "sessionId": session_id, "nodeId": node_id, "state": if open { "open" } else { "closed" } }),
        )
    }

    pub(crate) fn dispatch_plugins_parameters(
        &self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let path = params
            .as_ref()
            .and_then(|value| value.get("path"))
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("path is required".into()))?;
        let (identity, root) = self.scanned_plugin_identity(path)?;
        let executable = plugin_worker_path().ok_or_else(|| {
            ControlError::InvalidRequest(plugin_worker_unavailable_message(
                std::env::current_exe().ok().as_deref(),
            ))
        })?;
        let worker = if identity.format == audiorouter_plugin_host::PluginFormat::Vst3 {
            #[cfg(windows)]
            {
                audiorouter_plugin_host::SupervisedWorkerProcess::spawn_verified_native_vst3_with_sample_rate(
                    &executable,
                    &identity,
                    std::slice::from_ref(&root),
                    1,
                    48_000,
                    Instant::now(),
                )
            }
            #[cfg(not(windows))]
            {
                return Err(ControlError::InvalidRequest(
                    "VST3 parameter discovery requires Windows".into(),
                ));
            }
        } else {
            audiorouter_plugin_host::SupervisedWorkerProcess::spawn_verified_with_sample_rate(
                &executable,
                &identity,
                std::slice::from_ref(&root),
                1,
                48_000,
                Instant::now(),
            )
        }
        .map_err(|error| ControlError::InvalidRequest(format!("plugin worker launch failed: {error:?}")))?;
        let mut worker = worker;
        let parameters = worker
            .describe_parameters(Instant::now())
            .map_err(|error| {
                ControlError::InvalidRequest(format!(
                    "plugin parameter discovery failed: {error:?}"
                ))
            })?;
        let _ = worker.shutdown_with_timeout(Duration::from_secs(2));
        Ok(json!({
            "path": path,
            "sha256": identity.sha256,
            "format": match identity.format {
                audiorouter_plugin_host::PluginFormat::Vst2 => "vst2",
                audiorouter_plugin_host::PluginFormat::Vst3 => "vst3",
                audiorouter_plugin_host::PluginFormat::Unknown => "unknown",
            },
            "parameters": parameters.into_iter().map(|parameter| json!({
                "parameterId": parameter.parameter_id,
                "title": parameter.title,
                "defaultValue": parameter.default_value,
                "minimum": parameter.minimum,
                "maximum": parameter.maximum,
            })).collect::<Vec<_>>()
        }))
    }
}
