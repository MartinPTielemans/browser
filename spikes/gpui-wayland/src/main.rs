//! Native Wayland spike: GPUI draws through another client's Wayland connection, as a subsurface of
//! that client's window, as it would inside Chromium (decision 3). The host stands in for Chromium:
//! it owns the display, the toplevel and the keyboard, runs the loop, and paints the page, right of
//! GPUI's sidebar. Tab moves the keyboard between the sidebar and the page, as the host decides.
//!
//! It prints `spike: ` lines, which check.sh reads. SPIKE_NAIVE=1 dispatches GPUI only when its own
//! file descriptor is readable, SPIKE_ROUNDTRIP=1 adds a roundtrip each turn, as Chromium makes some
//! of its requests, which reads GPUI's events off the socket outside the poll, and SPIKE_HOLD_READ=1
//! calls GPUI while the host has prepared a read: the ways of getting two readers of one display
//! wrong. SPIKE_STANDALONE=1 runs GPUI alone, in a window of its own, for the frame rate to compare.

use std::{
    cell::Cell,
    os::fd::{AsFd, AsRawFd, FromRawFd},
    time::{Duration, Instant},
};

use gpui::{
    App, Bounds, Context, FocusHandle, KeyDownEvent, Window, WindowBounds, WindowOptions, div,
    point, prelude::*, px, rgb, size,
};
use wayland_client::{
    Connection, Dispatch, Proxy, QueueHandle, WEnum,
    globals::{GlobalListContents, registry_queue_init},
    protocol::{
        wl_buffer, wl_compositor, wl_keyboard, wl_pointer, wl_registry, wl_seat, wl_shm,
        wl_shm_pool, wl_surface,
    },
};
use wayland_protocols::xdg::shell::client::{xdg_surface, xdg_toplevel, xdg_wm_base};

const SIDEBAR: i32 = 280;
const KEY_TAB: u32 = 15;

struct Host {
    surface: wl_surface::WlSurface,
    shm: wl_shm::WlShm,
    xdg_surface: xdg_surface::XdgSurface,
    size: (i32, i32),
    configured: bool,
    painted: Option<(i32, i32)>,
    ui_has_keyboard: bool,
    keyboard_serial: Option<u32>,
    page_keys: u32,
}

