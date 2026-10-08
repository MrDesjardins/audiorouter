//! Windows implementation of [`Platform`]: SetupAPI for the driver store and
//! the root device, WinTrust for catalog verification, MMDevice for
//! endpoints. Every `unsafe` block states its invariants. Only the
//! `ROOT\AudioRouterVirtual` device and the exact `oem*.inf` passed in are
//! ever changed; no other driver or device is touched (17 §9.2).

use crate::{
    classify_endpoint_properties, is_oem_inf_name, CableEndpoint, DriverVersion, Failure,
    PackageInfo, Platform, Signer, HARDWARE_ID, MICROSOFT_DRIVER_SIGNER,
};
use std::path::Path;
use windows::core::{PCWSTR, PWSTR};
use windows::Win32::Devices::DeviceAndDriverInstallation::*;
use windows::Win32::Foundation::{HANDLE, HWND};
use windows::Win32::System::Registry::{
    RegCloseKey, RegQueryValueExW, RegSetValueExW, HKEY, KEY_READ, KEY_SET_VALUE, REG_DWORD,
    REG_SZ, REG_VALUE_TYPE,
};

pub struct WindowsPlatform;

fn select_active_cable_endpoints(
    display: Vec<audiorouter_windows_audio::EndpointDisplayInfo>,
    states: &[audiorouter_windows_audio::EndpointStateInfo],
) -> Vec<CableEndpoint> {
    use audiorouter_windows_audio::{EndpointDirection, EndpointState};
    display
        .into_iter()
        .filter_map(|info| {
            let (cable, direction) =
                classify_endpoint_properties(&info.name, &info.device_description)?;
            let expected_flow = match direction {
                crate::Direction::Input => EndpointDirection::Render,
                crate::Direction::Output => EndpointDirection::Capture,
            };
            if info.direction != expected_flow
                || !states.iter().any(|state| {
                    state.id == info.id
                        && state.direction == expected_flow
                        && state.state == EndpointState::Active
                })
            {
                return None;
            }
            Some(CableEndpoint {
                cable,
                direction,
                endpoint_id: info.id,
                friendly_name: info.name,
            })
        })
        .collect()
}

impl WindowsPlatform {
    pub fn new() -> Self {
        Self
    }
}

