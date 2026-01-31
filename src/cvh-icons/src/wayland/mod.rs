//! Wayland integration for desktop icons
//!
//! Uses layer-shell protocol for desktop-level surfaces.
//! Icons are rendered as layer-shell surfaces on the background layer.

use anyhow::{Context, Result};
use std::collections::HashMap;
use tracing::{debug, info};

use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_layer, delegate_output, delegate_pointer, delegate_registry,
    delegate_seat, delegate_shm,
    output::{OutputHandler, OutputState},
    reexports::{
        calloop::{EventLoop, LoopHandle},
        calloop_wayland_source::WaylandSource,
        client::{
            globals::registry_queue_init,
            protocol::{
                wl_output::WlOutput,
                wl_pointer::WlPointer,
                wl_seat::WlSeat,
                wl_shm,
                wl_surface::WlSurface,
            },
            Connection, QueueHandle,
        },
    },
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        pointer::{PointerEvent, PointerEventKind, PointerHandler},
        Capability, SeatHandler, SeatState,
    },
    shell::{
        wlr_layer::{
            Anchor, KeyboardInteractivity, Layer, LayerShell, LayerShellHandler, LayerSurface,
            LayerSurfaceConfigure,
        },
        WaylandSurface,
    },
    shm::{
        slot::{Buffer, SlotPool},
        Shm, ShmHandler,
    },
};

/// Unique identifier for icon surfaces
pub type SurfaceId = u64;

/// Input event from Wayland
#[derive(Debug, Clone)]
pub enum InputEvent {
    /// Pointer entered a surface
    PointerEnter {
        surface_id: SurfaceId,
        x: f64,
        y: f64,
    },
    /// Pointer left a surface
    PointerLeave { surface_id: SurfaceId },
    /// Pointer moved on a surface
    PointerMotion {
        surface_id: SurfaceId,
        x: f64,
        y: f64,
    },
    /// Pointer button pressed/released
    PointerButton {
        surface_id: SurfaceId,
        button: u32,
        pressed: bool,
        x: f64,
        y: f64,
    },
}

/// Icon surface data
struct IconSurfaceData {
    layer_surface: LayerSurface,
    width: u32,
    height: u32,
    configured: bool,
    buffer: Option<Buffer>,
    #[allow(dead_code)]
    position_x: i32,
    #[allow(dead_code)]
    position_y: i32,
}

/// Wayland application state
pub struct WaylandState {
    /// Registry state
    registry_state: RegistryState,
    /// Compositor state
    compositor_state: CompositorState,
    /// Output state
    output_state: OutputState,
    /// Layer shell
    layer_shell: LayerShell,
    /// Shared memory
    shm: Shm,
    /// Seat state
    seat_state: SeatState,
    /// Buffer pool
    pool: SlotPool,
    /// Queue handle
    queue_handle: QueueHandle<Self>,
    /// Map of surface ID to surface data
    surfaces: HashMap<SurfaceId, IconSurfaceData>,
    /// Map of WlSurface to surface ID (for event routing)
    surface_ids: HashMap<WlSurface, SurfaceId>,
    /// Next surface ID
    next_surface_id: SurfaceId,
    /// Available outputs
    outputs: Vec<WlOutput>,
    /// Current pointer
    pointer: Option<WlPointer>,
    /// Pointer position
    pointer_x: f64,
    pointer_y: f64,
    /// Surface under pointer
    pointer_surface: Option<SurfaceId>,
    /// Pending input events
    input_events: Vec<InputEvent>,
    /// Whether to exit
    exit: bool,
}

