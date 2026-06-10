use std::collections::HashMap;

use wayland_client::protocol::wl_registry;
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle, delegate_noop};
use wayland_protocols_wlr::output_management::v1::client::{
    zwlr_output_configuration_head_v1, zwlr_output_configuration_v1, zwlr_output_head_v1,
    zwlr_output_manager_v1, zwlr_output_mode_v1,
};

use crate::backend::{Output, OutputMode};

#[derive(Debug, Clone)]
struct HeadBuilder {
    name: String,
    description: String,
    make: String,
    model: String,
    serial: String,
    physical_size: String,
    position: (i32, i32),
    scale: f32,
    transform: String,
    enabled: bool,
    vrr: bool,
    modes: Vec<wayland_client::backend::ObjectId>,
    current_mode: Option<wayland_client::backend::ObjectId>,
    head_proxy: Option<zwlr_output_head_v1::ZwlrOutputHeadV1>,
}

impl Default for HeadBuilder {
    fn default() -> Self {
        Self {
            name: String::new(),
            description: String::new(),
            make: String::new(),
            model: String::new(),
            serial: String::new(),
            physical_size: String::new(),
            position: (0, 0),
            scale: 1.0,
            transform: "normal".to_string(),
            enabled: true,
            vrr: false,
            modes: Vec::new(),
            current_mode: None,
            head_proxy: None,
        }
    }
}

#[derive(Debug, Clone)]
struct ModeBuilder {
    width: i32,
    height: i32,
    refresh_rate: f32,
    preferred: bool,
    mode_proxy: Option<zwlr_output_mode_v1::ZwlrOutputModeV1>,
}

impl Default for ModeBuilder {
    fn default() -> Self {
        Self {
            width: 0,
            height: 0,
            refresh_rate: 0.0,
            preferred: false,
            mode_proxy: None,
        }
    }
}

struct WaylandState {
    output_manager: Option<zwlr_output_manager_v1::ZwlrOutputManagerV1>,
    heads: HashMap<wayland_client::backend::ObjectId, HeadBuilder>,
    modes: HashMap<wayland_client::backend::ObjectId, ModeBuilder>,
    serial: Option<u32>,
    apply_status: Option<Result<(), String>>,
}

impl Dispatch<wl_registry::WlRegistry, ()> for WaylandState {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            if interface == "zwlr_output_manager_v1" {
                let manager = registry.bind::<zwlr_output_manager_v1::ZwlrOutputManagerV1, _, _>(
                    name,
                    version.min(4),
                    qh,
                    (),
                );
                state.output_manager = Some(manager);
            }
        }
    }
}

impl Dispatch<zwlr_output_manager_v1::ZwlrOutputManagerV1, ()> for WaylandState {
    fn event(
        state: &mut Self,
        _proxy: &zwlr_output_manager_v1::ZwlrOutputManagerV1,
        event: zwlr_output_manager_v1::Event,
        _: &(),
        _: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            zwlr_output_manager_v1::Event::Head { head } => {
                let id = head.id();
                let mut builder = HeadBuilder::default();
                builder.head_proxy = Some(head);
                state.heads.insert(id, builder);
            }
            zwlr_output_manager_v1::Event::Done { serial } => {
                state.serial = Some(serial);
            }
            _ => {}
        }
    }


    wayland_client::event_created_child!(WaylandState, zwlr_output_manager_v1::ZwlrOutputManagerV1, [
        0 => (zwlr_output_head_v1::ZwlrOutputHeadV1, ())
    ]);
}