fn main() {
    let naive = std::env::var_os("SPIKE_NAIVE").is_some();
    let hold_read = std::env::var_os("SPIKE_HOLD_READ").is_some();
    let roundtrip = std::env::var_os("SPIKE_ROUNDTRIP").is_some();
    if std::env::var_os("SPIKE_STANDALONE").is_some() {
        return standalone();
    }
    let conn = Connection::connect_to_env().expect("no Wayland display");
    let (globals, mut queue) = registry_queue_init::<Host>(&conn).expect("no registry");
    let qh = queue.handle();
    let compositor: wl_compositor::WlCompositor = globals.bind(&qh, 4..=6, ()).unwrap();
    let shm: wl_shm::WlShm = globals.bind(&qh, 1..=1, ()).unwrap();
    let wm_base: xdg_wm_base::XdgWmBase = globals.bind(&qh, 1..=6, ()).unwrap();
    let _seat: wl_seat::WlSeat = globals.bind(&qh, 5..=9, ()).unwrap();

    let surface = compositor.create_surface(&qh, ());
    let xdg_surface = wm_base.get_xdg_surface(&surface, &qh, ());
    let toplevel = xdg_surface.get_toplevel(&qh, ());
    toplevel.set_title("gpui-wayland spike".into());
    toplevel.set_app_id("ilo-gpui-wayland-spike".into());
    surface.commit();

    let mut host = Host {
        surface: surface.clone(),
        shm,
        xdg_surface,
        size: (1000, 640),
        configured: false,
        painted: None,
        ui_has_keyboard: true,
        keyboard_serial: None,
        page_keys: 0,
    };
    while !host.configured {
        queue.blocking_dispatch(&mut host).unwrap();
    }
    host.paint(&qh);

    // GPUI on the host's display, in a subsurface of its window: what Chromium would hand it.
    gpui_linux::embed_wayland(conn.backend().display_ptr().cast());
    let height = host.size.1;
    let surface_ptr = surface.id().as_ptr().cast();
    let _app = gpui_platform::application().run_embedded(move |cx: &mut App| {
        gpui_linux::open_next_window_in_surface(surface_ptr, 0, 0);
        let bounds = Bounds::new(point(px(0.), px(0.)), size(px(SIDEBAR as f32), px(height as f32)));
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |window, cx| {
                cx.new(|cx| {
                    let focus = cx.focus_handle();
                    window.focus(&focus, cx);
                    Sidebar::new(focus)
                })
            },
        )
        .unwrap();
    });
    let gpui_fd = gpui_linux::embedded_fd().expect("GPUI isn't embedded");
    println!("spike: started");

    let wayland_fd = conn.as_fd().as_raw_fd();
    let mut woken_by_host_read = 0u32;
    let mut last_report = Instant::now();
    loop {
        queue.dispatch_pending(&mut host).unwrap();
        host.paint(&qh);
        conn.flush().ok();
        let guard = queue.prepare_read();
        if hold_read && guard.is_some() {
            // Another reader that prepares while this one holds a prepared read waits for it.
            unsafe { libc::alarm(5) };
            println!("spike: calling GPUI while holding a prepared read");
            gpui_linux::dispatch_embedded();
            println!("spike: GPUI returned while a read was held");
        }
        let mut fds = [
            libc::pollfd { fd: wayland_fd, events: libc::POLLIN, revents: 0 },
            libc::pollfd { fd: gpui_fd, events: libc::POLLIN, revents: 0 },
        ];
        unsafe { libc::poll(fds.as_mut_ptr(), 2, 1000) };
        let mut host_read = false;
        if let Some(guard) = guard {
            if fds[0].revents & libc::POLLIN != 0 {
                host_read = guard.read().is_ok();
            }
        }
        queue.dispatch_pending(&mut host).unwrap();
        if roundtrip {
            queue.roundtrip(&mut host).unwrap();
        }
        let gpui_ready = fds[1].revents & libc::POLLIN != 0;
        // What the host read can include GPUI's events, which then wait in GPUI's queue with
        // nothing left on the socket to wake it.
        if gpui_ready || (host_read && !naive) {
            if !gpui_ready {
                woken_by_host_read += 1;
            }
            gpui_linux::dispatch_embedded();
        }
        if last_report.elapsed() >= Duration::from_secs(1) {
            println!(
                "spike: frames {} in the last second, {} GPUI dispatches only the host's read woke",
                FRAMES.replace(0),
                woken_by_host_read
            );
            woken_by_host_read = 0;
            last_report = Instant::now();
        }
    }
}

fn standalone() {
    gpui_platform::application().run(|cx: &mut App| {
        let bounds = Bounds::new(point(px(0.), px(0.)), size(px(SIDEBAR as f32), px(480.)));
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                app_id: Some("ilo-gpui-wayland-spike".into()),
                ..Default::default()
            },
            |window, cx| {
                cx.new(|cx| {
                    let focus = cx.focus_handle();
                    window.focus(&focus, cx);
                    Sidebar::new(focus)
                })
            },
        )
        .unwrap();
        println!("spike: started");
        cx.spawn(async |cx| {
            loop {
                cx.background_executor().timer(Duration::from_secs(1)).await;
                println!("spike: frames {} in the last second", FRAMES.replace(0));
            }
        })
        .detach();
    });
}

thread_local! {
    static FRAMES: Cell<u32> = const { Cell::new(0) };
}

struct Sidebar {
    focus: FocusHandle,
    typed: String,
    hovered: bool,
    drawn: Option<(i32, i32)>,
}