impl WaylandState {
    /// Create a new surface for an icon
    pub fn create_surface(&mut self, x: i32, y: i32, width: u32, height: u32) -> Result<SurfaceId> {
        let surface_id = self.next_surface_id;
        self.next_surface_id += 1;

        // Get the first output (or create surface without specific output)
        let output = self.outputs.first().cloned();

        // Create the wl_surface
        let wl_surface = self.compositor_state.create_surface(&self.queue_handle);

        // Create layer surface on background layer
        let layer_surface = self.layer_shell.create_layer_surface(
            &self.queue_handle,
            wl_surface.clone(),
            Layer::Background,
            Some("cvh-icon"),
            output.as_ref(),
        );

        // Configure layer surface
        layer_surface.set_anchor(Anchor::TOP | Anchor::LEFT);
        layer_surface.set_exclusive_zone(-1); // Don't reserve space
        layer_surface.set_size(width, height);
        layer_surface.set_margin(y, 0, 0, x); // top, right, bottom, left margins for positioning
        layer_surface.set_keyboard_interactivity(KeyboardInteractivity::None);

        // Commit initial state
        layer_surface.commit();

        // Store surface data
        let surface_data = IconSurfaceData {
            layer_surface,
            width,
            height,
            configured: false,
            buffer: None,
            position_x: x,
            position_y: y,
        };

        self.surfaces.insert(surface_id, surface_data);
        self.surface_ids.insert(wl_surface, surface_id);

        debug!("Created surface {} at ({}, {}) size {}x{}", surface_id, x, y, width, height);

        Ok(surface_id)
    }

    /// Destroy a surface
    pub fn destroy_surface(&mut self, surface_id: SurfaceId) {
        if let Some(surface_data) = self.surfaces.remove(&surface_id) {
            // Find and remove the WlSurface entry
            let wl_surface = surface_data.layer_surface.wl_surface().clone();
            self.surface_ids.remove(&wl_surface);

            debug!("Destroyed surface {}", surface_id);
        }
    }

    /// Set surface position (via margins)
    pub fn set_surface_position(&mut self, surface_id: SurfaceId, x: i32, y: i32) {
        if let Some(surface_data) = self.surfaces.get_mut(&surface_id) {
            surface_data.position_x = x;
            surface_data.position_y = y;
            // Layer-shell uses margins for positioning relative to anchor
            surface_data.layer_surface.set_margin(y, 0, 0, x);
            surface_data.layer_surface.commit();
        }
    }

    /// Attach a pixmap buffer to a surface
    pub fn attach_buffer(&mut self, surface_id: SurfaceId, pixels: &[u8], width: u32, height: u32) -> Result<()> {
        let surface_data = self.surfaces.get_mut(&surface_id)
            .ok_or_else(|| anyhow::anyhow!("Surface {} not found", surface_id))?;

        if !surface_data.configured {
            // Wait for configure event before attaching buffer
            debug!("Surface {} not yet configured, skipping buffer attach", surface_id);
            return Ok(());
        }

        // Ensure buffer size matches
        let expected_size = (width * height * 4) as usize;
        if pixels.len() != expected_size {
            return Err(anyhow::anyhow!(
                "Buffer size mismatch: got {} bytes, expected {} bytes ({}x{}x4)",
                pixels.len(), expected_size, width, height
            ));
        }

        // Create or reuse buffer
        let (buffer, canvas) = self.pool
            .create_buffer(
                width as i32,
                height as i32,
                width as i32 * 4,
                wl_shm::Format::Argb8888,
            )
            .context("Failed to create buffer")?;

        // Copy pixels (converting from RGBA to ARGB if needed)
        // tiny-skia uses RGBA premultiplied, Wayland expects ARGB
        for (i, chunk) in pixels.chunks(4).enumerate() {
            let r = chunk[0];
            let g = chunk[1];
            let b = chunk[2];
            let a = chunk[3];
            let offset = i * 4;
            canvas[offset] = b;     // B
            canvas[offset + 1] = g; // G
            canvas[offset + 2] = r; // R
            canvas[offset + 3] = a; // A
        }

        // Attach and commit
        let wl_surface = surface_data.layer_surface.wl_surface();
        buffer.attach_to(wl_surface).context("Failed to attach buffer")?;
        wl_surface.damage_buffer(0, 0, width as i32, height as i32);
        wl_surface.commit();

        // Store buffer reference to keep it alive
        surface_data.buffer = Some(buffer);

        Ok(())
    }