impl Dispatch<zwlr_output_head_v1::ZwlrOutputHeadV1, ()> for WaylandState {
    fn event(
        state: &mut Self,
        proxy: &zwlr_output_head_v1::ZwlrOutputHeadV1,
        event: zwlr_output_head_v1::Event,
        _: &(),
        _: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        if let zwlr_output_head_v1::Event::Finished = event {
            state.heads.remove(&proxy.id());
            if proxy.version() >= 3 {
                proxy.release();
            }
            return;
        }

        let id = proxy.id();
        let builder = match state.heads.get_mut(&id) {
            Some(b) => b,
            None => return,
        };

        match event {
            zwlr_output_head_v1::Event::Name { name } => {
                builder.name = name;
            }
            zwlr_output_head_v1::Event::Description { description } => {
                builder.description = description;
            }
            zwlr_output_head_v1::Event::PhysicalSize { width, height } => {
                builder.physical_size = format!("{}x{} mm", width, height);
            }
            zwlr_output_head_v1::Event::Mode { mode } => {
                let mode_id = mode.id();
                builder.modes.push(mode_id.clone());
                state.modes.entry(mode_id).or_default();
            }
            zwlr_output_head_v1::Event::Enabled { enabled } => {
                builder.enabled = enabled != 0;
            }
            zwlr_output_head_v1::Event::CurrentMode { mode } => {
                builder.current_mode = Some(mode.id());
            }
            zwlr_output_head_v1::Event::Position { x, y } => {
                builder.position = (x, y);
            }
            zwlr_output_head_v1::Event::Transform { transform } => {
                builder.transform = transform_to_string(
                    transform.into_result().unwrap_or(
                        wayland_client::protocol::wl_output::Transform::Normal,
                    ),
                );
            }
            zwlr_output_head_v1::Event::Scale { scale } => {
                builder.scale = scale as f32;
            }
            zwlr_output_head_v1::Event::Make { make } => {
                builder.make = make;
            }
            zwlr_output_head_v1::Event::Model { model } => {
                builder.model = model;
            }
            zwlr_output_head_v1::Event::SerialNumber { serial_number } => {
                builder.serial = serial_number;
            }
            zwlr_output_head_v1::Event::AdaptiveSync { state } => {
                builder.vrr = state.into_result().ok()
                    == Some(zwlr_output_head_v1::AdaptiveSyncState::Enabled);
            }
            _ => {}
        }
    }


    wayland_client::event_created_child!(WaylandState, zwlr_output_head_v1::ZwlrOutputHeadV1, [
        3 => (zwlr_output_mode_v1::ZwlrOutputModeV1, ())
    ]);
}

impl Dispatch<zwlr_output_mode_v1::ZwlrOutputModeV1, ()> for WaylandState {
    fn event(
        state: &mut Self,
        proxy: &zwlr_output_mode_v1::ZwlrOutputModeV1,
        event: zwlr_output_mode_v1::Event,
        _: &(),
        _: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        if let zwlr_output_mode_v1::Event::Finished = event {
            let id = proxy.id();
            state.modes.remove(&id);
            for head in state.heads.values_mut() {
                head.modes.retain(|m| *m != id);
            }
            if proxy.version() >= 3 {
                proxy.release();
            }
            return;
        }

        let id = proxy.id();
        let builder = match state.modes.get_mut(&id) {
            Some(b) => b,
            None => return,
        };

        match event {
            zwlr_output_mode_v1::Event::Size { width, height } => {
                builder.width = width;
                builder.height = height;
                builder.mode_proxy = Some(proxy.clone());
            }
            zwlr_output_mode_v1::Event::Refresh { refresh } => {
                builder.refresh_rate = (refresh as f32) / 1000.0;
            }
            zwlr_output_mode_v1::Event::Preferred => {
                builder.preferred = true;
            }
            _ => {}
        }
    }
}

impl Dispatch<zwlr_output_configuration_v1::ZwlrOutputConfigurationV1, ()> for WaylandState {
    fn event(
        state: &mut Self,
        _proxy: &zwlr_output_configuration_v1::ZwlrOutputConfigurationV1,
        event: zwlr_output_configuration_v1::Event,
        _: &(),
        _: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            zwlr_output_configuration_v1::Event::Succeeded => {
                state.apply_status = Some(Ok(()));
            }
            zwlr_output_configuration_v1::Event::Failed => {
                state.apply_status =
                    Some(Err("Configuration rejected by compositor".to_string()));
            }
            zwlr_output_configuration_v1::Event::Cancelled => {
                state.apply_status = Some(Err("Configuration cancelled".to_string()));
            }
            _ => {}
        }
    }
}