impl Default for WindowsPlatform {
    fn default() -> Self {
        Self::new()
    }
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

fn wide_path(path: &Path) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    path.as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

fn refused(step: &str, error: windows::core::Error) -> Failure {
    Failure::windows(format!("{step}: {}", error.message()), error.code().0)
}

fn win32_refused(step: &str, code: windows::Win32::Foundation::WIN32_ERROR) -> Failure {
    let hresult = code.to_hresult();
    Failure::windows(
        format!("{step}: {}", windows::core::Error::from(hresult).message()),
        hresult.0,
    )
}

/// Owns an HDEVINFO and destroys it on drop.
struct DeviceInfoSet(HDEVINFO);

impl Drop for DeviceInfoSet {
    fn drop(&mut self) {
        // SAFETY: the set was returned by SetupDi* and is destroyed once.
        unsafe {
            let _ = SetupDiDestroyDeviceInfoList(self.0);
        }
    }
}

fn devinfo_data() -> SP_DEVINFO_DATA {
    SP_DEVINFO_DATA {
        cbSize: std::mem::size_of::<SP_DEVINFO_DATA>() as u32,
        ..Default::default()
    }
}

/// Open one device by instance ID in a fresh set.
fn open_device(instance_id: &str) -> Result<(DeviceInfoSet, SP_DEVINFO_DATA), Failure> {
    // SAFETY: a new empty set; the instance ID buffer is NUL-terminated and
    // outlives the call; `data` is a correctly sized SP_DEVINFO_DATA.
    unsafe {
        let set = DeviceInfoSet(
            SetupDiCreateDeviceInfoList(None, None).map_err(|e| refused("open device list", e))?,
        );
        let id = wide(instance_id);
        let mut data = devinfo_data();
        SetupDiOpenDeviceInfoW(set.0, PCWSTR(id.as_ptr()), None, 0, Some(&mut data))
            .map_err(|e| refused("open device", e))?;
        Ok((set, data))
    }
}

/// Read a REG_MULTI_SZ device registry property as strings.
fn multi_sz_property(
    set: HDEVINFO,
    data: &SP_DEVINFO_DATA,
    property: SETUP_DI_REGISTRY_PROPERTY,
) -> Vec<String> {
    let mut required = 0_u32;
    // SAFETY: size query with no buffer; `required` receives the byte count.
    unsafe {
        let _ =
            SetupDiGetDeviceRegistryPropertyW(set, data, property, None, None, Some(&mut required));
    }
    if required == 0 || required > 64 * 1024 {
        return Vec::new();
    }
    let mut buffer = vec![0_u8; required as usize];
    // SAFETY: `buffer` has exactly `required` bytes and lives across the call.
    let ok = unsafe {
        SetupDiGetDeviceRegistryPropertyW(set, data, property, None, Some(&mut buffer), None)
    };
    if ok.is_err() {
        return Vec::new();
    }
    let units: Vec<u16> = buffer
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .collect();
    units
        .split(|unit| *unit == 0)
        .filter(|part| !part.is_empty())
        .map(String::from_utf16_lossy)
        .collect()
}

fn instance_id(set: HDEVINFO, data: &SP_DEVINFO_DATA) -> Result<String, Failure> {
    let mut buffer = vec![0_u16; 512];
    let mut required = 0_u32;
    // SAFETY: `buffer` is writable for its full length; `required` is a valid out pointer.
    unsafe {
        SetupDiGetDeviceInstanceIdW(set, data, Some(&mut buffer), Some(&mut required))
            .map_err(|e| refused("read device instance ID", e))?;
    }
    let length = buffer
        .iter()
        .position(|unit| *unit == 0)
        .unwrap_or(buffer.len());
    Ok(String::from_utf16_lossy(&buffer[..length]))
}

fn open_registry(
    instance: &str,
    key_type: u32,
    access: windows::Win32::System::Registry::REG_SAM_FLAGS,
) -> Result<(HKEY, DeviceInfoSet), Failure> {
    let (set, data) = open_device(instance)?;
    // SAFETY: `set`/`data` identify an existing device; the returned key is
    // closed by the caller with RegCloseKey.
    let key =
        unsafe { SetupDiOpenDevRegKey(set.0, &data, DICS_FLAG_GLOBAL.0, 0, key_type, access.0) }
            .map_err(|e| refused("open device registry key", e))?;
    Ok((key, set))
}

/// Calls CertGetNameStringW on the leaf signer of a verified WinTrust state.
fn signer_name(state: HANDLE) -> Option<String> {
    use windows::Win32::Security::Cryptography::{
        CertGetNameStringW, CERT_NAME_SIMPLE_DISPLAY_TYPE,
    };
    use windows::Win32::Security::WinTrust::{
        WTHelperGetProvSignerFromChain, WTHelperProvDataFromStateData,
    };
    // SAFETY: `state` is the hWVTStateData of a successful WTD_STATEACTION_VERIFY
    // call that is still open; all pointers come from WinTrust and are checked
    // for null before use; the name buffer is sized by the first call.
    unsafe {
        let provider = WTHelperProvDataFromStateData(state);
        if provider.is_null() {
            return None;
        }
        let signer = WTHelperGetProvSignerFromChain(provider, 0, false, 0);
        if signer.is_null() || (*signer).csCertChain == 0 || (*signer).pasCertChain.is_null() {
            return None;
        }
        let certificate = (*(*signer).pasCertChain).pCert;
        if certificate.is_null() {
            return None;
        }
        let length = CertGetNameStringW(certificate, CERT_NAME_SIMPLE_DISPLAY_TYPE, 0, None, None);
        if length <= 1 || length > 1024 {
            return None;
        }
        let mut name = vec![0_u16; length as usize];
        CertGetNameStringW(
            certificate,
            CERT_NAME_SIMPLE_DISPLAY_TYPE,
            0,
            None,
            Some(&mut name),
        );
        let end = name
            .iter()
            .position(|unit| *unit == 0)
            .unwrap_or(name.len());
        Some(String::from_utf16_lossy(&name[..end]))
    }
}

/// Verify that `member` is listed in `catalog` with a matching hash and
/// that the catalog's signature is trusted under `action`. Returns the
/// leaf signer's display name.
fn verify_catalog_member(
    catalog: &Path,
    member: &Path,
    action: windows::core::GUID,
) -> Result<String, Failure> {
    use windows::Win32::Security::Cryptography::Catalog::{
        CryptCATAdminAcquireContext2, CryptCATAdminCalcHashFromFileHandle2,
        CryptCATAdminReleaseContext,
    };
    use windows::Win32::Security::WinTrust::*;
    use windows::Win32::Storage::FileSystem::{
        CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_GENERIC_READ, FILE_SHARE_READ, OPEN_EXISTING,
    };
    let catalog_w = wide_path(catalog);
    let member_w = wide_path(member);
    // SAFETY: all buffers (paths, tag, hash) are owned locals that outlive
    // every call; the file and catalog-admin handles are released on every
    // path; the WinTrust state is closed with WTD_STATEACTION_CLOSE after
    // the signer name has been read from it.
    unsafe {
        let file = CreateFileW(
            PCWSTR(member_w.as_ptr()),
            FILE_GENERIC_READ.0,
            FILE_SHARE_READ,
            None,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            None,
        )
        .map_err(|e| refused("open package file", e))?;
        let close_file = || {
            let _ = windows::Win32::Foundation::CloseHandle(file);
        };
        let mut admin = 0_isize;
        if let Err(error) = CryptCATAdminAcquireContext2(
            &mut admin,
            Some(&DRIVER_ACTION_VERIFY),
            windows::core::w!("SHA256"),
            None,
            None,
        ) {
            close_file();
            return Err(refused("acquire catalog context", error));
        }
        let mut hash = vec![0_u8; 64];
        let mut hash_length = hash.len() as u32;
        let hashed = CryptCATAdminCalcHashFromFileHandle2(
            admin,
            file,
            &mut hash_length,
            Some(hash.as_mut_ptr()),
            None,
        );
        if let Err(error) = hashed {
            close_file();
            let _ = CryptCATAdminReleaseContext(admin, 0);
            return Err(refused("hash package file", error));
        }
        hash.truncate(hash_length as usize);
        let tag: String = hash.iter().map(|byte| format!("{byte:02X}")).collect();
        let tag_w = wide(&tag);
        let mut catalog_info = WINTRUST_CATALOG_INFO {
            cbStruct: std::mem::size_of::<WINTRUST_CATALOG_INFO>() as u32,
            pcwszCatalogFilePath: PCWSTR(catalog_w.as_ptr()),
            pcwszMemberTag: PCWSTR(tag_w.as_ptr()),
            pcwszMemberFilePath: PCWSTR(member_w.as_ptr()),
            hMemberFile: file,
            pbCalculatedFileHash: hash.as_mut_ptr(),
            cbCalculatedFileHash: hash.len() as u32,
            hCatAdmin: admin,
            ..Default::default()
        };
        let mut data = WINTRUST_DATA {
            cbStruct: std::mem::size_of::<WINTRUST_DATA>() as u32,
            dwUIChoice: WTD_UI_NONE,
            fdwRevocationChecks: WTD_REVOKE_NONE,
            dwUnionChoice: WTD_CHOICE_CATALOG,
            dwStateAction: WTD_STATEACTION_VERIFY,
            ..Default::default()
        };
        data.Anonymous.pCatalog = &mut catalog_info;
        let mut action = action;
        let status = WinVerifyTrust(
            HWND::default(),
            &mut action,
            (&mut data as *mut WINTRUST_DATA).cast(),
        );
        let name = if status == 0 {
            signer_name(data.hWVTStateData)
        } else {
            None
        };
        data.dwStateAction = WTD_STATEACTION_CLOSE;
        let _ = WinVerifyTrust(
            HWND::default(),
            &mut action,
            (&mut data as *mut WINTRUST_DATA).cast(),
        );
        let _ = CryptCATAdminReleaseContext(admin, 0);
        close_file();
        if status != 0 {
            return Err(Failure {
                kind: crate::ExitKind::VerificationFailed,
                message: format!(
                    "{} is not validly signed by its catalog: {}",
                    member
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default(),
                    windows::core::Error::from(windows::core::HRESULT(status)).message()
                ),
                hresult: Some(status),
            });
        }
        name.ok_or_else(|| Failure::verification("the catalog signer could not be read"))
    }
}

impl Platform for WindowsPlatform {
    fn verify_package(
        &mut self,
        package: &PackageInfo,
        allow_test: bool,
    ) -> Result<Signer, Failure> {
        use windows::Win32::Security::WinTrust::{
            DRIVER_ACTION_VERIFY, WINTRUST_ACTION_GENERIC_VERIFY_V2,
        };
        let mut signer = None;
        for member in [&package.inf, &package.sys] {
            // Driver policy first (the policy Windows uses for driver
            // packages). Only a test-enabled helper may fall back to plain
            // Authenticode, which a VM-trusted test certificate satisfies.
            let name = match verify_catalog_member(&package.cat, member, DRIVER_ACTION_VERIFY) {
                Ok(name) => name,
                Err(failure) if allow_test => {
                    verify_catalog_member(&package.cat, member, WINTRUST_ACTION_GENERIC_VERIFY_V2)
                        .map_err(|_| failure)?
                }
                Err(failure) => return Err(failure),
            };
            match &signer {
                None => signer = Some(name),
                Some(previous) if *previous == name => {}
                Some(_) => {
                    return Err(Failure::verification(
                        "the INF and SYS are signed by different certificates",
                    ))
                }
            }
        }
        let name = signer.unwrap_or_default();
        Ok(if name == MICROSOFT_DRIVER_SIGNER {
            Signer::Microsoft
        } else {
            Signer::Other(name)
        })
    }

