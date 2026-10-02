//! Desktop instance ownership and explicit, same-user recovery before any DB
//! or control connection is opened. No process ID is accepted from the UI.
use windows::core::{w, PCWSTR, PWSTR};
use windows::Win32::Foundation::{CloseHandle, HANDLE, WAIT_ABANDONED, WAIT_OBJECT_0};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows::Win32::System::RemoteDesktop::ProcessIdToSessionId;
use windows::Win32::System::Threading::{
    CreateMutexW, OpenProcess, QueryFullProcessImageNameW, ReleaseMutex, TerminateProcess,
    WaitForSingleObject, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
    PROCESS_SYNCHRONIZE, PROCESS_TERMINATE,
};
use windows::Win32::UI::Controls::{
    TaskDialogIndirect, TASKDIALOGCONFIG, TASKDIALOG_BUTTON, TDCBF_CANCEL_BUTTON,
    TDF_ALLOW_DIALOG_CANCELLATION, TDF_USE_COMMAND_LINKS,
};
use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};

struct OwnedHandle(HANDLE);
impl Drop for OwnedHandle {
    fn drop(&mut self) {
        // SAFETY: construction takes exactly one valid owned Windows handle.
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

pub struct InstanceGuard {
    handle: OwnedHandle,
    owned: bool,
}
impl Drop for InstanceGuard {
    fn drop(&mut self) {
        if self.owned {
            // SAFETY: this thread acquired the mutex, and releases it once.
            unsafe {
                let _ = ReleaseMutex(self.handle.0);
            }
        }
    }
}

fn eligible(pid: u32, own_pid: u32, image: &str, same_user: bool, same_session: bool) -> bool {
    pid != own_pid
        && same_user
        && same_session
        && std::path::Path::new(image).file_name().is_some_and(|name| {
            name.to_string_lossy()
                .eq_ignore_ascii_case("audiorouter-shell.exe")
        })
}

/// Returned handles pin verified process objects, avoiding PID reuse when the
/// user takes time to read the recovery dialog.
fn existing_instances() -> Result<Vec<OwnedHandle>, String> {
    // SAFETY: snapshot handle is owned locally; entry size matches the ABI and
    // Windows fills only its fixed fields. Each process is opened independently.
    unsafe {
        let snapshot = OwnedHandle(CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0).map_err(
            |_| "Could not check running apps. Please close AudioRouter and try again.",
        )?);
        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        let mut found = Vec::new();
        let mut own_session = 0;
        ProcessIdToSessionId(std::process::id(), &mut own_session)
            .map_err(|_| "Could not check this Windows session.")?;
        if Process32FirstW(snapshot.0, &mut entry).is_err() {
            return Ok(found);
        }
        loop {
            let end = entry
                .szExeFile
                .iter()
                .position(|c| *c == 0)
                .unwrap_or(entry.szExeFile.len());
            let name = String::from_utf16_lossy(&entry.szExeFile[..end]);
            if entry.th32ProcessID != std::process::id()
                && name.eq_ignore_ascii_case("audiorouter-shell.exe")
            {
                // Query first; other users' apps are ignored. A denied query on
                // an unknown process never grants termination authority.
                if let Ok(handle) = OpenProcess(
                    PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE,
                    false,
                    entry.th32ProcessID,
                ) {
                    let pinned = OwnedHandle(handle);
                    let mut image = vec![0_u16; 32768];
                    let mut length = image.len() as u32;
                    let mut session = u32::MAX;
                    let verified_image = QueryFullProcessImageNameW(
                        pinned.0,
                        PROCESS_NAME_WIN32,
                        PWSTR(image.as_mut_ptr()),
                        &mut length,
                    )
                    .is_ok();
                    let verified_session =
                        ProcessIdToSessionId(entry.th32ProcessID, &mut session).is_ok();
                    let same_user = audiorouter_transport::client_is_same_user(entry.th32ProcessID)
                        .unwrap_or(false);
                    if verified_image && verified_session && same_user && session != own_session {
                        return Err("AudioRouter is running in another Windows session. Please close it there and try again.".into());
                    }
                    if verified_image
                        && verified_session
                        && eligible(
                            entry.th32ProcessID,
                            std::process::id(),
                            &String::from_utf16_lossy(&image[..length as usize]),
                            same_user,
                            session == own_session,
                        )
                        && WaitForSingleObject(pinned.0, 0) != WAIT_OBJECT_0
                    {
                        // Open termination rights only after identity checks.
                        let killer = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_TERMINATE | PROCESS_SYNCHRONIZE, false, entry.th32ProcessID)
                            .map_err(|_| "There is already an instance running. Please close it. Windows would not allow this app to close it automatically.")?;
                        found.push(OwnedHandle(killer));
                        if found.len() > 8 {
                            return Err(
                                "Please close the extra AudioRouter instances and try again."
                                    .into(),
                            );
                        }
                    }
                }
            }
            if Process32NextW(snapshot.0, &mut entry).is_err() {
                break;
            }
        }
        Ok(found)
    }
}