delegate_noop!(WaylandState: ignore zwlr_output_configuration_head_v1::ZwlrOutputConfigurationHeadV1);

use wayland_client::protocol::wl_output::Transform;

fn transform_to_string(transform: Transform) -> String {
    match transform {
        Transform::Normal => "normal".to_string(),
        Transform::_90 => "90".to_string(),
        Transform::_180 => "180".to_string(),
        Transform::_270 => "270".to_string(),
        Transform::Flipped => "flipped".to_string(),
        Transform::Flipped90 => "flipped-90".to_string(),
        Transform::Flipped180 => "flipped-180".to_string(),
        Transform::Flipped270 => "flipped-270".to_string(),
        _ => "normal".to_string(),
    }
}

fn string_to_transform(s: &str) -> Transform {
    match s {
        "normal" => Transform::Normal,
        "90" => Transform::_90,
        "180" => Transform::_180,
        "270" => Transform::_270,
        "flipped" => Transform::Flipped,
        "flipped-90" => Transform::Flipped90,
        "flipped-180" => Transform::Flipped180,
        "flipped-270" => Transform::Flipped270,
        _ => Transform::Normal,
    }
}

pub fn fetch_outputs() -> Result<Vec<Output>, String> {
    let conn =
        Connection::connect_to_env().map_err(|e| format!("Failed to connect to Wayland: {}", e))?;

    let mut event_queue = conn.new_event_queue();
    let qhandle = event_queue.handle();

    let display = conn.display();
    display.get_registry(&qhandle, ());

    let mut state = WaylandState {
        output_manager: None,
        heads: HashMap::new(),
        modes: HashMap::new(),
        serial: None,
        apply_status: None,
    };

    event_queue
        .roundtrip(&mut state)
        .map_err(|e| e.to_string())?;

    if state.output_manager.is_none() {
        return Err(
            "Compositor does not support wlr-output-management-unstable-v1".to_string(),
        );
    }

    event_queue
        .roundtrip(&mut state)
        .map_err(|e| e.to_string())?;
    event_queue
        .roundtrip(&mut state)
        .map_err(|e| e.to_string())?;

    Ok(state_to_outputs(&state))
}

fn state_to_outputs(state: &WaylandState) -> Vec<Output> {
    let mut outputs = Vec::new();

    for head_builder in state.heads.values() {
        let mut modes = Vec::new();
        for mode_id in &head_builder.modes {
            if let Some(mode_builder) = state.modes.get(mode_id) {
                let is_current = Some(mode_id.clone()) == head_builder.current_mode;
                modes.push(OutputMode {
                    width: mode_builder.width,
                    height: mode_builder.height,
                    refresh_rate: mode_builder.refresh_rate,
                    current: is_current,
                    preferred: mode_builder.preferred,
                });
            }
        }

        outputs.push(Output {
            name: head_builder.name.clone(),
            description: head_builder.description.clone(),
            make: head_builder.make.clone(),
            model: head_builder.model.clone(),
            serial: head_builder.serial.clone(),
            physical_size: head_builder.physical_size.clone(),
            position: head_builder.position,
            scale: head_builder.scale,
            transform: head_builder.transform.clone(),
            modes,
            enabled: head_builder.enabled,
            vrr: head_builder.vrr,
        });
    }

    outputs
}