    fn stage_package(&mut self, inf: &Path) -> Result<String, Failure> {
        let inf_w = wide_path(inf);
        let source_w = wide_path(inf.parent().unwrap_or(Path::new(".")));
        let mut destination = vec![0_u16; 260];
        let mut component = PWSTR::null();
        // SAFETY: input paths are NUL-terminated owned buffers; `destination`
        // is writable for its length and `component` points into it.
        unsafe {
            SetupCopyOEMInfW(
                PCWSTR(inf_w.as_ptr()),
                PCWSTR(source_w.as_ptr()),
                SPOST_PATH,
                SP_COPY_STYLE(0),
                Some(&mut destination),
                None,
                Some(&mut component),
            )
            .map_err(|e| refused("add the package to the driver store", e))?;
            let name = if component.is_null() {
                String::new()
            } else {
                component.to_string().unwrap_or_default()
            };
            if !is_oem_inf_name(&name) {
                return Err(Failure::windows(
                    format!("unexpected driver store name {name:?}"),
                    0,
                ));
            }
            Ok(name.to_ascii_lowercase())
        }
    }

    fn unstage_package(&mut self, oem_inf: &str) -> Result<(), Failure> {
        if !is_oem_inf_name(oem_inf) {
            return Err(Failure::invalid(format!(
                "refusing to remove {oem_inf:?}: not an oem*.inf name"
            )));
        }
        let name = wide(oem_inf);
        // SAFETY: NUL-terminated name; flags 0 = refuse while a device uses it.
        let ok = unsafe { SetupUninstallOEMInfW(PCWSTR(name.as_ptr()), 0, None) };
        if ok.as_bool() {
            Ok(())
        } else {
            Err(refused(
                &format!("remove {oem_inf} from the driver store"),
                windows::core::Error::from_thread(),
            ))
        }
    }

