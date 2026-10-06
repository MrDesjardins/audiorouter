use super::*;
use std::sync::atomic::{AtomicU64, Ordering};

static UNIQUE: AtomicU64 = AtomicU64::new(0);

fn temp_dir(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "ar-helper-{label}-{}-{}",
        std::process::id(),
        UNIQUE.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

const INF_TEMPLATE: &str = "\
[Version]
Signature=\"$WINDOWS NT$\"
Class=MEDIA
Provider=%ProviderName%
DriverVer = 10/06/2026,VERSION
CatalogFile=AudioRouterVirtual.cat
PnpLockDown=1

[Manufacturer]
%MfgName%=AUDIOROUTERVIRTUAL,NTamd64.10.0...22000

[AUDIOROUTERVIRTUAL.NTamd64.10.0...22000]
%AUDIOROUTERVIRTUAL_SA.DeviceDesc%=AUDIOROUTERVIRTUAL_SA, ROOT\\AudioRouterVirtual ; root device

[Strings]
ProviderName=\"AudioRouter Project\"
";

fn inf_text(version: &str) -> String {
    INF_TEMPLATE.replace("VERSION", version)
}

fn write_package(dir: &Path, version: &str, signed: &str) {
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(dir.join(INF_NAME), inf_text(version)).unwrap();
    std::fs::write(dir.join(SYS_NAME), b"MZ driver").unwrap();
    std::fs::write(dir.join(CAT_NAME), b"0 catalog").unwrap();
    std::fs::write(
        dir.join(PACKAGE_JSON),
        format!(
            "{{\"version\":\"0.1.0\",\"driverVersion\":\"{version}\",\"signed\":\"{signed}\"}}"
        ),
    )
    .unwrap();
}

/// In-memory Windows: one optional device, a driver store, endpoints that
/// follow CableCount, default devices, endpoint descriptions.
struct Fake {
    signer: Signer,
    store: BTreeMap<String, DriverVersion>,
    next_oem: u32,
    device: Option<String>,
    device_version: Option<DriverVersion>,
    dwords: BTreeMap<String, u32>,
    fail_create: bool,
    fail_unstage: bool,
    restart_pending: bool,
    reboot_on_create: bool,
    endpoints_never_appear: bool,
    polls_before_endpoints: u32,
    polls: u32,
    defaults: Vec<(String, String)>,
    defaults_after_device: Option<Vec<(String, String)>>,
    descriptions: BTreeMap<String, String>,
    calls: Vec<String>,
    slept: u64,
}

impl Fake {
    fn new() -> Self {
        Self {
            signer: Signer::Microsoft,
            store: BTreeMap::new(),
            next_oem: 41,
            device: None,
            device_version: None,
            dwords: BTreeMap::new(),
            fail_create: false,
            fail_unstage: false,
            restart_pending: false,
            reboot_on_create: false,
            endpoints_never_appear: false,
            polls_before_endpoints: 0,
            polls: 0,
            defaults: vec![("render.console".into(), "{speakers}".into())],
            defaults_after_device: None,
            descriptions: BTreeMap::new(),
            calls: Vec::new(),
            slept: 0,
        }
    }
    fn count(&self) -> u8 {
        self.dwords
            .get("CableCount")
            .map(|v| *v as u8)
            .unwrap_or(DEFAULT_CABLE_COUNT)
    }
    fn version_of(inf: &Path) -> DriverVersion {
        inf_identity(&std::fs::read_to_string(inf).unwrap()).unwrap()
    }
}

impl Platform for Fake {
    fn verify_package(
        &mut self,
        _package: &PackageInfo,
        _allow_test: bool,
    ) -> Result<Signer, Failure> {
        self.calls.push("verify".into());
        Ok(self.signer.clone())
    }
    fn stage_package(&mut self, inf: &Path) -> Result<String, Failure> {
        let version = Self::version_of(inf);
        // Windows returns the existing oem name for an identical package.
        if let Some((name, _)) = self.store.iter().find(|(_, v)| **v == version) {
            self.calls.push(format!("stage {name} (existing)"));
            return Ok(name.clone());
        }
        let name = format!("oem{}.inf", self.next_oem);
        self.next_oem += 1;
        self.store.insert(name.clone(), version);
        self.calls.push(format!("stage {name}"));
        Ok(name)
    }
    fn unstage_package(&mut self, oem_inf: &str) -> Result<(), Failure> {
        self.calls.push(format!("unstage {oem_inf}"));
        if self.fail_unstage {
            return Err(Failure::windows("in use", 0x8007_0020u32 as i32));
        }
        self.store
            .remove(oem_inf)
            .map(|_| ())
            .ok_or_else(|| Failure::windows("not staged", 0x8007_0002u32 as i32))
    }
    fn find_device(&mut self) -> Result<Option<String>, Failure> {
        Ok(self.device.clone())
    }
    fn create_device(&mut self, inf: &Path) -> Result<(String, bool), Failure> {
        self.calls.push("create".into());
        if self.fail_create {
            return Err(Failure::windows(
                "DIF_REGISTERDEVICE refused",
                0x8007_0005u32 as i32,
            ));
        }
        self.device = Some("ROOT\\MEDIA\\0000".into());
        self.device_version = Some(Self::version_of(inf));
        if let Some(after) = self.defaults_after_device.take() {
            self.defaults = after;
        }
        Ok(("ROOT\\MEDIA\\0000".into(), self.reboot_on_create))
    }
    fn update_device(&mut self, inf: &Path, force: bool) -> Result<bool, Failure> {
        self.calls.push(format!("update force={force}"));
        self.device_version = Some(Self::version_of(inf));
        Ok(false)
    }
    fn remove_device(&mut self, _instance_id: &str) -> Result<bool, Failure> {
        self.calls.push("remove-device".into());
        self.device = None;
        self.device_version = None;
        Ok(false)
    }
    fn read_device_dword(
        &mut self,
        _instance_id: &str,
        name: &str,
    ) -> Result<Option<u32>, Failure> {
        Ok(self.dwords.get(name).copied())
    }
    fn write_device_dword(
        &mut self,
        _instance_id: &str,
        name: &str,
        value: u32,
    ) -> Result<(), Failure> {
        self.calls.push(format!("dword {name}={value}"));
        self.dwords.insert(name.into(), value);
        Ok(())
    }
    fn restart_device(&mut self, _instance_id: &str) -> Result<bool, Failure> {
        self.calls.push("restart".into());
        Ok(self.restart_pending)
    }
    fn device_driver_version(
        &mut self,
        _instance_id: &str,
    ) -> Result<Option<DriverVersion>, Failure> {
        Ok(self.device_version)
    }
    fn default_endpoints(&mut self) -> Vec<(String, String)> {
        self.defaults.clone()
    }
    fn cable_endpoints(&mut self) -> Vec<CableEndpoint> {
        if self.device.is_none() || self.endpoints_never_appear {
            return Vec::new();
        }
        self.polls += 1;
        if self.polls <= self.polls_before_endpoints {
            return Vec::new();
        }
        let mut result = Vec::new();
        for cable in 0..self.count() {
            for direction in [Direction::Input, Direction::Output] {
                let id = format!(
                    "{{0.0.{}.0}}.{cable}",
                    if direction == Direction::Input { 0 } else { 1 }
                );
                let base = interface_name(cable, direction);
                let friendly = match self.descriptions.get(&id) {
                    Some(description) => format!("{description} ({base})"),
                    None => format!("Speakers ({base})"),
                };
                result.push(CableEndpoint {
                    cable,
                    direction,
                    endpoint_id: id,
                    friendly_name: friendly,
                });
            }
        }
        result
    }
    fn set_endpoint_description(
        &mut self,
        endpoint_id: &str,
        description: &str,
    ) -> Result<(), Failure> {
        self.descriptions
            .insert(endpoint_id.into(), description.into());
        Ok(())
    }
    fn query_driver(&mut self) -> Option<(u16, u16, u32, u32)> {
        self.device
            .as_ref()
            .map(|_| (1, 1, 0x3f, u32::from(self.count())))
    }
    fn sleep_ms(&mut self, ms: u64) {
        self.slept += ms;
    }
    fn now_utc(&mut self) -> String {
        "2026-10-06T12:00:00Z".into()
    }
}

struct Env {
    root: PathBuf,
    context: Context,
}

impl Env {
    fn new(allow_test: bool) -> Self {
        let root = temp_dir("env");
        let helper_dir = root.join("app");
        std::fs::create_dir_all(&helper_dir).unwrap();
        write_package(&helper_dir.join("driver"), "0.1.0.0", "microsoft");
        let context = Context {
            state: StateStore::new(root.join("programdata")),
            log_dir: root.join("programdata"),
            helper_dir,
            allow_test_driver: allow_test,
        };
        Self { root, context }
    }
    fn package(&self) -> PathBuf {
        self.context.helper_dir.join("driver")
    }
    fn other_package(&self, version: &str, signed: &str) -> PathBuf {
        let dir = self.root.join(format!("pkg-{version}-{signed}"));
        write_package(&dir, version, signed);
        dir
    }
}

fn args(text: &str) -> Vec<String> {
    text.split_whitespace().map(str::to_owned).collect()
}

#[test]
fn arguments_follow_the_command_contract() {
    let parsed = parse_arguments(&args(
        "install --package C:\\ar\\driver --result C:\\ar\\r.json",
    ))
    .unwrap();
    assert_eq!(
        parsed.request,
        Request::Install {
            package: PathBuf::from("C:\\ar\\driver")
        }
    );
    assert_eq!(parsed.result_path, Some(PathBuf::from("C:\\ar\\r.json")));
    assert_eq!(
        parse_arguments(&args("set-cables --count 8"))
            .unwrap()
            .request,
        Request::SetCables { count: 8 }
    );
    let configure = parse_arguments(&args(
        "configure --name 2=Discord --param MinPeriodFrames=256",
    ))
    .unwrap();
    match configure.request {
        Request::Configure { names, params } => {
            assert_eq!(names.get(&1).map(String::as_str), Some("Discord"));
            assert_eq!(params.get(&Parameter::MinPeriodFrames), Some(&256));
        }
        other => panic!("{other:?}"),
    }
    for bad in [
        "",
        "install",
        "install --package",
        "status --package C:\\x",
        "set-cables",
        "set-cables --count 0",
        "set-cables --count 9",
        "remove --count 2",
        "configure",
        "configure --name 9=X",
        "configure --name 1=",
        "configure --name 1=a(b)",
        "configure --param MinPeriodFrames=63",
        "configure --param MinPeriodFrames=481",
        "configure --param SilenceOnStaleMs=100",
        "configure --param MaxLeaseMs=499",
        "configure --param MinPeriodFrames=400 --param DefaultPeriodFrames=200",
        "configure --name 1=A --name 1=B",
        "install --package a --package b",
        "bogus",
        "status extra",
    ] {
        let parsed = parse_arguments(&args(bad));
        assert!(parsed.is_err(), "accepted: {bad}");
        assert_eq!(parsed.unwrap_err().kind, ExitKind::InvalidRequest, "{bad}");
    }
}

#[test]
fn cable_names_are_bounded_printable_and_parenthesis_free() {
    assert!(validate_cable_name("Discord").is_ok());
    assert!(validate_cable_name(&"x".repeat(48)).is_ok());
    assert!(validate_cable_name("Jeu vidéo").is_ok());
    for bad in [
        "",
        " Lead",
        "Trail ",
        "Tab\tName",
        "a(b",
        "a)b",
        &"x".repeat(49),
    ] {
        assert!(validate_cable_name(bad).is_err(), "{bad:?}");
    }
}

#[test]
fn inf_identity_accepts_only_audiorouter_packages() {
    assert_eq!(
        inf_identity(&inf_text("0.1.0.0")).unwrap(),
        DriverVersion([0, 1, 0, 0])
    );
    for (from, to) in [
        ("Provider=%ProviderName%", "Provider=Contoso"),
        (
            "ProviderName=\"AudioRouter Project\"",
            "ProviderName=\"Contoso\"",
        ),
        ("ROOT\\AudioRouterVirtual ; root device", "ROOT\\Contoso"),
        (
            "CatalogFile=AudioRouterVirtual.cat",
            "CatalogFile=other.cat",
        ),
        ("DriverVer = 10/06/2026,0.1.0.0", "DriverVer = 10/06/2026"),
    ] {
        let text = inf_text("0.1.0.0").replace(from, to);
        assert!(
            inf_identity(&text).is_err(),
            "accepted after {from} -> {to}"
        );
    }
    let swd = inf_text("0.1.0.0") + "%X%=AUDIOROUTERVIRTUAL_SA, SWD\\AudioRouterVirtual\n";
    assert!(
        inf_identity(&swd).is_err(),
        "software-device match accepted"
    );
    // StampInf output can be UTF-16LE with a BOM.
    let mut utf16 = vec![0xFF, 0xFE];
    for unit in inf_text("1.2.3.4").encode_utf16() {
        utf16.extend_from_slice(&unit.to_le_bytes());
    }
    assert_eq!(
        inf_identity(&decode_text(&utf16).unwrap()).unwrap(),
        DriverVersion([1, 2, 3, 4])
    );
}

#[test]
fn inf_identity_accepts_the_real_repository_inf() {
    // The checked-in template and, when present, the last built package INF
    // (StampInf output) must both pass the helper's identity check.
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../drivers/audiorouter-virtual");
    let template = std::fs::read(root.join("Source/Main/AudioRouterVirtual.inx")).unwrap();
    assert_eq!(
        inf_identity(&decode_text(&template).unwrap()).unwrap(),
        DriverVersion([1, 0, 0, 1])
    );
    for platform in ["x64", "ARM64"] {
        if let Ok(built) = std::fs::read(
            root.join(platform)
                .join("Release/package/AudioRouterVirtual.inf"),
        ) {
            assert!(
                inf_identity(&decode_text(&built).unwrap()).is_ok(),
                "{platform} built INF"
            );
        }
    }
}

#[test]
fn driver_versions_parse_and_order_numerically() {
    assert_eq!(
        DriverVersion::parse("0.10.0.0"),
        Some(DriverVersion([0, 10, 0, 0]))
    );
    assert!(DriverVersion::parse("0.10.0.0").unwrap() > DriverVersion::parse("0.9.9.9").unwrap());
    for bad in [
        "",
        "1.2.3",
        "1.2.3.4.5",
        "1.x.3.4",
        "1..3.4",
        "70000.0.0.0",
        "-1.0.0.0",
    ] {
        assert_eq!(DriverVersion::parse(bad), None, "{bad}");
    }
    assert_eq!(DriverVersion([1, 2, 3, 4]).to_string(), "1.2.3.4");
}

#[test]
fn package_inspection_checks_files_manifest_and_signing_label() {
    let env = Env::new(false);
    let package = inspect_package(&env.package()).unwrap();
    assert_eq!(package.version, DriverVersion([0, 1, 0, 0]));
    assert_eq!(package.declared_signing, "microsoft");
    let unsigned = env.other_package("0.1.0.0", "unsigned");
    assert_eq!(
        inspect_package(&unsigned).unwrap_err().kind,
        ExitKind::VerificationFailed
    );
    let mismatch = env.other_package("0.2.0.0", "test");
    std::fs::write(
        mismatch.join(PACKAGE_JSON),
        "{\"driverVersion\":\"0.1.0.0\",\"signed\":\"test\"}",
    )
    .unwrap();
    assert!(inspect_package(&mismatch)
        .unwrap_err()
        .message
        .contains("does not match"));
    let missing_sys = env.other_package("0.3.0.0", "test");
    std::fs::remove_file(missing_sys.join(SYS_NAME)).unwrap();
    assert!(inspect_package(&missing_sys)
        .unwrap_err()
        .message
        .contains("missing"));
}

#[test]
fn package_must_be_the_install_driver_folder_unless_test_mode() {
    let env = Env::new(false);
    let elsewhere = env.other_package("0.1.0.0", "microsoft");
    assert!(resolve_package_dir(&env.package(), &env.context.helper_dir, false).is_ok());
    assert_eq!(
        resolve_package_dir(&elsewhere, &env.context.helper_dir, false)
            .unwrap_err()
            .kind,
        ExitKind::InvalidRequest
    );
    assert!(resolve_package_dir(&elsewhere, &env.context.helper_dir, true).is_ok());
    assert!(
        resolve_package_dir(Path::new("relative\\driver"), &env.context.helper_dir, true).is_err()
    );
}

#[test]
fn signer_rules_follow_9_1() {
    assert!(signer_allowed(&Signer::Microsoft, false).is_ok());
    let test = Signer::Other("AudioRouter Test Driver".into());
    assert_eq!(
        signer_allowed(&test, false).unwrap_err().kind,
        ExitKind::VerificationFailed
    );
    assert!(signer_allowed(&test, true).is_ok());
}

#[test]
fn endpoint_names_are_recognised_through_renames() {
    assert_eq!(
        classify_endpoint_name("AudioRouter Cable A Input"),
        Some((0, Direction::Input))
    );
    assert_eq!(
        classify_endpoint_name("Speakers (AudioRouter Cable H Output)"),
        Some((7, Direction::Output))
    );
    assert_eq!(
        classify_endpoint_name("AudioRouter Discord Output (AudioRouter Cable B Output)"),
        Some((1, Direction::Output))
    );
    for other in [
        "Speakers (Realtek(R) Audio)",
        "CABLE Input (VB-Audio Virtual Cable)",
        "AudioRouter Cable I Input",
        "Speakers (AudioRouter Cable A Input) ",
        "audiorouter cable a input",
        "",
    ] {
        assert_eq!(classify_endpoint_name(other), None, "{other}");
    }
    assert_eq!(
        description_for("Discord", Direction::Input),
        "AudioRouter Discord Input"
    );
}

#[test]
fn cables_match_requires_exactly_the_first_n_in_both_directions() {
    let make = |cable, direction| CableEndpoint {
        cable,
        direction,
        endpoint_id: String::new(),
        friendly_name: String::new(),
    };
    let two = vec![
        make(0, Direction::Input),
        make(0, Direction::Output),
        make(1, Direction::Input),
        make(1, Direction::Output),
    ];
    assert!(cables_match(&two, 2));
    assert!(!cables_match(&two, 3));
    assert!(!cables_match(&two[..3], 2));
    let mut extra = two.clone();
    extra.push(make(2, Direction::Input));
    assert!(
        !cables_match(&extra, 2),
        "a cable above the count must be gone"
    );
    let mut duplicate = two.clone();
    duplicate[3] = make(1, Direction::Input);
    assert!(!cables_match(&duplicate, 2));
}

#[test]
fn status_has_the_five_states() {
    assert_eq!(classify_status(false, false, false, None), "notInstalled");
    assert_eq!(classify_status(true, false, false, None), "needsRepair");
    assert_eq!(
        classify_status(true, true, true, Some((1, 1, 0x20, 2))),
        "installed"
    );
    assert_eq!(
        classify_status(true, true, false, Some((1, 1, 0x20, 2))),
        "needsRepair"
    );
    assert_eq!(classify_status(true, true, true, None), "needsRestart");
    assert_eq!(
        classify_status(true, true, true, Some((2, 0, 0x20, 2))),
        "incompatible"
    );
    assert_eq!(
        classify_status(true, true, true, Some((1, 1, 0, 2))),
        "incompatible"
    );
    assert_eq!(
        classify_status(false, true, true, Some((1, 1, 0x20, 2))),
        "installed",
        "installed without the helper"
    );
}

#[test]
fn install_stages_creates_records_state_and_reports_defaults() {
    let env = Env::new(false);
    let mut fake = Fake::new();
    fake.polls_before_endpoints = 3;
    fake.defaults_after_device = Some(vec![
        ("render.console".into(), "{speakers}".into()),
        ("capture.console".into(), "{cable-a-out}".into()),
    ]);
    let outcome = run(
        &Request::Install {
            package: env.package(),
        },
        &mut fake,
        &env.context,
    );
    assert_eq!(outcome.kind, ExitKind::Success, "{}", outcome.json);
    assert_eq!(outcome.json["oemInf"], "oem41.inf");
    assert_eq!(outcome.json["endpoints"].as_array().unwrap().len(), 4);
    assert_eq!(
        outcome.json["defaultsChanged"],
        serde_json::json!(["capture.console"])
    );
    assert_eq!(fake.slept, 3 * ENDPOINT_POLL_MS);
    let state = env.context.state.load().unwrap().unwrap();
    assert_eq!(
        (state.oem_inf.as_str(), state.package_version),
        ("oem41.inf", DriverVersion([0, 1, 0, 0]))
    );
    assert!(
        std::fs::read_to_string(env.context.log_dir.join("helper.log"))
            .unwrap()
            .contains("install exit=0")
    );

    // Idempotent: a second install of the same version changes nothing.
    fake.calls.clear();
    let again = run(
        &Request::Install {
            package: env.package(),
        },
        &mut fake,
        &env.context,
    );
    assert_eq!(again.kind, ExitKind::Success);
    assert_eq!(again.json["alreadyInstalled"], true);
    assert_eq!(fake.calls, vec!["verify"]);
}

#[test]
fn install_rolls_back_the_package_when_the_device_cannot_be_created() {
    let env = Env::new(false);
    let mut fake = Fake::new();
    fake.fail_create = true;
    let outcome = run(
        &Request::Install {
            package: env.package(),
        },
        &mut fake,
        &env.context,
    );
    assert_eq!(outcome.kind, ExitKind::WindowsRefused);
    assert_eq!(outcome.json["hresult"], "0x80070005");
    assert!(outcome.json["error"]
        .as_str()
        .unwrap()
        .contains("rollback of oem41.inf: removed"));
    assert!(
        fake.store.is_empty(),
        "the staged package must be removed again"
    );
    assert!(env.context.state.load().unwrap().is_none());
}

#[test]
fn install_reports_a_pending_restart_and_a_timeout() {
    let env = Env::new(false);
    let mut fake = Fake::new();
    fake.reboot_on_create = true;
    let outcome = run(
        &Request::Install {
            package: env.package(),
        },
        &mut fake,
        &env.context,
    );
    assert_eq!(outcome.kind, ExitKind::RestartRequired);
    assert_eq!(outcome.kind.code(), 5);

    let env = Env::new(false);
    let mut fake = Fake::new();
    fake.endpoints_never_appear = true;
    let outcome = run(
        &Request::Install {
            package: env.package(),
        },
        &mut fake,
        &env.context,
    );
    assert_eq!(outcome.kind, ExitKind::Timeout);
    assert_eq!(fake.slept, ENDPOINT_WAIT_MS);
    assert!(
        env.context.state.load().unwrap().is_some(),
        "the installed package stays recorded for repair/remove"
    );
}

#[test]
fn test_signed_packages_need_a_test_enabled_helper() {
    let env = Env::new(false);
    let test_package = env.other_package("0.1.0.0", "test");
    let mut fake = Fake::new();
    let refused = run(
        &Request::Install {
            package: test_package.clone(),
        },
        &mut fake,
        &env.context,
    );
    assert_eq!(
        refused.kind,
        ExitKind::InvalidRequest,
        "not the install folder"
    );

    // In the install folder but test-signed: release rules refuse it (exit 2)
    // before any Windows change.
    write_package(&env.package(), "0.1.0.0", "test");
    fake.signer = Signer::Other("AudioRouter Test Driver".into());
    let refused = run(
        &Request::Install {
            package: env.package(),
        },
        &mut fake,
        &env.context,
    );
    assert_eq!(refused.kind, ExitKind::VerificationFailed);
    assert!(
        fake.calls.is_empty(),
        "no Windows call for a refused package: {:?}",
        fake.calls
    );

    // A package declared "microsoft" but signed by someone else is refused too.
    write_package(&env.package(), "0.1.0.0", "microsoft");
    let refused = run(
        &Request::Install {
            package: env.package(),
        },
        &mut fake,
        &env.context,
    );
    assert_eq!(refused.kind, ExitKind::VerificationFailed);
    assert!(fake.store.is_empty());

    let vm = Env::new(true);
    let mut fake = Fake::new();
    fake.signer = Signer::Other("AudioRouter Test Driver".into());
    let package = vm.other_package("0.1.0.0", "test");
    let accepted = run(&Request::Install { package }, &mut fake, &vm.context);
    assert_eq!(accepted.kind, ExitKind::Success, "{}", accepted.json);
}

#[test]
fn update_applies_newer_keeps_same_and_refuses_older() {
    let env = Env::new(true);
    let mut fake = Fake::new();
    run(
        &Request::Install {
            package: env.package(),
        },
        &mut fake,
        &env.context,
    );
    let newer = env.other_package("0.2.0.0", "microsoft");
    fake.calls.clear();
    let outcome = run(&Request::Update { package: newer }, &mut fake, &env.context);
    assert_eq!(outcome.kind, ExitKind::Success, "{}", outcome.json);
    assert_eq!(outcome.json["previousDriverVersion"], "0.1.0.0");
    assert_eq!(outcome.json["oemInf"], "oem42.inf");
    assert_eq!(outcome.json["supersededPackage"], "oem41.inf removed");
    assert!(fake.calls.contains(&"update force=false".to_owned()));
    assert_eq!(
        fake.device.as_deref(),
        Some("ROOT\\MEDIA\\0000"),
        "same device instance"
    );

    let same = env.other_package("0.2.0.0", "test");
    fake.calls.clear();
    let outcome = run(&Request::Update { package: same }, &mut fake, &env.context);
    assert_eq!(outcome.kind, ExitKind::Success);
    assert!(
        !fake.calls.iter().any(|call| call.starts_with("update")),
        "same version is not reinstalled"
    );

    let older = env.other_package("0.1.5.0", "microsoft");
    let outcome = run(&Request::Update { package: older }, &mut fake, &env.context);
    assert_eq!(outcome.kind, ExitKind::InvalidRequest);
    assert!(outcome.json["error"]
        .as_str()
        .unwrap()
        .contains("downgrades"));

    let fresh = Env::new(true);
    let mut empty = Fake::new();
    let outcome = run(
        &Request::Update {
            package: fresh.package(),
        },
        &mut empty,
        &fresh.context,
    );
    assert_eq!(
        outcome.kind,
        ExitKind::InvalidRequest,
        "update needs an installed device"
    );
}

#[test]
fn repair_recreates_a_deleted_device_and_keeps_names() {
    let env = Env::new(false);
    let mut fake = Fake::new();
    run(
        &Request::Install {
            package: env.package(),
        },
        &mut fake,
        &env.context,
    );
    let names = BTreeMap::from([(1_u8, "Discord".to_owned())]);
    let outcome = run(
        &Request::Configure {
            names,
            params: BTreeMap::new(),
        },
        &mut fake,
        &env.context,
    );
    assert_eq!(outcome.kind, ExitKind::Success, "{}", outcome.json);
    // The user deletes the device in Device Manager.
    fake.device = None;
    fake.device_version = None;
    fake.descriptions.clear();
    let outcome = run(
        &Request::Repair {
            package: env.package(),
        },
        &mut fake,
        &env.context,
    );
    assert_eq!(outcome.kind, ExitKind::Success, "{}", outcome.json);
    assert_eq!(outcome.json["command"], "repair");
    assert!(fake.device.is_some());
    assert_eq!(
        env.context
            .state
            .load()
            .unwrap()
            .unwrap()
            .cable_names
            .get(&1)
            .map(String::as_str),
        Some("Discord")
    );

    // Repair on a present device reinstalls the same version (force).
    fake.calls.clear();
    let outcome = run(
        &Request::Repair {
            package: env.package(),
        },
        &mut fake,
        &env.context,
    );
    assert_eq!(outcome.kind, ExitKind::Success);
    assert!(fake.calls.contains(&"update force=true".to_owned()));
}

#[test]
fn set_cables_changes_the_count_and_waits_for_exactly_n() {
    let env = Env::new(false);
    let mut fake = Fake::new();
    run(
        &Request::Install {
            package: env.package(),
        },
        &mut fake,
        &env.context,
    );
    let outcome = run(&Request::SetCables { count: 8 }, &mut fake, &env.context);
    assert_eq!(outcome.kind, ExitKind::Success, "{}", outcome.json);
    assert_eq!(outcome.json["endpoints"].as_array().unwrap().len(), 16);
    assert_eq!(outcome.json["previousCableCount"], 2);
    let outcome = run(&Request::SetCables { count: 3 }, &mut fake, &env.context);
    assert_eq!(outcome.json["endpoints"].as_array().unwrap().len(), 6);
    assert!(fake.calls.contains(&"dword CableCount=3".to_owned()));
    fake.restart_pending = true;
    let outcome = run(&Request::SetCables { count: 2 }, &mut fake, &env.context);
    assert_eq!(outcome.kind, ExitKind::RestartRequired);

    let fresh = Env::new(false);
    let mut empty = Fake::new();
    assert_eq!(
        run(&Request::SetCables { count: 2 }, &mut empty, &fresh.context).kind,
        ExitKind::InvalidRequest
    );
}

#[test]
fn configure_writes_parameters_and_names_without_new_endpoint_ids() {
    let env = Env::new(false);
    let mut fake = Fake::new();
    run(
        &Request::Install {
            package: env.package(),
        },
        &mut fake,
        &env.context,
    );
    let ids_before: Vec<String> = fake
        .cable_endpoints()
        .into_iter()
        .map(|e| e.endpoint_id)
        .collect();
    let names = BTreeMap::from([(1_u8, "Discord".to_owned())]);
    let params = BTreeMap::from([(Parameter::MinPeriodFrames, 256_u32)]);
    let outcome = run(
        &Request::Configure { names, params },
        &mut fake,
        &env.context,
    );
    assert_eq!(outcome.kind, ExitKind::Success, "{}", outcome.json);
    assert_eq!(fake.dwords.get("MinPeriodFrames"), Some(&256));
    assert!(fake.calls.contains(&"restart".to_owned()));
    let endpoints = fake.cable_endpoints();
    assert_eq!(
        endpoints
            .iter()
            .map(|e| e.endpoint_id.clone())
            .collect::<Vec<_>>(),
        ids_before
    );
    let cable_b_input = endpoints
        .iter()
        .find(|e| e.cable == 1 && e.direction == Direction::Input)
        .unwrap();
    assert_eq!(
        cable_b_input.friendly_name,
        "AudioRouter Discord Input (AudioRouter Cable B Input)"
    );
    assert_eq!(
        classify_endpoint_name(&cable_b_input.friendly_name),
        Some((1, Direction::Input))
    );

    // A stored minimum makes a lower default invalid even if given alone.
    let params = BTreeMap::from([(Parameter::DefaultPeriodFrames, 200_u32)]);
    let outcome = run(
        &Request::Configure {
            names: BTreeMap::new(),
            params,
        },
        &mut fake,
        &env.context,
    );
    assert_eq!(outcome.kind, ExitKind::InvalidRequest);
}

#[test]
fn remove_deletes_only_the_recorded_package_and_device() {
    let env = Env::new(false);
    let mut fake = Fake::new();
    fake.store
        .insert("oem7.inf".into(), DriverVersion([9, 9, 9, 9])); // someone else's driver
    run(
        &Request::Install {
            package: env.package(),
        },
        &mut fake,
        &env.context,
    );
    let outcome = run(&Request::Remove, &mut fake, &env.context);
    assert_eq!(outcome.kind, ExitKind::Success, "{}", outcome.json);
    assert_eq!(outcome.json["removedPackage"], "oem41.inf");
    assert!(fake.device.is_none());
    assert_eq!(
        fake.store.keys().collect::<Vec<_>>(),
        vec!["oem7.inf"],
        "other drivers untouched"
    );
    assert!(env.context.state.load().unwrap().is_none());

    let outcome = run(&Request::Remove, &mut fake, &env.context);
    assert_eq!(
        outcome.kind,
        ExitKind::InvalidRequest,
        "no state: refuse to guess what to remove"
    );
    assert!(!fake.calls.iter().any(|c| c == "unstage oem7.inf"));
}

#[test]
fn status_reports_install_state_and_endpoints() {
    let env = Env::new(false);
    let mut fake = Fake::new();
    let outcome = run(&Request::Status, &mut fake, &env.context);
    assert_eq!(outcome.json["state"], "notInstalled");
    run(
        &Request::Install {
            package: env.package(),
        },
        &mut fake,
        &env.context,
    );
    let outcome = run(&Request::Status, &mut fake, &env.context);
    assert_eq!(outcome.json["state"], "installed");
    assert_eq!(outcome.json["protocol"], "1.1");
    assert_eq!(outcome.json["installedDriverVersion"], "0.1.0.0");
}

#[test]
fn state_store_is_atomic_bounded_and_validated() {
    let dir = temp_dir("state");
    let store = StateStore::new(&dir);
    assert!(store.load().unwrap().is_none());
    let state = State {
        schema: STATE_SCHEMA,
        package_version: DriverVersion([0, 1, 0, 0]),
        oem_inf: "oem12.inf".into(),
        device_instance_id: "ROOT\\MEDIA\\0000".into(),
        installed_at: "now".into(),
        cable_names: BTreeMap::from([(0, "Game".into())]),
    };
    store.save(&state).unwrap();
    assert_eq!(store.load().unwrap(), Some(state.clone()));
    assert_eq!(
        std::fs::read_dir(&dir).unwrap().count(),
        1,
        "no temporary file left behind"
    );
    for bad_oem in [
        "oem.inf",
        "x12.inf",
        "oem12.sys",
        "oemAB.inf",
        "oem12345678901.inf",
        "oém1.inf",
    ] {
        let mut bad = state.clone();
        bad.oem_inf = bad_oem.into();
        assert!(store.save(&bad).is_err(), "{bad_oem}");
    }
    std::fs::write(store.path(), "{not json").unwrap();
    assert!(store.load().is_err());
    std::fs::write(store.path(), "x".repeat(MAX_STATE_BYTES as usize + 1)).unwrap();
    assert!(store.load().is_err());
    store.delete().unwrap();
    store.delete().unwrap();
}

#[test]
fn log_rotates_once_at_one_megabyte_and_strips_control_characters() {
    let dir = temp_dir("log");
    std::fs::write(dir.join("helper.log"), vec![b'x'; MAX_LOG_BYTES as usize]).unwrap();
    append_log(&dir, "line\r\nwith control");
    assert_eq!(
        std::fs::metadata(dir.join("helper.log.1")).unwrap().len(),
        MAX_LOG_BYTES
    );
    assert_eq!(
        std::fs::read_to_string(dir.join("helper.log")).unwrap(),
        "linewith control\n"
    );
}

#[test]
fn exit_codes_match_the_spec() {
    let codes: Vec<i32> = [
        ExitKind::Success,
        ExitKind::InvalidRequest,
        ExitKind::VerificationFailed,
        ExitKind::WindowsRefused,
        ExitKind::Timeout,
        ExitKind::RestartRequired,
    ]
    .iter()
    .map(|k| k.code())
    .collect();
    assert_eq!(codes, vec![0, 1, 2, 3, 4, 5]);
}
