//! WebKit tabs' side in the app: which loads leave Chromium for a WKWebView, and what a WebKit
//! tab's page asks of the app (platform/mac/webkit.mm, docs/decisions.md, decision 9).

use std::cell::RefCell;
use std::collections::HashSet;
use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::sync::OnceLock;

use browser::{log, webkit};

use crate::{CX, bc, front, open_tab, open_tab_behind, platform, profile_dir, run_page_shortcut};

/// Where a link in a WebKit tab opens (platform/mac/host.h).
const LINK_HERE: c_int = 0;
const LINK_NEW_TAB: c_int = 1;

thread_local! {
    /// The WebKit tabs whose state says they can go back, which, once their own page can't, is
    /// to the Chromium page they left.
    static CAN_GO_BACK: RefCell<HashSet<u64>> = RefCell::new(HashSet::new());
}

/// The hosts the checks add to the sites, with BROWSER_WEBKIT_SITES, separated by commas.
fn extra_sites() -> &'static [String] {
    static EXTRA: OnceLock<Vec<String>> = OnceLock::new();
    EXTRA.get_or_init(|| {
        let sites = std::env::var("BROWSER_WEBKIT_SITES").unwrap_or_default();
        sites.split(',').map(str::trim).filter(|s| !s.is_empty()).map(str::to_lowercase).collect()
    })
}

/// Runs `then` once the event or the core's callback that wants it is over.
fn later(then: impl FnOnce() + 'static) {
    if let Some(cx) = CX.with_borrow(Clone::clone) {
        cx.foreground_executor().spawn(async move { then() }).detach();
    }
}

pub fn start() {
    unsafe { platform::host_webkit_start(link, key, focused) };
}

/// Where WebKit keeps the session of `profile`'s tabs: its directory, as Chromium's.
fn profile_path(profile: bc::bc_id) -> Option<String> {
    let dir = crate::name(bc::bc_profile_dir, profile);
    (!dir.is_empty()).then(|| format!("{}/{dir}", profile_dir()))
}

pub unsafe extern "C" fn tab_navigating(
    _: *mut c_void,
    tab: bc::bc_id,
    url: *const c_char,
) -> bool {
    let url = unsafe { CStr::from_ptr(url) }.to_string_lossy().into_owned();
    if !webkit::plays_in_webkit(&url, extra_sites()) {
        if unsafe { platform::host_webkit_has(tab) } {
            // check.sh reads this line.
            log!("browser: webkit {tab} leaves for {url}");
            close(tab);
        }
        return false;
    }
    // check.sh reads this line.
    log!("browser: webkit {tab} {url}");
    later(move || {
        let profile = unsafe { bc::bc_tab_profile(tab) };
        let (Some(path), Ok(url)) = (profile_path(profile), CString::new(url)) else {
            return;
        };
        let path = CString::new(path).unwrap_or_default();
        let private = unsafe { bc::bc_tab_private(tab) };
        unsafe { platform::host_webkit_show(tab, path.as_ptr(), private, url.as_ptr()) };
    });
    true
}

/// A link in a WebKit tab: to a site that plays in WebKit, it stays there, else it goes to
/// Chromium, in this tab or a new one, which a listed site brings back to WebKit.
extern "C" fn link(tab: u64, url: *const c_char, place: c_int) -> bool {
    let url = unsafe { CStr::from_ptr(url) }.to_string_lossy().into_owned();
    if place == LINK_HERE && webkit::stays_in_webkit(&url, extra_sites()) {
        return false;
    }
    later(move || {
        if place == LINK_HERE {
            if let Ok(url) = CString::new(url) {
                unsafe { bc::bc_tab_navigate(tab, url.as_ptr()) };
            }
            return;
        }
        let window = match unsafe { bc::bc_tab_window(tab) } {
            0 => front().0,
            window => window,
        };
        match place {
            LINK_NEW_TAB => open_tab(window, &url),
            _ => _ = open_tab_behind(window, &url, false),
        }
    });
    true
}

/// A key a WebKit tab's page has, which runs the app's shortcut as a Chromium page's does. WebKit
/// doesn't say whether the page's focus takes text, so its own editing keys, as ⌘Z, stay its.
extern "C" fn key(tab: u64, key: *const c_char, modifiers: u32, reserved: bool) -> bool {
    let key = bc::bc_key { key, modifiers, editing: true };
    run_page_shortcut(tab, &key, reserved)
}

/// A WebKit tab's page took the keyboard, as a click in it does, which in a split view selects it
/// as a click in a Chromium page does.
extern "C" fn focused(tab: u64) {
    let window = unsafe { bc::bc_tab_window(tab) };
    if window != 0 && unsafe { bc::bc_window_selected(window) } != tab {
        unsafe { bc::bc_window_select(window, tab) };
    }
}

/// The core's state of `tab`, which for a WebKit tab is its page's, or that the core let go of it.
pub fn state_changed(tab: u64, state: &bc::bc_tab_state) {
    if !unsafe { platform::host_webkit_has(tab) } {
        return;
    }
    if state.unloaded {
        close(tab);
        return;
    }
    CAN_GO_BACK.with_borrow_mut(|tabs| match state.can_go_back {
        true => tabs.insert(tab),
        false => tabs.remove(&tab),
    });
    unsafe { platform::host_webkit_set_muted(tab, state.muted) };
}

pub fn closed(tab: u64) {
    close(tab);
}

fn close(tab: u64) {
    CAN_GO_BACK.with_borrow_mut(|tabs| tabs.remove(&tab));
    unsafe { platform::host_webkit_close(tab) };
}

/// Whether `tab` is a WebKit tab, whose page the Engine's page commands go to here.
pub fn has(tab: u64) -> bool {
    unsafe { platform::host_webkit_has(tab) }
}

/// Goes back in a WebKit tab: in its page, else to the Chromium page it left.
pub fn go_back(tab: u64) {
    if unsafe { platform::host_webkit_go(tab, -1) } {
        return;
    }
    if CAN_GO_BACK.with_borrow(|tabs| tabs.contains(&tab)) {
        // check.sh reads this line.
        log!("browser: webkit {tab} back to Chromium");
        close(tab);
        unsafe { bc::bc_tab_set_native_page(tab, std::ptr::null_mut()) };
    }
}

pub fn go_forward(tab: u64) {
    unsafe { platform::host_webkit_go(tab, 1) };
}

pub fn reload(tab: u64, bypass_cache: bool) {
    unsafe { platform::host_webkit_reload(tab, bypass_cache) };
}

pub fn stop(tab: u64) {
    unsafe { platform::host_webkit_stop(tab) };
}

pub fn clear_site(tab: u64, cookies: bool) {
    unsafe { platform::host_webkit_clear_site(tab, cookies) };
}

/// The private session of `profile` ended, and with it what its WebKit tabs kept.
pub fn private_session_ended(profile: bc::bc_id) {
    if let Some(path) = profile_path(profile).and_then(|path| CString::new(path).ok()) {
        unsafe { platform::host_webkit_end_private(path.as_ptr()) };
    }
}

/// Where `profile` keeps its WebKit tabs' session, read while it still has its directory, for
/// `profile_deleted`.
pub fn profile_store(profile: bc::bc_id) -> Option<CString> {
    profile_path(profile).and_then(|path| CString::new(path).ok())
}

pub fn profile_deleted(store: &CStr) {
    unsafe { platform::host_webkit_delete_profile(store.as_ptr()) };
}