impl Sidebar {
    fn new(focus: FocusHandle) -> Self {
        Self { focus, typed: String::new(), hovered: false, drawn: None }
    }
}

impl Render for Sidebar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let viewport = window.viewport_size();
        let drawn = (f32::from(viewport.width) as i32, f32::from(viewport.height) as i32);
        if self.drawn != Some(drawn) {
            self.drawn = Some(drawn);
            println!("spike: ui drawn at {}x{}", drawn.0, drawn.1);
        }
        FRAMES.set(FRAMES.get() + 1);
        // Draws every frame, so the frame count shows whether GPUI's frame callbacks arrive.
        window.request_animation_frame();
        let focused = self.focus.is_focused(window);
        div()
            .id("sidebar")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                println!("spike: ui key {}", event.keystroke.key);
                this.typed.push_str(&event.keystroke.key);
                cx.notify();
            }))
            .on_hover(cx.listener(|this, hovered: &bool, _, cx| {
                println!("spike: ui pointer {}", if *hovered { "entered" } else { "left" });
                this.hovered = *hovered;
                cx.notify();
            }))
            .size_full()
            .flex()
            .flex_col()
            .gap_2()
            .p_4()
            .bg(if self.hovered { rgb(0x3b3f5c) } else { rgb(0x2b2d42) })
            .text_color(rgb(0xf0f0f0))
            .child("GPUI sidebar")
            .child(format!("keyboard: {}", if focused && window.is_window_active() { "here" } else { "page" }))
            .child(format!("typed: {}", self.typed))
    }
}

impl Host {
    /// The page: everything right of the sidebar, in a colour that changes with each key it gets.
    fn paint(&mut self, qh: &QueueHandle<Host>) {
        let (width, height) = self.size;
        if !self.configured || self.painted == Some((width, height + self.page_keys as i32 * 10000)) {
            return;
        }
        self.painted = Some((width, height + self.page_keys as i32 * 10000));
        let stride = width * 4;
        let len = (stride * height) as usize;
        let fd = unsafe { libc::memfd_create(c"spike-page".as_ptr(), libc::MFD_CLOEXEC) };
        assert!(fd >= 0);
        let fd = unsafe { std::os::fd::OwnedFd::from_raw_fd(fd) };
        let file = std::fs::File::from(fd);
        file.set_len(len as u64).unwrap();
        let shade = 0xe0 - (self.page_keys * 24 % 0xa0);
        let pixel = 0xff00_0000 | shade << 16 | shade << 8 | 0xff;
        let mut bytes = Vec::with_capacity(len);
        for _ in 0..(width * height) {
            bytes.extend_from_slice(&pixel.to_le_bytes());
        }
        std::os::unix::fs::FileExt::write_all_at(&file, &bytes, 0).unwrap();
        let pool = self.shm.create_pool(file.as_fd(), len as i32, qh, ());
        let buffer = pool.create_buffer(0, width, height, stride, wl_shm::Format::Argb8888, qh, ());
        pool.destroy();
        self.surface.attach(Some(&buffer), 0, 0);
        self.surface.damage_buffer(0, 0, width, height);
        self.surface.commit();
    }

    fn forward(&self, event: wl_keyboard::Event) {
        gpui_linux::forward_keyboard(event);
    }
}