    fn find_device(&mut self) -> Result<Option<String>, Failure> {
        // SAFETY: enumerates root-enumerated devices of all classes; each
        // SP_DEVINFO_DATA is initialized with its size; the set is destroyed
        // by the guard.
        unsafe {
            let set = DeviceInfoSet(
                SetupDiGetClassDevsW(None, windows::core::w!("ROOT"), None, DIGCF_ALLCLASSES)
                    .map_err(|e| refused("enumerate root devices", e))?,
            );
            let mut found = Vec::new();
            let mut index = 0;
            loop {
                let mut data = devinfo_data();
                if SetupDiEnumDeviceInfo(set.0, index, &mut data).is_err() {
                    break;
                }
                index += 1;
                if multi_sz_property(set.0, &data, SPDRP_HARDWAREID)
                    .iter()
                    .any(|id| id.eq_ignore_ascii_case(HARDWARE_ID))
                {
                    found.push(instance_id(set.0, &data)?);
                }
            }
            match found.len() {
                0 => Ok(None),
                1 => Ok(found.pop()),
                _ => Err(Failure::invalid(format!(
                    "{} AudioRouter cable devices exist ({}); remove the extra ones in Device Manager",
                    found.len(), found.join(", ")
                ))),
            }
        }
    }

    fn create_device(&mut self, inf: &Path) -> Result<(String, bool), Failure> {
        let inf_w = wide_path(inf);
        let hardware_id = wide(HARDWARE_ID);
        // REG_MULTI_SZ: the ID, then an empty string terminator.
        let multi: Vec<u8> = HARDWARE_ID
            .encode_utf16()
            .chain([0, 0])
            .flat_map(|unit| unit.to_le_bytes())
            .collect();
        // SAFETY: devcon-equivalent sequence. All strings are owned and
        // NUL-terminated; `data` is sized; on any failure after registration
        // the new device is removed again with DIF_REMOVE before returning.
        unsafe {
            let set = DeviceInfoSet(
                SetupDiCreateDeviceInfoList(Some(&GUID_DEVCLASS_MEDIA), None)
                    .map_err(|e| refused("create device list", e))?,
            );
            let mut data = devinfo_data();
            SetupDiCreateDeviceInfoW(
                set.0,
                windows::core::w!("MEDIA"),
                &GUID_DEVCLASS_MEDIA,
                PCWSTR::null(),
                None,
                DICD_GENERATE_ID,
                Some(&mut data),
            )
            .map_err(|e| refused("create device information", e))?;
            SetupDiSetDeviceRegistryPropertyW(set.0, &mut data, SPDRP_HARDWAREID, Some(&multi))
                .map_err(|e| refused("set the hardware ID", e))?;
            SetupDiCallClassInstaller(DIF_REGISTERDEVICE, set.0, Some(&data))
                .map_err(|e| refused("register the root device", e))?;
            let id = instance_id(set.0, &data)?;
            let mut reboot = windows::core::BOOL(0);
            if let Err(error) = UpdateDriverForPlugAndPlayDevicesW(
                None,
                PCWSTR(hardware_id.as_ptr()),
                PCWSTR(inf_w.as_ptr()),
                INSTALLFLAG_FORCE,
                Some(&mut reboot),
            ) {
                let _ = SetupDiCallClassInstaller(DIF_REMOVE, set.0, Some(&data));
                return Err(refused("install the driver on the new device", error));
            }
            Ok((id, reboot.as_bool()))
        }
    }