fn recovery_dialog() -> Result<bool, String> {
    let buttons = [TASKDIALOG_BUTTON {
        nButtonID: 100,
        pszButtonText: w!("Force close old instance and continue\nStops its audio. Unsaved changes and unfinished recordings may be lost."),
    }];
    let config = TASKDIALOGCONFIG {
        cbSize: std::mem::size_of::<TASKDIALOGCONFIG>() as u32,
        pszWindowTitle: w!("AudioRouter"),
        pszMainInstruction: w!("There is already an instance running. Please close it."),
        pszContent: w!("You can close the existing AudioRouter window normally, then open this version again. If the old app is stuck, force close it below to continue with this version."),
        dwFlags: TDF_ALLOW_DIALOG_CANCELLATION | TDF_USE_COMMAND_LINKS,
        dwCommonButtons: TDCBF_CANCEL_BUTTON,
        cButtons: buttons.len() as u32,
        pButtons: buttons.as_ptr(),
        nDefaultButton: 2, // Cancel; Enter must not terminate another process.
        ..Default::default()
    };
    let mut chosen = 0;
    // SAFETY: all literal strings and button/config buffers outlive this modal
    // call. No callbacks, parent windows or borrowed handles are supplied.
    unsafe { TaskDialogIndirect(&config, Some(&mut chosen), None, None) }.map_err(|_| {
        "There is already an instance running. Please close it and open AudioRouter again."
    })?;
    Ok(chosen == 100)
}

/// None means the user canceled: exit without touching database/backend.
pub fn claim_or_recover() -> Result<Option<InstanceGuard>, String> {
    let sid = audiorouter_transport::current_user_sid()
        .map_err(|_| "Could not verify this Windows user.")?;
    let name: Vec<u16> = format!("Local\\AudioRouter.DesktopInstance.{sid}")
        .encode_utf16()
        .chain(Some(0))
        .collect();
    // SAFETY: name is NUL terminated and alive for the call. The handle is
    // owned by this scope; default security inherits the user's ACL.
    let handle = OwnedHandle(
        unsafe { CreateMutexW(None, false, PCWSTR(name.as_ptr())) }
            .map_err(|_| "Could not check AudioRouter instance ownership.")?,
    );
    // SAFETY: valid owned mutex; zero wait does not block startup.
    let result = unsafe { WaitForSingleObject(handle.0, 0) };
    let mut guard = InstanceGuard {
        handle,
        owned: result == WAIT_OBJECT_0 || result == WAIT_ABANDONED,
    };
    let existing = existing_instances()?;
    if !existing.is_empty() {
        if !recovery_dialog()? {
            return Ok(None);
        }
        for process in existing {
            // SAFETY: process is pinned and verified same-user/same-session
            // AudioRouter. Termination happens only after explicit dialog action.
            unsafe {
                if WaitForSingleObject(process.0, 0) != WAIT_OBJECT_0 {
                    TerminateProcess(process.0, 0).map_err(|_| {
                        "Could not close the old app. Close it in Task Manager, then try again."
                    })?;
                }
                if WaitForSingleObject(process.0, 5000) != WAIT_OBJECT_0 {
                    return Err(
                        "The old app is still closing. Please wait and open AudioRouter again."
                            .into(),
                    );
                }
            }
        }
    }
    if !guard.owned {
        // SAFETY: same valid owned mutex, now bounded to allow prior shutdown.
        let result = unsafe { WaitForSingleObject(guard.handle.0, 1000) };
        guard.owned = result == WAIT_OBJECT_0 || result == WAIT_ABANDONED;
        if !guard.owned {
            return Err(
                "There is already an instance starting. Please close it or wait, then try again."
                    .into(),
            );
        }
    }
    Ok(Some(guard))
}

pub fn show_error(message: &str) {
    let message: Vec<u16> = message.encode_utf16().chain(Some(0)).collect();
    // SAFETY: both strings are NUL terminated and outlive this modal call.
    unsafe {
        MessageBoxW(
            None,
            PCWSTR(message.as_ptr()),
            w!("AudioRouter"),
            MB_OK | MB_ICONERROR,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recovery_targets_only_another_verified_user_session_shell() {
        assert!(eligible(42, 7, "C:/old/audiorouter-shell.exe", true, true));
        for (pid, image, same_user, same_session) in [
            (7, "audiorouter-shell.exe", true, true),
            (42, "discord.exe", true, true),
            (42, "audiorouter-plugin-worker.exe", true, true),
            (42, "audiorouter-shell.exe", false, true),
            (42, "audiorouter-shell.exe", true, false),
        ] {
            assert!(!eligible(pid, 7, image, same_user, same_session));
        }
    }
    #[test]
    fn existing_instance_inventory_is_read_only_and_never_targets_self() {
        let found = existing_instances().unwrap();
        assert!(found.len() <= 8);
    }
}