impl Dispatch<wl_keyboard::WlKeyboard, ()> for Host {
    fn event(
        host: &mut Self,
        _: &wl_keyboard::WlKeyboard,
        event: wl_keyboard::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            wl_keyboard::Event::Enter { serial, .. } => {
                host.keyboard_serial = Some(serial);
                println!("spike: host has the keyboard");
                if host.ui_has_keyboard {
                    host.forward(event);
                }
            }
            wl_keyboard::Event::Leave { .. } => {
                host.keyboard_serial = None;
                if host.ui_has_keyboard {
                    host.forward(event);
                }
            }
            wl_keyboard::Event::Key { key: KEY_TAB, state: WEnum::Value(state), serial, .. } => {
                if state == wl_keyboard::KeyState::Pressed {
                    host.ui_has_keyboard = !host.ui_has_keyboard;
                    println!(
                        "spike: keyboard to the {}",
                        if host.ui_has_keyboard { "sidebar" } else { "page" }
                    );
                    let surface = host.surface.clone();
                    host.forward(if host.ui_has_keyboard {
                        wl_keyboard::Event::Enter { serial, surface, keys: Vec::new() }
                    } else {
                        wl_keyboard::Event::Leave { serial, surface }
                    });
                }
            }
            wl_keyboard::Event::Key { key, state, .. } if !host.ui_has_keyboard => {
                if state == WEnum::Value(wl_keyboard::KeyState::Pressed) {
                    host.page_keys += 1;
                    println!("spike: page key {key}");
                }
            }
            // The keymap, modifiers and repeat rate, and keys while the sidebar has the keyboard.
            event => host.forward(event),
        }
    }
}

impl Dispatch<wl_pointer::WlPointer, ()> for Host {
    fn event(
        host: &mut Self,
        _: &wl_pointer::WlPointer,
        event: wl_pointer::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wl_pointer::Event::Enter { surface, .. } = event {
            // Every wl_pointer of a client hears of the surface the pointer enters, the
            // sidebar's included, so the host sees GPUI's surface go by and leaves it to GPUI.
            if surface == host.surface {
                println!("spike: host pointer entered the page");
            } else {
                println!("spike: host pointer entered another surface of this client");
            }
        }
    }
}

impl Dispatch<wl_seat::WlSeat, ()> for Host {
    fn event(
        _: &mut Self,
        seat: &wl_seat::WlSeat,
        event: wl_seat::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_seat::Event::Capabilities { capabilities: WEnum::Value(capabilities) } = event {
            if capabilities.contains(wl_seat::Capability::Keyboard) {
                seat.get_keyboard(qh, ());
            }
            if capabilities.contains(wl_seat::Capability::Pointer) {
                seat.get_pointer(qh, ());
            }
        }
    }
}

impl Dispatch<xdg_wm_base::XdgWmBase, ()> for Host {
    fn event(
        _: &mut Self,
        wm_base: &xdg_wm_base::XdgWmBase,
        event: xdg_wm_base::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let xdg_wm_base::Event::Ping { serial } = event {
            wm_base.pong(serial);
        }
    }
}

impl Dispatch<xdg_toplevel::XdgToplevel, ()> for Host {
    fn event(
        host: &mut Self,
        _: &xdg_toplevel::XdgToplevel,
        event: xdg_toplevel::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            xdg_toplevel::Event::Configure { width, height, .. } if width > 0 && height > 0 => {
                host.size = (width, height);
            }
            xdg_toplevel::Event::Close => std::process::exit(0),
            _ => {}
        }
    }
}

impl Dispatch<xdg_surface::XdgSurface, ()> for Host {
    fn event(
        host: &mut Self,
        _: &xdg_surface::XdgSurface,
        event: xdg_surface::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let xdg_surface::Event::Configure { serial } = event {
            host.xdg_surface.ack_configure(serial);
            host.configured = true;
            println!("spike: host configured at {}x{}", host.size.0, host.size.1);
            gpui_linux::resize_embedded(SIDEBAR as f32, host.size.1 as f32);
        }
    }
}

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for Host {
    fn event(
        _: &mut Self,
        _: &wl_registry::WlRegistry,
        _: wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

wayland_client::delegate_noop!(Host: ignore wl_compositor::WlCompositor);
wayland_client::delegate_noop!(Host: ignore wl_surface::WlSurface);
wayland_client::delegate_noop!(Host: ignore wl_shm::WlShm);
wayland_client::delegate_noop!(Host: ignore wl_shm_pool::WlShmPool);
wayland_client::delegate_noop!(Host: ignore wl_buffer::WlBuffer);
