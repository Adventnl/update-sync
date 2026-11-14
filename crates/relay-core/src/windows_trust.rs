//! Windows owns attachment policy, origin marking, and shell execution.
use anyhow::{Context, Result};
use std::{os::windows::ffi::OsStrExt, path::Path};
use windows::{
    core::{w, GUID, PCWSTR},
    Win32::{
        Foundation::HWND,
        System::Com::{
            CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER,
            COINIT_APARTMENTTHREADED,
        },
        UI::{
            Shell::{AttachmentServices, IAttachmentExecute, ShellExecuteW},
            WindowsAndMessaging::SW_SHOWNORMAL,
        },
    },
};

struct Apartment(bool);
impl Apartment {
    fn enter() -> Result<Self> {
        let result = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
        // A pre-existing apartment is also usable. Do not uninitialize it.
        if result.0 as u32 == 0x80010106 {
            return Ok(Self(false));
        }
        result.ok()?;
        Ok(Self(true))
    }
}
impl Drop for Apartment {
    fn drop(&mut self) {
        if self.0 {
            unsafe {
                CoUninitialize();
            }
        }
    }
}

fn shell_path(path: &Path) -> Vec<u16> {
    let mut value: Vec<_> = path.as_os_str().encode_wide().collect();
    let prefix: Vec<_> = "\\\\?\\".encode_utf16().collect();
    if value.starts_with(&prefix) {
        value.drain(..4);
        let unc: Vec<_> = "UNC\\".encode_utf16().collect();
        if value.starts_with(&unc) {
            value.splice(..4, "\\\\".encode_utf16());
        }
    }
    value.push(0);
    value
}

pub fn preserve_origin(path: &Path, source: &str) -> Result<()> {
    let _apartment = Apartment::enter()?;
    let local = shell_path(path);
    let source: Vec<_> = source.encode_utf16().chain(Some(0)).collect();
    // All pointers remain valid for the duration of each COM call. The interface
    // is released before its apartment guard and is never shared across threads.
    unsafe {
        let attachment: IAttachmentExecute =
            CoCreateInstance(&AttachmentServices, None, CLSCTX_INPROC_SERVER)?;
        attachment.SetClientGuid(&GUID::from_u128(0x72d384df_7594_49fb_904b_7915c80c454e))?;
        attachment.SetClientTitle(w!("Relay"))?;
        attachment.SetLocalPath(PCWSTR(local.as_ptr()))?;
        attachment.SetSource(PCWSTR(source.as_ptr()))?;
        attachment.Save().context("Windows attachment security did not approve the downloaded application. The installed version is unchanged.")?;
    }
    Ok(())
}

pub fn open(path: &Path) -> Result<()> {
    let _apartment = Apartment::enter()?;
    let local = shell_path(path);
    let directory = shell_path(
        path.parent()
            .context("The application folder is missing.")?,
    );
    let result = unsafe {
        ShellExecuteW(
            Some(HWND::default()),
            w!("open"),
            PCWSTR(local.as_ptr()),
            PCWSTR::null(),
            PCWSTR(directory.as_ptr()),
            SW_SHOWNORMAL,
        )
    };
    anyhow::ensure!(result.0 as isize>32,"Windows did not open this application. Check its security prompt and try again. Shell error: {}.",result.0 as isize);
    Ok(())
}