pub fn watch_outputs(
    mut sender: iced::futures::channel::mpsc::Sender<Vec<Output>>,
) -> Result<(), String> {
    let conn =
        Connection::connect_to_env().map_err(|e| format!("Failed to connect to Wayland: {}", e))?;

    let mut event_queue = conn.new_event_queue();
    let qhandle = event_queue.handle();
    conn.display().get_registry(&qhandle, ());

    let mut state = WaylandState {
        output_manager: None,
        heads: HashMap::new(),
        modes: HashMap::new(),
        serial: None,
        apply_status: None,
    };

    event_queue
        .roundtrip(&mut state)
        .map_err(|e| e.to_string())?;
    if state.output_manager.is_none() {
        return Err("Compositor does not support wlr-output-management-unstable-v1".to_string());
    }

    let mut last_sent_serial: Option<u32> = None;
    loop {
        event_queue
            .blocking_dispatch(&mut state)
            .map_err(|e| e.to_string())?;

        if state.serial.is_some() && state.serial != last_sent_serial {
            last_sent_serial = state.serial;
            if sender.try_send(state_to_outputs(&state)).is_err() {
                return Ok(());
            }
        }
    }
}

pub fn apply_outputs(outputs: &[Output]) -> Result<(), String> {
    let conn =
        Connection::connect_to_env().map_err(|e| format!("Failed to connect to Wayland: {}", e))?;

    let mut event_queue = conn.new_event_queue();
    let qhandle = event_queue.handle();
    let display = conn.display();
    display.get_registry(&qhandle, ());

    let mut state = WaylandState {
        output_manager: None,
        heads: HashMap::new(),
        modes: HashMap::new(),
        serial: None,
        apply_status: None,
    };

    event_queue
        .roundtrip(&mut state)
        .map_err(|e| e.to_string())?;

    event_queue
        .roundtrip(&mut state)
        .map_err(|e| e.to_string())?;
    event_queue
        .roundtrip(&mut state)
        .map_err(|e| e.to_string())?;

    let manager = state.output_manager.as_ref()
        .ok_or_else(|| "Compositor does not support wlr-output-management-unstable-v1".to_string())?;
    let serial = state.serial.unwrap_or(0);
    let config = manager.create_configuration(serial, &qhandle, ());

    for out in outputs {
        let mut head_proxy = None;
        let mut head_modes = Vec::new();

        for (_, hb) in &state.heads {
            if hb.name == out.name {
                head_proxy = hb.head_proxy.clone();
                head_modes = hb.modes.clone();
                break;
            }
        }

        if let Some(proxy) = head_proxy {
            if out.enabled {
                let head_config = config.enable_head(&proxy, &qhandle, ());

                head_config.set_position(out.position.0, out.position.1);
                head_config.set_scale(out.scale as f64);
                head_config.set_transform(string_to_transform(&out.transform));

                if proxy.version() >= 4 {
                    head_config.set_adaptive_sync(if out.vrr {
                        zwlr_output_head_v1::AdaptiveSyncState::Enabled
                    } else {
                        zwlr_output_head_v1::AdaptiveSyncState::Disabled
                    });
                }

                if let Some(active_mode) = out.modes.iter().find(|m| m.current) {
                    let mut found_proxy = None;

                    for mode_id in &head_modes {
                        if let Some(mode_builder) = state.modes.get(mode_id) {
                            if mode_builder.width == active_mode.width
                                && mode_builder.height == active_mode.height
                                && (mode_builder.refresh_rate - active_mode.refresh_rate).abs()
                                    < 0.5
                            {
                                found_proxy = mode_builder.mode_proxy.clone();
                                break;
                            }
                        }
                    }

                    if let Some(mp) = found_proxy {
                        head_config.set_mode(&mp);
                    } else {
                        head_config.set_custom_mode(
                            active_mode.width,
                            active_mode.height,
                            (active_mode.refresh_rate * 1000.0) as i32,
                        );
                    }
                }
            } else {
                config.disable_head(&proxy);
            }
        }
    }

    config.apply();

    loop {
        event_queue
            .blocking_dispatch(&mut state)
            .map_err(|e| e.to_string())?;
        if state.apply_status.is_some() {
            break;
        }
    }

    state.apply_status.ok_or_else(|| "Compositor did not send apply status".to_string())?
}