    fn update_device(&mut self, inf: &Path, force: bool) -> Result<bool, Failure> {
        let inf_w = wide_path(inf);
        let hardware_id = wide(HARDWARE_ID);
        let flags = if force {
            INSTALLFLAG_FORCE
        } else {
            UPDATEDRIVERFORPLUGANDPLAYDEVICES_FLAGS(0)
        };
        let mut reboot = windows::core::BOOL(0);
        // SAFETY: owned NUL-terminated strings; `reboot` is a valid out pointer.
        unsafe {
            UpdateDriverForPlugAndPlayDevicesW(
                None,
                PCWSTR(hardware_id.as_ptr()),
                PCWSTR(inf_w.as_ptr()),
                flags,
                Some(&mut reboot),
            )
            .map_err(|e| refused("update the device driver", e))?;
        }
        Ok(reboot.as_bool())
    }

    fn remove_device(&mut self, instance: &str) -> Result<bool, Failure> {
        let (set, data) = open_device(instance)?;
        let mut reboot = windows::core::BOOL(0);
        // SAFETY: `set`/`data` were opened for this exact instance ID.
        unsafe {
            DiUninstallDevice(HWND::default(), set.0, &data, 0, Some(&mut reboot))
                .map_err(|e| refused("remove the device", e))?;
        }
        Ok(reboot.as_bool())
    }