    /// Get pending input events (drains the queue)
    pub fn take_input_events(&mut self) -> Vec<InputEvent> {
        std::mem::take(&mut self.input_events)
    }

    /// Check if should exit
    pub fn should_exit(&self) -> bool {
        self.exit
    }

    /// Request exit
    #[allow(dead_code)]
    pub fn request_exit(&mut self) {
        self.exit = true;
    }

    /// Get surface IDs
    pub fn surface_ids(&self) -> Vec<SurfaceId> {
        self.surfaces.keys().copied().collect()
    }
}

// Implement required trait delegates

impl CompositorHandler for WaylandState {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &WlSurface,
        _new_factor: i32,
    ) {
        // Handle scale factor changes if needed
    }

    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &WlSurface,
        _new_transform: smithay_client_toolkit::reexports::client::protocol::wl_output::Transform,
    ) {
        // Handle transform changes if needed
    }

    fn frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &WlSurface,
        _time: u32,
    ) {
        // Handle frame callbacks
    }

    fn surface_enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &WlSurface,
        _output: &WlOutput,
    ) {
    }

    fn surface_leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &WlSurface,
        _output: &WlOutput,
    ) {
    }
}

impl OutputHandler for WaylandState {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        output: WlOutput,
    ) {
        info!("New output detected");
        self.outputs.push(output);
    }

    fn update_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: WlOutput,
    ) {
        // Handle output updates (dimensions may have changed)
        debug!("Output updated");
    }

    fn output_destroyed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        output: WlOutput,
    ) {
        info!("Output destroyed");
        self.outputs.retain(|o| o != &output);
    }
}

impl WaylandState {
    /// Get the dimensions of the primary output
    pub fn get_output_dimensions(&self) -> Option<(u32, u32)> {
        // Get the first output's info
        if let Some(output) = self.outputs.first() {
            if let Some(info) = self.output_state.info(output) {
                // Get the logical size (respects scaling)
                if let Some(logical_size) = info.logical_size {
                    return Some((logical_size.0 as u32, logical_size.1 as u32));
                }
                // Fall back to physical mode size if logical not available
                if let Some(mode) = info.modes.iter().find(|m| m.current) {
                    return Some((mode.dimensions.0 as u32, mode.dimensions.1 as u32));
                }
            }
        }
        None
    }
}

impl LayerShellHandler for WaylandState {
    fn closed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, layer: &LayerSurface) {
        // Find the surface that was closed
        let wl_surface = layer.wl_surface();
        if let Some(&surface_id) = self.surface_ids.get(wl_surface) {
            debug!("Layer surface {} closed", surface_id);
            self.surfaces.remove(&surface_id);
            self.surface_ids.remove(wl_surface);
        }
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        layer: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _serial: u32,
    ) {
        let wl_surface = layer.wl_surface();
        if let Some(&surface_id) = self.surface_ids.get(wl_surface) {
            if let Some(surface_data) = self.surfaces.get_mut(&surface_id) {
                // Update size if the compositor requested a different size
                if configure.new_size.0 > 0 {
                    surface_data.width = configure.new_size.0;
                }
                if configure.new_size.1 > 0 {
                    surface_data.height = configure.new_size.1;
                }
                surface_data.configured = true;
                debug!(
                    "Surface {} configured with size {}x{}",
                    surface_id, surface_data.width, surface_data.height
                );

                // Acknowledge configure
                layer.wl_surface().commit();
            }
        }
    }
}

impl SeatHandler for WaylandState {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }

    fn new_seat(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _seat: WlSeat) {
        // New seat available
    }

    fn new_capability(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        seat: WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Pointer && self.pointer.is_none() {
            debug!("Creating pointer for seat");
            self.pointer = self.seat_state.get_pointer(qh, &seat).ok();
        }
    }

    fn remove_capability(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _seat: WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Pointer {
            self.pointer = None;
        }
    }

    fn remove_seat(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _seat: WlSeat) {
        // Seat removed
    }
}

