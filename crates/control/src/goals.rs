//! Starting routes for a person's goal ("clean up my mic"). A goal becomes an
//! ordinary stopped session with inspectable tools (UI-01, UI-09). It never
//! chooses a device, starts audio, arms a recorder or sends a microphone to
//! headphones. An application source needs a real application, so without one
//! that source is left out and named in `stillToChoose`.

use super::simple::{connect, default_parameters, node_template};
use super::*;
use audiorouter_domain::{Node, Port};

/// A source that only exists once the person names an application.
#[derive(Clone, Copy)]
struct AppSlot {
    /// Name of the source on the canvas.
    name: &'static str,
    /// The plain question asked before the route is made.
    prompt: &'static str,
}

/// One tool in a starting route: its ID in the route, kind and name.
type Tool = (&'static str, NodeKind, &'static str);

struct Goal {
    id: &'static str,
    title: &'static str,
    summary: &'static str,
    /// Plain list of what the route adds, shown before choosing.
    adds: &'static [&'static str],
    /// Also offered on the first screen, rather than under "More".
    main: bool,
    application: Option<AppSlot>,
    tools: &'static [Tool],
    /// Connections as (from, to) route IDs. Those naming the application
    /// source (`APP`) are left out when no application was given.
    links: &'static [(&'static str, &'static str)],
    /// What a person must still pick, whatever else was given.
    choose: &'static [&'static str],
}

const APP: &str = "application";

const CLEANUP: [Tool; 4] = [
    ("microphone", NodeKind::PhysicalInput, "Microphone"),
    ("denoise", NodeKind::SpeechDenoise, "Speech Denoise"),
    ("gate", NodeKind::Gate, "Gate"),
    ("compressor", NodeKind::Compressor, "Compressor"),
];

const GOALS: &[Goal] = &[
    Goal {
        id: "clean-mic",
        title: "Clean up my mic",
        summary: "A clearer voice for calls and games.",
        adds: &[
            "Input device",
            "Speech Denoise",
            "Gate",
            "Compressor",
            "Output device",
        ],
        main: true,
        application: None,
        tools: &[
            CLEANUP[0],
            CLEANUP[1],
            CLEANUP[2],
            CLEANUP[3],
            (
                "voice-out",
                NodeKind::PhysicalOutput,
                "Where your voice goes",
            ),
        ],
        links: &[
            ("microphone", "denoise"),
            ("denoise", "gate"),
            ("gate", "compressor"),
            ("compressor", "voice-out"),
        ],
        choose: &[
            "your microphone",
            "where your voice goes, such as a virtual cable",
        ],
    },
    Goal {
        id: "game-discord",
        title: "Game and voice for Discord",
        summary: "Your voice goes to Discord, and the game stays out of it.",
        adds: &[
            "Input device",
            "voice cleanup",
            "Application source",
            "Mixer",
            "Output device",
        ],
        main: true,
        application: Some(AppSlot {
            name: "Game",
            prompt: "Which game should play in your headphones?",
        }),
        tools: &[
            CLEANUP[0],
            CLEANUP[1],
            CLEANUP[2],
            CLEANUP[3],
            ("voice-out", NodeKind::PhysicalOutput, "Voice to Discord"),
            ("game-mix", NodeKind::Mixer, "Game mix"),
            ("headphones", NodeKind::PhysicalOutput, "Headphones"),
        ],
        links: &[
            ("microphone", "denoise"),
            ("denoise", "gate"),
            ("gate", "compressor"),
            ("compressor", "voice-out"),
            (APP, "game-mix"),
            ("game-mix", "headphones"),
        ],
        choose: &[
            "your microphone",
            "the virtual cable Discord listens to",
            "your headphones",
        ],
    },
    Goal {
        id: "podcast",
        title: "Record a podcast",
        summary: "Your mic and a guest's app, saved to a file.",
        adds: &["Input device", "Application source", "Mixer", "Recorder"],
        main: true,
        application: Some(AppSlot {
            name: "Guest",
            prompt: "Which app is your guest's voice in?",
        }),
        tools: &[
            ("microphone", NodeKind::PhysicalInput, "Microphone"),
            ("mix", NodeKind::Mixer, "Podcast mix"),
            ("recorder", NodeKind::Recorder, "Recorder"),
        ],
        links: &[("microphone", "mix"), (APP, "mix"), ("mix", "recorder")],
        choose: &["your microphone"],
    },
    Goal {
        id: "empty",
        title: "Start empty",
        summary: "An empty canvas. Add tools yourself.",
        adds: &[],
        main: true,
        application: None,
        tools: &[],
        links: &[],
        choose: &[],
    },
    Goal {
        id: "record-app",
        title: "Record one app",
        summary: "One app's sound saved to a file, and nothing else.",
        adds: &["Application source", "Recorder"],
        main: false,
        application: Some(AppSlot {
            name: "App",
            prompt: "Which app do you want to record?",
        }),
        tools: &[("recorder", NodeKind::Recorder, "Recorder")],
        links: &[(APP, "recorder")],
        choose: &[],
    },
    Goal {
        id: "mix-minus",
        title: "Talk to a guest without echo",
        summary: "Your guest hears you but not themselves. You hear your guest.",
        adds: &[
            "Input device",
            "Application source",
            "Mixer",
            "Output device",
        ],
        main: false,
        application: Some(AppSlot {
            name: "Guest",
            prompt: "Which app is your guest's voice in?",
        }),
        tools: &[
            ("microphone", NodeKind::PhysicalInput, "Microphone"),
            ("guest-mix", NodeKind::Mixer, "Mix for your guest"),
            ("to-guest", NodeKind::PhysicalOutput, "To your guest"),
            ("your-mix", NodeKind::Mixer, "What you hear"),
            ("headphones", NodeKind::PhysicalOutput, "Headphones"),
        ],
        links: &[
            ("microphone", "guest-mix"),
            ("guest-mix", "to-guest"),
            (APP, "your-mix"),
            ("your-mix", "headphones"),
        ],
        choose: &[
            "your microphone",
            "the virtual cable your guest's app listens to",
            "your headphones",
        ],
    },
];