    fn read_device_dword(&mut self, instance: &str, name: &str) -> Result<Option<u32>, Failure> {
        let (key, _set) = open_registry(instance, DIREG_DEV, KEY_READ)?;
        let name_w = wide(name);
        let mut kind = REG_VALUE_TYPE(0);
        let mut value = 0_u32;
        let mut size = 4_u32;
        // SAFETY: `value` provides exactly `size` writable bytes; key closed below.
        let status = unsafe {
            let status = RegQueryValueExW(
                key,
                PCWSTR(name_w.as_ptr()),
                None,
                Some(&mut kind),
                Some((&mut value as *mut u32).cast()),
                Some(&mut size),
            );
            let _ = RegCloseKey(key);
            status
        };
        Ok((status.is_ok() && kind == REG_DWORD && size == 4).then_some(value))
    }

    fn write_device_dword(
        &mut self,
        instance: &str,
        name: &str,
        value: u32,
    ) -> Result<(), Failure> {
        let (key, _set) = open_registry(instance, DIREG_DEV, KEY_SET_VALUE)?;
        let name_w = wide(name);
        // SAFETY: four little-endian bytes for REG_DWORD; key closed below.
        let status = unsafe {
            let status = RegSetValueExW(
                key,
                PCWSTR(name_w.as_ptr()),
                None,
                REG_DWORD,
                Some(&value.to_le_bytes()),
            );
            let _ = RegCloseKey(key);
            status
        };
        if status.is_ok() {
            Ok(())
        } else {
            Err(win32_refused(&format!("write {name}"), status))
        }
    }

    fn restart_device(&mut self, instance: &str) -> Result<bool, Failure> {
        let (set, data) = open_device(instance)?;
        let params = SP_PROPCHANGE_PARAMS {
            ClassInstallHeader: SP_CLASSINSTALL_HEADER {
                cbSize: std::mem::size_of::<SP_CLASSINSTALL_HEADER>() as u32,
                InstallFunction: DIF_PROPERTYCHANGE,
            },
            StateChange: DICS_PROPCHANGE,
            Scope: DICS_FLAG_GLOBAL,
            HwProfile: 0,
        };
        // SAFETY: `params` begins with its class-install header and is passed
        // with its full size; `install` is a sized SP_DEVINSTALL_PARAMS_W.
        unsafe {
            SetupDiSetClassInstallParamsW(
                set.0,
                Some(&data),
                Some(&params.ClassInstallHeader),
                std::mem::size_of::<SP_PROPCHANGE_PARAMS>() as u32,
            )
            .map_err(|e| refused("prepare the device restart", e))?;
            SetupDiCallClassInstaller(DIF_PROPERTYCHANGE, set.0, Some(&data))
                .map_err(|e| refused("restart the device", e))?;
            let mut install = SP_DEVINSTALL_PARAMS_W {
                cbSize: std::mem::size_of::<SP_DEVINSTALL_PARAMS_W>() as u32,
                ..Default::default()
            };
            SetupDiGetDeviceInstallParamsW(set.0, Some(&data), &mut install)
                .map_err(|e| refused("read the restart result", e))?;
            Ok(install.Flags.0 & (DI_NEEDREBOOT.0 | DI_NEEDRESTART.0) != 0)
        }
    }