impl PointerHandler for WaylandState {
    fn pointer_frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _pointer: &WlPointer,
        events: &[PointerEvent],
    ) {
        for event in events {
            let (x, y) = event.position;
            let surface = &event.surface;

            match &event.kind {
                PointerEventKind::Enter { .. } => {
                    self.pointer_x = x;
                    self.pointer_y = y;
                    if let Some(&surface_id) = self.surface_ids.get(surface) {
                        self.pointer_surface = Some(surface_id);
                        self.input_events.push(InputEvent::PointerEnter {
                            surface_id,
                            x,
                            y,
                        });
                    }
                }
                PointerEventKind::Leave { .. } => {
                    if let Some(&surface_id) = self.surface_ids.get(surface) {
                        self.pointer_surface = None;
                        self.input_events.push(InputEvent::PointerLeave { surface_id });
                    }
                }
                PointerEventKind::Motion { .. } => {
                    self.pointer_x = x;
                    self.pointer_y = y;
                    if let Some(surface_id) = self.pointer_surface {
                        self.input_events.push(InputEvent::PointerMotion {
                            surface_id,
                            x,
                            y,
                        });
                    }
                }
                PointerEventKind::Press { button, .. } => {
                    if let Some(surface_id) = self.pointer_surface {
                        self.input_events.push(InputEvent::PointerButton {
                            surface_id,
                            button: *button,
                            pressed: true,
                            x: self.pointer_x,
                            y: self.pointer_y,
                        });
                    }
                }
                PointerEventKind::Release { button, .. } => {
                    if let Some(surface_id) = self.pointer_surface {
                        self.input_events.push(InputEvent::PointerButton {
                            surface_id,
                            button: *button,
                            pressed: false,
                            x: self.pointer_x,
                            y: self.pointer_y,
                        });
                    }
                }
                PointerEventKind::Axis { .. } => {
                    // Scroll events - not handling for now
                }
            }
        }
    }
}

impl ShmHandler for WaylandState {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

impl ProvidesRegistryState for WaylandState {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState, SeatState];
}

delegate_compositor!(WaylandState);
delegate_output!(WaylandState);
delegate_layer!(WaylandState);
delegate_seat!(WaylandState);
delegate_pointer!(WaylandState);
delegate_shm!(WaylandState);
delegate_registry!(WaylandState);

/// Wayland manager - high level interface for the daemon
pub struct WaylandManager {
    /// The event loop
    event_loop: EventLoop<'static, WaylandState>,
    /// Wayland state (shared with event loop)
    state: WaylandState,
}

impl WaylandManager {
    /// Connect to Wayland display and initialize
    pub fn new() -> Result<Self> {
        // Check for Wayland display
        if std::env::var("WAYLAND_DISPLAY").is_err() {
            return Err(anyhow::anyhow!("WAYLAND_DISPLAY not set - not running under Wayland"));
        }

        // Connect to the Wayland display
        let conn = Connection::connect_to_env()
            .context("Failed to connect to Wayland display")?;

        info!("Connected to Wayland display");

        // Initialize the registry
        let (globals, event_queue) = registry_queue_init(&conn)
            .context("Failed to initialize registry")?;

        let qh = event_queue.handle();

        // Get compositor state
        let compositor_state = CompositorState::bind(&globals, &qh)
            .context("Failed to bind compositor")?;

        // Get output state
        let output_state = OutputState::new(&globals, &qh);

        // Get layer shell
        let layer_shell = LayerShell::bind(&globals, &qh)
            .context("Failed to bind layer shell - is wlr-layer-shell-unstable-v1 supported?")?;

        // Get shared memory
        let shm = Shm::bind(&globals, &qh)
            .context("Failed to bind shm")?;

        // Get seat state
        let seat_state = SeatState::new(&globals, &qh);

        // Create buffer pool (initial size 1MB, will grow as needed)
        let pool = SlotPool::new(1024 * 1024, &shm)
            .context("Failed to create buffer pool")?;

        // Create calloop event loop
        let event_loop: EventLoop<WaylandState> = EventLoop::try_new()
            .context("Failed to create event loop")?;

        // Create state
        let state = WaylandState {
            registry_state: RegistryState::new(&globals),
            compositor_state,
            output_state,
            layer_shell,
            shm,
            seat_state,
            pool,
            queue_handle: qh.clone(),
            surfaces: HashMap::new(),
            surface_ids: HashMap::new(),
            next_surface_id: 1,
            outputs: Vec::new(),
            pointer: None,
            pointer_x: 0.0,
            pointer_y: 0.0,
            pointer_surface: None,
            input_events: Vec::new(),
            exit: false,
        };

        // Insert Wayland source into calloop
        let wayland_source = WaylandSource::new(conn.clone(), event_queue);
        let loop_handle = event_loop.handle();
        loop_handle
            .insert_source(wayland_source, |_, queue, state| {
                queue.dispatch_pending(state)
            })
            .map_err(|e| anyhow::anyhow!("Failed to insert Wayland source: {:?}", e))?;

        info!("Wayland manager initialized");

        Ok(Self { event_loop, state })
    }

