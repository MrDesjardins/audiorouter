//! Native Save/Open dialogs for `.audiorouter` session files and for choosing
//! an application executable.
//!
//! The dialog runs on its own COM single-threaded-apartment thread so it
//! never depends on how the WebView thread initialized COM. The caller must
//! not block the owner window's thread while it waits (the Tauri command is
//! async), because the dialog disables its owner through window messages.

use std::path::PathBuf;
use windows::core::{HSTRING, PCWSTR, PWSTR};
use windows::Win32::Foundation::{ERROR_CANCELLED, HWND};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, CLSCTX_INPROC_SERVER,
    COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE,
};
use windows::Win32::UI::Shell::Common::COMDLG_FILTERSPEC;
use windows::Win32::UI::Shell::{
    FileOpenDialog, FileSaveDialog, IFileDialog, IFileOpenDialog, IFileSaveDialog,
    FOS_FILEMUSTEXIST, FOS_FORCEFILESYSTEM, FOS_OVERWRITEPROMPT, FOS_PATHMUSTEXIST,
    SIGDN_FILESYSPATH,
};

/// What one dialog asks for. Plain strings, so the request can move to the
/// dialog thread; the wide strings are built there.
struct DialogRequest {
    save: bool,
    title: &'static str,
    filter_name: &'static str,
    filter_spec: &'static str,
    default_extension: Option<&'static str>,
    file_name: Option<String>,
    thread_name: &'static str,
}

/// Ask for a session file. `save` shows a Save dialog prefilled with
/// `suggested_name`; otherwise an Open dialog. `Ok(None)` means cancelled.
pub fn choose(save: bool, suggested_name: &str, owner: isize) -> Result<Option<PathBuf>, String> {
    run(
        DialogRequest {
            save,
            title: if save {
                "Save session to a file"
            } else {
                "Open a session file"
            },
            filter_name: "AudioRouter session (*.audiorouter)",
            filter_spec: "*.audiorouter",
            default_extension: Some("audiorouter"),
            file_name: save.then(|| suggested_name.to_owned()),
            thread_name: "audiorouter-session-file-dialog",
        },
        owner,
    )
}

/// Ask the user for one existing Windows executable.
pub fn choose_executable(owner: isize) -> Result<Option<PathBuf>, String> {
    run(
        DialogRequest {
            save: false,
            title: "Choose an application executable",
            filter_name: "Applications (*.exe)",
            filter_spec: "*.exe",
            default_extension: None,
            file_name: None,
            thread_name: "audiorouter-application-file-dialog",
        },
        owner,
    )
}

fn run(request: DialogRequest, owner: isize) -> Result<Option<PathBuf>, String> {
    std::thread::Builder::new()
        .name(request.thread_name.into())
        .spawn(move || {
            // SAFETY: this thread owns its COM apartment; every successful
            // CoInitializeEx is paired with CoUninitialize below, after all
            // COM interface values created here have been dropped.
            let initialized =
                unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) }
                    .is_ok();
            // SAFETY: `owner` is a window handle from this process (or 0);
            // the dialog only uses it as the modal owner while it is shown.
            let result = unsafe { show(&request, owner) };
            if initialized {
                // SAFETY: balances the successful CoInitializeEx above.
                unsafe { CoUninitialize() };
            }
            result
        })
        .map_err(|error| format!("dialog thread failed: {error}"))?
        .join()
        .map_err(|_| "the file dialog stopped unexpectedly".to_owned())?
}

unsafe fn show(request: &DialogRequest, owner: isize) -> Result<Option<PathBuf>, String> {
    let dialog: IFileDialog = if request.save {
        let dialog: IFileSaveDialog = CoCreateInstance(&FileSaveDialog, None, CLSCTX_INPROC_SERVER)
            .map_err(|error| format!("Save dialog unavailable: {error}"))?;
        dialog.into()
    } else {
        let dialog: IFileOpenDialog = CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER)
            .map_err(|error| format!("Open dialog unavailable: {error}"))?;
        dialog.into()
    };
    // The HSTRINGs outlive SetFileTypes, which copies the filter strings.
    let filter_name = HSTRING::from(request.filter_name);
    let filter_spec = HSTRING::from(request.filter_spec);
    let filters = [COMDLG_FILTERSPEC {
        pszName: PCWSTR(filter_name.as_ptr()),
        pszSpec: PCWSTR(filter_spec.as_ptr()),
    }];
    let options = dialog.GetOptions().map_err(|error| error.to_string())?;
    let extra = if request.save {
        FOS_OVERWRITEPROMPT | FOS_PATHMUSTEXIST
    } else {
        FOS_FILEMUSTEXIST | FOS_PATHMUSTEXIST
    };
    dialog
        .SetOptions(options | extra | FOS_FORCEFILESYSTEM)
        .map_err(|error| error.to_string())?;
    dialog
        .SetFileTypes(&filters)
        .map_err(|error| error.to_string())?;
    if let Some(extension) = request.default_extension {
        dialog
            .SetDefaultExtension(&HSTRING::from(extension))
            .map_err(|error| error.to_string())?;
    }
    dialog
        .SetTitle(&HSTRING::from(request.title))
        .map_err(|error| error.to_string())?;
    if let Some(file_name) = &request.file_name {
        dialog
            .SetFileName(&HSTRING::from(file_name.as_str()))
            .map_err(|error| error.to_string())?;
    }
    let owner = (owner != 0).then_some(HWND(owner as *mut _));
    if let Err(error) = dialog.Show(owner) {
        if error.code() == ERROR_CANCELLED.to_hresult() {
            return Ok(None);
        }
        return Err(format!("file dialog failed: {error}"));
    }
    let item = dialog.GetResult().map_err(|error| error.to_string())?;
    let name: PWSTR = item
        .GetDisplayName(SIGDN_FILESYSPATH)
        .map_err(|error| error.to_string())?;
    // SAFETY: GetDisplayName returns a NUL-terminated string allocated with
    // CoTaskMemAlloc; it is copied before being freed exactly once.
    let path = name.to_string().map_err(|error| error.to_string());
    CoTaskMemFree(Some(name.0 as *const _));
    Ok(Some(PathBuf::from(path?)))
}