    fn device_driver_version(&mut self, instance: &str) -> Result<Option<DriverVersion>, Failure> {
        // The installed driver's software key holds "DriverVersion" (REG_SZ).
        let Ok((key, _set)) = open_registry(instance, DIREG_DRV, KEY_READ) else {
            return Ok(None);
        };
        let name = wide("DriverVersion");
        let mut kind = REG_VALUE_TYPE(0);
        let mut buffer = [0_u16; 64];
        let mut size = std::mem::size_of_val(&buffer) as u32;
        // SAFETY: `buffer` provides `size` writable bytes; key closed below.
        let status = unsafe {
            let status = RegQueryValueExW(
                key,
                PCWSTR(name.as_ptr()),
                None,
                Some(&mut kind),
                Some(buffer.as_mut_ptr().cast()),
                Some(&mut size),
            );
            let _ = RegCloseKey(key);
            status
        };
        if status.is_err() || kind != REG_SZ {
            return Ok(None);
        }
        let units = (size as usize / 2).min(buffer.len());
        let text = String::from_utf16_lossy(&buffer[..units]);
        Ok(DriverVersion::parse(text.trim_end_matches('\0')))
    }

    fn default_endpoints(&mut self) -> Vec<(String, String)> {
        use audiorouter_windows_audio::{DefaultEndpointRole, EndpointDirection};
        audiorouter_windows_audio::enumerate_default_endpoint_bindings()
            .unwrap_or_default()
            .into_iter()
            .map(|binding| {
                let direction = match binding.direction {
                    EndpointDirection::Render => "render",
                    EndpointDirection::Capture => "capture",
                };
                let role = match binding.role {
                    DefaultEndpointRole::Console => "console",
                    DefaultEndpointRole::Multimedia => "multimedia",
                    DefaultEndpointRole::Communications => "communications",
                };
                (format!("{direction}.{role}"), binding.endpoint_id)
            })
            .collect()
    }

    fn cable_endpoints(&mut self) -> Vec<CableEndpoint> {
        // Display inventory contains disabled and historical MMDevice records.
        // Only active endpoints of the matching data flow count as installed.
        let display =
            audiorouter_windows_audio::enumerate_active_endpoint_display_info().unwrap_or_default();
        let states = audiorouter_windows_audio::enumerate_endpoint_states().unwrap_or_default();
        select_active_cable_endpoints(display, &states)
    }