    /// Create a new surface for an icon
    pub fn create_surface(&mut self, x: i32, y: i32, width: u32, height: u32) -> Result<SurfaceId> {
        self.state.create_surface(x, y, width, height)
    }

    /// Destroy a surface
    pub fn destroy_surface(&mut self, surface_id: SurfaceId) {
        self.state.destroy_surface(surface_id)
    }

    /// Set surface position
    pub fn set_surface_position(&mut self, surface_id: SurfaceId, x: i32, y: i32) {
        self.state.set_surface_position(surface_id, x, y)
    }

    /// Attach a buffer to a surface (pixels in RGBA format)
    pub fn attach_buffer(&mut self, surface_id: SurfaceId, pixels: &[u8], width: u32, height: u32) -> Result<()> {
        self.state.attach_buffer(surface_id, pixels, width, height)
    }

    /// Dispatch Wayland events (non-blocking)
    pub fn dispatch_events(&mut self) -> Result<()> {
        self.event_loop
            .dispatch(Some(std::time::Duration::ZERO), &mut self.state)
            .context("Failed to dispatch Wayland events")?;
        Ok(())
    }

    /// Get pending input events
    pub fn take_input_events(&mut self) -> Vec<InputEvent> {
        self.state.take_input_events()
    }

    /// Get the calloop handle for integrating with external event sources
    #[allow(dead_code)]
    pub fn loop_handle(&self) -> LoopHandle<'static, WaylandState> {
        self.event_loop.handle()
    }

    /// Check if should exit
    pub fn should_exit(&self) -> bool {
        self.state.should_exit()
    }

    /// Get all surface IDs
    #[allow(dead_code)]
    pub fn surface_ids(&self) -> Vec<SurfaceId> {
        self.state.surface_ids()
    }

    /// Get the dimensions of the primary output
    pub fn get_output_dimensions(&self) -> Option<(u32, u32)> {
        self.state.get_output_dimensions()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_event_debug() {
        let event = InputEvent::PointerEnter {
            surface_id: 1,
            x: 10.0,
            y: 20.0,
        };
        let debug_str = format!("{:?}", event);
        assert!(debug_str.contains("PointerEnter"));
        assert!(debug_str.contains("surface_id: 1"));
    }

    #[test]
    fn test_input_event_clone() {
        let event = InputEvent::PointerButton {
            surface_id: 1,
            button: 272,
            pressed: true,
            x: 10.0,
            y: 20.0,
        };
        let cloned = event.clone();
        match cloned {
            InputEvent::PointerButton { surface_id, button, pressed, .. } => {
                assert_eq!(surface_id, 1);
                assert_eq!(button, 272);
                assert!(pressed);
            }
            _ => panic!("Expected PointerButton"),
        }
    }

    // Note: WaylandManager tests require a running Wayland display
    // and are better suited for integration testing
}