fn goal_json(goal: &Goal) -> Value {
    json!({
        "id": goal.id,
        "title": goal.title,
        "summary": goal.summary,
        "adds": goal.adds,
        "main": goal.main,
        "applicationPrompt": goal.application.map(|slot| slot.prompt),
        "stillToChoose": goal.choose,
    })
}

/// The application a goal's source should capture: every verified instance of
/// one executable, so the route keeps working when the app restarts.
fn application_node(slot: AppSlot, application: &Value) -> Result<Node, ControlError> {
    let executable = application
        .get("executable")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            ControlError::InvalidRequest(
                "application.executable is required; see list_applications".into(),
            )
        })?;
    let mut parameters = serde_json::Map::new();
    parameters.insert("executable".into(), json!(executable));
    parameters.insert("processPolicy".into(), json!("allVerifiedInstances"));
    if let Some(path) = application.get("executablePath").and_then(Value::as_str) {
        parameters.insert("executablePath".into(), json!(path));
    }
    Ok(Node {
        id: EntityId::new(APP),
        kind: NodeKind::ApplicationCapture,
        type_version: 1,
        name: slot.name.into(),
        enabled: true,
        bypass: false,
        parameters,
        ports: vec![Port {
            name: "out".into(),
            direction: PortDirection::Output,
            channels: 2,
        }],
    })
}

/// Build the stopped session for a goal. Every kind used has a template; a
/// missing one is a programming error reported as a refused request.
fn goal_session(
    goal: &Goal,
    id: EntityId,
    name: String,
    application: Option<&Value>,
) -> Result<Session, ControlError> {
    let mut session = Session {
        id,
        name,
        schema_version: 1,
        revision: 0,
        nodes: Vec::new(),
        edges: Vec::new(),
    };
    if let (Some(slot), Some(application)) = (goal.application, application) {
        session.nodes.push(application_node(slot, application)?);
    }
    for (tool_id, kind, tool_name) in goal.tools {
        let (_, ports, _) = node_template(*kind).ok_or_else(|| {
            ControlError::InvalidRequest(format!("goal tool \"{tool_id}\" cannot be added by kind"))
        })?;
        session.nodes.push(Node {
            id: EntityId::new(*tool_id),
            kind: *kind,
            type_version: 1,
            name: (*tool_name).into(),
            enabled: true,
            bypass: false,
            parameters: default_parameters(*kind),
            ports,
        });
    }
    for (from, to) in goal.links {
        if *from == APP && application.is_none() {
            continue;
        }
        connect(&mut session, &EntityId::new(*from), &EntityId::new(*to))?;
    }
    Ok(session)
}

impl ControlPlane {
    pub(crate) fn dispatch_session_goals(
        &mut self,
        _params: Option<Value>,
    ) -> Result<Value, ControlError> {
        Ok(json!({ "goals": GOALS.iter().map(goal_json).collect::<Vec<_>>() }))
    }

    pub(crate) fn dispatch_session_create_from_goal(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let params = params.unwrap_or_else(|| json!({}));
        let key = params
            .get("idempotencyKey")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("idempotencyKey is required".into()))?;
        let goal_id = params
            .get("goal")
            .and_then(Value::as_str)
            .ok_or_else(|| ControlError::InvalidRequest("goal is required".into()))?;
        let goal = GOALS
            .iter()
            .find(|goal| goal.id == goal_id)
            .ok_or_else(|| {
                ControlError::InvalidRequest(format!(
                    "unknown goal \"{goal_id}\"; choose one of: {}",
                    GOALS
                        .iter()
                        .map(|goal| goal.id)
                        .collect::<Vec<_>>()
                        .join(", ")
                ))
            })?;
        let application = params.get("application").filter(|value| !value.is_null());
        if application.is_some() && goal.application.is_none() {
            return Err(ControlError::InvalidRequest(format!(
                "the goal \"{goal_id}\" has no application source"
            )));
        }
        let scoped = self.scoped_idempotency_key("sessions.createFromGoal", key);
        let hash = Self::request_hash(&json!({
            "goal": goal_id,
            "name": params.get("name"),
            "application": application,
        }));
        if let Some(previous) = self.lookup_idempotent_result(&scoped, &hash)? {
            return Ok(previous);
        }
        let name = params
            .get("name")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map_or_else(|| goal.title.to_owned(), str::to_owned);
        let mut suffix = 1;
        let id = loop {
            let id = EntityId::new(format!("goal-{goal_id}-{suffix}"));
            let taken = self.store.session(&id).is_some()
                || self
                    .storage
                    .as_ref()
                    .is_some_and(|storage| matches!(storage.load_session(&id), Ok(Some(_))));
            if !taken {
                break id;
            }
            suffix += 1;
        };
        let session = goal_session(goal, id, name, application)?;
        let created = self.create_session(session)?;
        let mut still_to_choose: Vec<String> =
            goal.choose.iter().map(|text| (*text).to_owned()).collect();
        if let (Some(slot), None) = (goal.application, application) {
            still_to_choose.push(format!("the app for \"{}\"", slot.name));
        }
        let result = json!({
            "session": created["session"],
            "state": "stopped",
            "stillToChoose": still_to_choose,
        });
        self.journal_idempotent_result(&scoped, "sessions.createFromGoal", &hash, &result)?;
        Ok(result)
    }
}

#[cfg(test)]
#[path = "goals_tests.rs"]
mod tests;