    fn set_endpoint_description(
        &mut self,
        endpoint_id: &str,
        description: &str,
    ) -> Result<(), Failure> {
        use windows::Win32::Foundation::PROPERTYKEY;
        use windows::Win32::Media::Audio::{IMMDeviceEnumerator, MMDeviceEnumerator};
        use windows::Win32::System::Com::StructuredStorage::{
            PROPVARIANT, PROPVARIANT_0, PROPVARIANT_0_0, PROPVARIANT_0_0_0,
        };
        use windows::Win32::System::Com::{
            CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_MULTITHREADED,
            STGM_READWRITE,
        };
        use windows::Win32::System::Variant::VT_LPWSTR;
        // PKEY_Device_FriendlyName is the display label. DeviceDesc retains
        // the driver category name and is used to recover cable identity.
        const DEVICE_FRIENDLY_NAME: PROPERTYKEY = PROPERTYKEY {
            fmtid: windows::core::GUID::from_u128(0xa45c254e_df1c_4efd_8020_67d146a850e0),
            pid: 14,
        };
        let id = wide(endpoint_id);
        let mut text = wide(&format!("{description} (AudioRouter Virtual Cable)"));
        // SAFETY: COM is initialized for this call and uninitialized after;
        // the PROPVARIANT borrows `text`, which outlives SetValue (the store
        // copies it), and is never passed to PropVariantClear.
        unsafe {
            CoInitializeEx(None, COINIT_MULTITHREADED)
                .ok()
                .map_err(|e| refused("initialize COM", e))?;
            let result = (|| -> windows::core::Result<()> {
                let enumerator: IMMDeviceEnumerator =
                    CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
                let device = enumerator.GetDevice(PCWSTR(id.as_ptr()))?;
                let store = device.OpenPropertyStore(STGM_READWRITE)?;
                let value = PROPVARIANT {
                    Anonymous: PROPVARIANT_0 {
                        Anonymous: std::mem::ManuallyDrop::new(PROPVARIANT_0_0 {
                            vt: VT_LPWSTR,
                            wReserved1: 0,
                            wReserved2: 0,
                            wReserved3: 0,
                            Anonymous: PROPVARIANT_0_0_0 {
                                pwszVal: PWSTR(text.as_mut_ptr()),
                            },
                        }),
                    },
                };
                store.SetValue(&DEVICE_FRIENDLY_NAME, &value)?;
                store.Commit()
            })();
            CoUninitialize();
            result.map_err(|e| refused("set the endpoint name", e))
        }
    }

    fn query_driver(&mut self) -> Option<(u16, u16, u32, u32)> {
        let client = audiorouter_windows_audio::NativeBridgeControlClient::open(
            r"\\.\AudioRouterVirtualBridge",
        )
        .ok()?;
        let info = client.query().ok()?;
        Some((
            info.protocol_major,
            info.protocol_minor,
            info.capabilities,
            info.cable_count,
        ))
    }

    fn sleep_ms(&mut self, ms: u64) {
        std::thread::sleep(std::time::Duration::from_millis(ms));
    }

    fn now_utc(&mut self) -> String {
        let seconds = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        format_utc(seconds)
    }
}

/// RFC 3339 UTC timestamp without a date library (civil-from-days).
pub fn format_utc(seconds: u64) -> String {
    let days = (seconds / 86_400) as i64;
    let rem = seconds % 86_400;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        rem / 60 % 60,
        rem % 60
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn cable_inventory_ignores_stale_records_and_wrong_data_flow() {
        use audiorouter_windows_audio::{
            EndpointDirection, EndpointDisplayInfo, EndpointState, EndpointStateInfo,
        };
        let display = |id: &str, flow| EndpointDisplayInfo {
            id: id.into(),
            direction: flow,
            name: "My renamed endpoint".into(),
            device_description: "AudioRouter Cable A Input".into(),
            driver_inf_section: String::new(),
        };
        let states = vec![
            EndpointStateInfo {
                id: "active".into(),
                direction: EndpointDirection::Render,
                state: EndpointState::Active,
            },
            EndpointStateInfo {
                id: "stale".into(),
                direction: EndpointDirection::Render,
                state: EndpointState::NotPresent,
            },
            EndpointStateInfo {
                id: "wrong-flow".into(),
                direction: EndpointDirection::Capture,
                state: EndpointState::Active,
            },
        ];
        let result = super::select_active_cable_endpoints(
            vec![
                display("active", EndpointDirection::Render),
                display("stale", EndpointDirection::Render),
                display("wrong-flow", EndpointDirection::Capture),
                display("missing-state", EndpointDirection::Render),
            ],
            &states,
        );
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].endpoint_id, "active");
        assert_eq!(result[0].cable, 0);
        assert_eq!(result[0].direction, crate::Direction::Input);
    }

    #[test]
    fn utc_formatting_matches_known_instants() {
        assert_eq!(super::format_utc(0), "1970-01-01T00:00:00Z");
        assert_eq!(super::format_utc(1_791_244_800), "2026-10-06T00:00:00Z");
        assert_eq!(
            super::format_utc(951_782_400 + 3_661),
            "2000-02-29T01:01:01Z"
        );
    }
}
