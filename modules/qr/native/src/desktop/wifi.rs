use std::{ffi::c_void, ptr, slice};

use windows_sys::{
    Win32::{
        Foundation::{ERROR_SUCCESS, HANDLE},
        NetworkManagement::WiFi::{
            WLAN_CONNECTION_PARAMETERS, WLAN_INTERFACE_INFO, WLAN_INTERFACE_INFO_LIST,
            WLAN_PROFILE_USER, WlanCloseHandle, WlanConnect, WlanEnumInterfaces, WlanFreeMemory,
            WlanOpenHandle, WlanSetProfile, dot11_BSS_type_infrastructure,
            wlan_connection_mode_profile,
        },
    },
    core::GUID,
};

use super::canvas::wide;
use crate::wifi_profile::{Security, profile};

/// The version of the WLAN API of Windows Vista and later.
const CLIENT_VERSION: u32 = 2;
/// All users: what Windows does when a network is joined from its own list.
const ALL_USERS: u32 = 0;

/// A session with the WLAN service, closed when dropped.
struct Session(HANDLE);

impl Session {
    fn open() -> Result<Self, String> {
        let (mut version, mut handle) = (0, ptr::null_mut());
        // SAFETY: writes the negotiated version and the handle into locals.
        let status = unsafe {
            WlanOpenHandle(
                CLIENT_VERSION,
                ptr::null(),
                &raw mut version,
                &raw mut handle,
            )
        };
        if status == ERROR_SUCCESS {
            Ok(Self(handle))
        } else {
            Err("no Wi-Fi on this computer".to_string())
        }
    }

    /// The first Wi-Fi adapter: a computer rarely has two.
    fn interface(&self) -> Result<GUID, String> {
        let mut list: *mut WLAN_INTERFACE_INFO_LIST = ptr::null_mut();
        // SAFETY: Windows allocates the list, read here and freed before returning.
        unsafe {
            if WlanEnumInterfaces(self.0, ptr::null(), &raw mut list) != ERROR_SUCCESS
                || list.is_null()
            {
                return Err("no Wi-Fi adapter".to_string());
            }
            let count = usize::try_from((*list).dwNumberOfItems).unwrap_or(0);
            let interfaces: &[WLAN_INTERFACE_INFO] =
                slice::from_raw_parts((*list).InterfaceInfo.as_ptr(), count);
            let first = interfaces.first().map(|interface| interface.InterfaceGuid);
            WlanFreeMemory(list.cast::<c_void>());
            first.ok_or_else(|| "no Wi-Fi adapter".to_string())
        }
    }

    fn set_profile(&self, interface: &GUID, xml: &[u16], flags: u32) -> bool {
        let mut reason = 0;
        // SAFETY: the profile ends with a zero and every pointer outlives the call.
        let status = unsafe {
            WlanSetProfile(
                self.0,
                interface,
                flags,
                xml.as_ptr(),
                ptr::null(),
                1,
                ptr::null(),
                &raw mut reason,
            )
        };
        status == ERROR_SUCCESS
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        // SAFETY: the handle came from `WlanOpenHandle` and is closed once.
        unsafe { WlanCloseHandle(self.0, ptr::null()) };
    }
}

/// Saves the network for Windows, as joining it from its list would, then joins it. Windows asks
/// nothing more: the code held all it needs.
pub fn connect(ssid: &str, password: &str, security: Security, hidden: bool) -> Result<(), String> {
    let session = Session::open()?;
    let interface = session.interface()?;
    let xml = wide(&profile(ssid, password, security, hidden));
    // Some computers keep shared profiles for administrators: the user's own then does.
    if !session.set_profile(&interface, &xml, ALL_USERS)
        && !session.set_profile(&interface, &xml, WLAN_PROFILE_USER)
    {
        return Err("Windows refused the network".to_string());
    }
    let name = wide(ssid);
    let parameters = WLAN_CONNECTION_PARAMETERS {
        wlanConnectionMode: wlan_connection_mode_profile,
        strProfile: name.as_ptr(),
        pDot11Ssid: ptr::null_mut(),
        pDesiredBssidList: ptr::null_mut(),
        dot11BssType: dot11_BSS_type_infrastructure,
        dwFlags: 0,
    };
    // SAFETY: the parameters and the name they point to outlive the call.
    let status = unsafe {
        WlanConnect(
            session.0,
            &raw const interface,
            &raw const parameters,
            ptr::null(),
        )
    };
    if status == ERROR_SUCCESS {
        Ok(())
    } else {
        Err("Windows could not join the network".to_string())
    }
}
