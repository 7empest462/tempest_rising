use bevy::prelude::*;
use bevy::render::{
    Render, RenderApp, RenderSystems,
    extract_resource::{ExtractResource, ExtractResourcePlugin},
    gpu_readback::{Readback, ReadbackComplete},
    render_asset::RenderAssets,
    render_resource::{
        binding_types::{storage_buffer, storage_buffer_read_only, uniform_buffer},
        *,
    },
    renderer::{RenderDevice, RenderQueue},
    storage::{GpuShaderBuffer, ShaderBuffer},
};
use std::borrow::Cow;

pub const WATER_GRID_SIZE: u32 = 256;
pub const WORKGROUP_SIZE: u32 = 8;

const SHADER_ASSET_PATH: &str = "shaders/water_compute.wgsl";

// --- Physics Parameters ---
#[derive(ShaderType, Default, Clone, Debug)]
pub struct WaterInteractorData {
    pub grid_x: f32,
    pub grid_z: f32,
    pub push_force: f32,
    pub push_radius: f32,
    pub swim_add_height: f32,
    pub swim_radius: f32,
    pub _pad1: f32,
    pub _pad2: f32,
}

#[derive(ShaderType, Default, Clone, Debug)]
pub struct WaterImpulseData {
    pub grid_x: f32,
    pub grid_z: f32,
    pub force: f32,
    pub radius: f32,
}

#[derive(ShaderType, Resource, Clone, ExtractResource, Debug)]
pub struct WaterSimParams {
    pub delta_time: f32,
    pub gravity: f32,
    pub friction: f32,
    pub interactor_count: u32,
    pub interactors: [WaterInteractorData; 16],
    pub impulse_count: u32,
    pub _pad1: f32,
    pub _pad2: f32,
    pub _pad3: f32,
    pub impulses: [WaterImpulseData; 8],
}

impl Default for WaterSimParams {
    fn default() -> Self {
        Self {
            delta_time: 0.016,
            gravity: 9.8,
            friction: 0.999,
            interactor_count: 0,
            interactors: core::array::from_fn(|_| WaterInteractorData::default()),
            impulse_count: 0,
            _pad1: 0.0,
            _pad2: 0.0,
            _pad3: 0.0,
            impulses: core::array::from_fn(|_| WaterImpulseData::default()),
        }
    }
}

// Compile-time verification of Rust <-> WGSL shader uniform buffer layout
const _: () = {
    assert!(core::mem::offset_of!(WaterInteractorData, grid_x) == 0);
    assert!(core::mem::offset_of!(WaterInteractorData, push_force) == 8);
    assert!(core::mem::offset_of!(WaterImpulseData, force) == 8);
};

// --- GPU Handles ---
#[derive(Resource, Clone, ExtractResource)]
pub struct WaterGpuHandles {
    pub height_current: Handle<ShaderBuffer>,
    pub height_next: Handle<ShaderBuffer>,
    pub flow_x_current: Handle<ShaderBuffer>,
    pub flow_x_next: Handle<ShaderBuffer>,
    pub flow_y_current: Handle<ShaderBuffer>,
    pub flow_y_next: Handle<ShaderBuffer>,
    pub wall_mask: Handle<ShaderBuffer>,
}

pub fn setup_water_gpu_buffers(
    mut commands: Commands,
    mut shader_buffers: ResMut<Assets<ShaderBuffer>>,
) {
    let grid_len = WATER_GRID_SIZE as usize;
    let grid_size = grid_len * grid_len;

    info!(
        "GPU water pipeline: setup complete with {}x{} grid",
        WATER_GRID_SIZE, WATER_GRID_SIZE
    );

    let height_data = vec![1.0f32; grid_size];
    let flow_data = vec![0.0f32; grid_size];
    let wall_data = vec![0u32; grid_size]; // 0 = water, 1 = wall

    let create_buffer = |data: Vec<u32>,
                         shader_buffers: &mut ResMut<Assets<ShaderBuffer>>|
     -> Handle<ShaderBuffer> {
        let mut buffer = ShaderBuffer::from(data);
        buffer.buffer_description.usage |= BufferUsages::COPY_SRC;
        shader_buffers.add(buffer)
    };

    let create_buffer_f32 = |data: Vec<f32>,
                             shader_buffers: &mut ResMut<Assets<ShaderBuffer>>|
     -> Handle<ShaderBuffer> {
        let mut buffer = ShaderBuffer::from(data);
        buffer.buffer_description.usage |= BufferUsages::COPY_SRC;
        shader_buffers.add(buffer)
    };

    let height_current = create_buffer_f32(height_data.clone(), &mut shader_buffers);

    commands
        .spawn(Readback::buffer(height_current.clone()))
        .observe(
            |event: On<ReadbackComplete>, mut query: Query<&mut crate::water::WaterSimData>| {
                let data: &[f32] = bytemuck::cast_slice(&event.data);
                for mut water_data in query.iter_mut() {
                    if data.len() == water_data.height.len() {
                        water_data.height.copy_from_slice(data);
                        water_data.dirty = true;
                    }
                }
            },
        );

    commands.insert_resource(WaterGpuHandles {
        height_current: height_current.clone(),
        height_next: create_buffer_f32(height_data, &mut shader_buffers),
        flow_x_current: create_buffer_f32(flow_data.clone(), &mut shader_buffers),
        flow_x_next: create_buffer_f32(flow_data.clone(), &mut shader_buffers),
        flow_y_current: create_buffer_f32(flow_data.clone(), &mut shader_buffers),
        flow_y_next: create_buffer_f32(flow_data, &mut shader_buffers),
        wall_mask: create_buffer(wall_data, &mut shader_buffers),
    });
}

// --- Compute Pipelines ---
#[derive(Resource)]
pub struct WaterComputePipelines {
    pub layout: BindGroupLayout,
    pub flow_pass_id: CachedComputePipelineId,
    pub outflow_pass_id: CachedComputePipelineId,
    pub height_pass_id: CachedComputePipelineId,
}

impl FromWorld for WaterComputePipelines {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let layout_descriptor = BindGroupLayoutDescriptor {
            label: Cow::from("water_compute_layout"),
            entries: vec![
                uniform_buffer::<WaterSimParams>(false).build(0, ShaderStages::COMPUTE),
                storage_buffer::<f32>(false).build(1, ShaderStages::COMPUTE),
                storage_buffer::<f32>(false).build(2, ShaderStages::COMPUTE),
                storage_buffer::<f32>(false).build(3, ShaderStages::COMPUTE),
                storage_buffer::<f32>(false).build(4, ShaderStages::COMPUTE),
                storage_buffer::<f32>(false).build(5, ShaderStages::COMPUTE),
                storage_buffer::<f32>(false).build(6, ShaderStages::COMPUTE),
                storage_buffer_read_only::<u32>(false).build(7, ShaderStages::COMPUTE),
            ],
        };

        let layout = render_device
            .create_bind_group_layout(Some("water_compute_layout"), &layout_descriptor.entries);

        let asset_server = world.resource::<AssetServer>();
        let shader = asset_server.load(SHADER_ASSET_PATH);
        let pipeline_cache = world.resource::<PipelineCache>();

        let flow_pass_id = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some(Cow::from("water_flow_pass")),
            layout: vec![layout_descriptor.clone()],
            shader: shader.clone(),
            shader_defs: vec![],
            entry_point: Some(Cow::from("water_flow_pass")),
            zero_initialize_workgroup_memory: false,
            ..default()
        });

        let outflow_pass_id = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some(Cow::from("water_outflow_pass")),
            layout: vec![layout_descriptor.clone()],
            shader: shader.clone(),
            shader_defs: vec![],
            entry_point: Some(Cow::from("water_outflow_pass")),
            zero_initialize_workgroup_memory: false,
            ..default()
        });

        let height_pass_id = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some(Cow::from("water_height_pass")),
            layout: vec![layout_descriptor.clone()],
            shader: shader.clone(),
            shader_defs: vec![],
            entry_point: Some(Cow::from("water_height_pass")),
            zero_initialize_workgroup_memory: false,
            ..default()
        });

        Self {
            layout,
            flow_pass_id,
            outflow_pass_id,
            height_pass_id,
        }
    }
}

#[derive(Resource, Default, PartialEq)]
pub enum WaterComputeState {
    #[default]
    Loading,
    Ready,
}

#[derive(Resource)]
#[allow(dead_code)]
pub struct WaterGpuBindGroups {
    pub bind_group: BindGroup,
    pub params_buffer: Buffer,
}

pub struct WaterComputePlugin;

impl Plugin for WaterComputePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WaterSimParams>()
            .add_plugins(ExtractResourcePlugin::<WaterSimParams>::default())
            .add_plugins(ExtractResourcePlugin::<WaterGpuHandles>::default())
            .add_systems(Startup, setup_water_gpu_buffers)
            .add_systems(Update, swap_water_buffers);

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<WaterComputeState>()
            .add_systems(
                Render,
                prepare_water_pipeline.in_set(RenderSystems::PrepareResources),
            )
            .add_systems(
                Render,
                queue_water_bind_group.in_set(RenderSystems::PrepareBindGroups),
            )
            .add_systems(Render, dispatch_water_compute.in_set(RenderSystems::Render));
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app.init_resource::<WaterComputePipelines>();
    }
}

fn swap_water_buffers(
    handles: Option<ResMut<WaterGpuHandles>>,
    query: Query<Entity, With<Readback>>,
    mut warned: Local<bool>,
) {
    if query.iter().count() == 0 {
        if !*warned {
            info!("WARNING: No entity with Readback component found in Main World!");
            *warned = true;
        }
    } else {
        *warned = false;
    }

    if let Some(mut handles) = handles {
        let h = &mut *handles;
        std::mem::swap(&mut h.height_current, &mut h.height_next);
        std::mem::swap(&mut h.flow_x_current, &mut h.flow_x_next);
        std::mem::swap(&mut h.flow_y_current, &mut h.flow_y_next);
    }
}

fn prepare_water_pipeline(
    mut state: ResMut<WaterComputeState>,
    pipeline_cache: Res<PipelineCache>,
    pipelines: Res<WaterComputePipelines>,
) {
    if *state == WaterComputeState::Ready {
        return;
    }

    let f = pipeline_cache.get_compute_pipeline_state(pipelines.flow_pass_id);
    let o = pipeline_cache.get_compute_pipeline_state(pipelines.outflow_pass_id);
    let h = pipeline_cache.get_compute_pipeline_state(pipelines.height_pass_id);

    if let (CachedPipelineState::Ok(_), CachedPipelineState::Ok(_), CachedPipelineState::Ok(_)) =
        (&f, &o, &h)
    {
        info!("Water compute pipelines are READY!");
        *state = WaterComputeState::Ready;
    }
}

#[allow(clippy::too_many_arguments)]
fn queue_water_bind_group(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    pipelines: Res<WaterComputePipelines>,
    gpu_handles: Option<Res<WaterGpuHandles>>,
    params: Option<Res<WaterSimParams>>,
    shader_buffers: Res<RenderAssets<GpuShaderBuffer>>,
    mut local_buffer: Local<Option<Buffer>>,
) {
    let (Some(handles), Some(params)) = (gpu_handles, params) else {
        return;
    };

    let get_buffer = |handle| -> Option<&Buffer> { shader_buffers.get(handle).map(|b| &b.buffer) };

    let Some(height_current) = get_buffer(&handles.height_current) else {
        return;
    };
    let Some(height_next) = get_buffer(&handles.height_next) else {
        return;
    };
    let Some(flow_x_current) = get_buffer(&handles.flow_x_current) else {
        return;
    };
    let Some(flow_x_next) = get_buffer(&handles.flow_x_next) else {
        return;
    };
    let Some(flow_y_current) = get_buffer(&handles.flow_y_current) else {
        return;
    };
    let Some(flow_y_next) = get_buffer(&handles.flow_y_next) else {
        return;
    };
    let Some(wall_mask) = get_buffer(&handles.wall_mask) else {
        return;
    };

    let params_buffer = local_buffer.get_or_insert_with(|| {
        render_device.create_buffer(&BufferDescriptor {
            label: Some("water_params_buffer"),
            size: std::mem::size_of::<WaterSimParams>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    });

    let mut encase_buffer = encase::UniformBuffer::new(Vec::new());
    encase_buffer.write(&*params).unwrap();
    render_queue.write_buffer(params_buffer, 0, &encase_buffer.into_inner());

    let bind_group = render_device.create_bind_group(
        Some("water_compute_bind_group"),
        &pipelines.layout,
        &[
            BindGroupEntry {
                binding: 0,
                resource: params_buffer.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 1,
                resource: height_current.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 2,
                resource: height_next.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 3,
                resource: flow_x_current.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 4,
                resource: flow_y_current.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 5,
                resource: flow_x_next.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 6,
                resource: flow_y_next.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 7,
                resource: wall_mask.as_entire_binding(),
            },
        ],
    );

    commands.insert_resource(WaterGpuBindGroups {
        bind_group,
        params_buffer: params_buffer.clone(),
    });
}

fn dispatch_water_compute(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    state: Res<WaterComputeState>,
    pipelines: Res<WaterComputePipelines>,
    bind_groups: Option<Res<WaterGpuBindGroups>>,
    pipeline_cache: Res<PipelineCache>,
) {
    if *state != WaterComputeState::Ready {
        return;
    }
    let Some(bind_groups) = bind_groups else {
        return;
    };

    let flow_pipeline = pipeline_cache
        .get_compute_pipeline(pipelines.flow_pass_id)
        .unwrap();
    let outflow_pipeline = pipeline_cache
        .get_compute_pipeline(pipelines.outflow_pass_id)
        .unwrap();
    let height_pipeline = pipeline_cache
        .get_compute_pipeline(pipelines.height_pass_id)
        .unwrap();

    let mut encoder = render_device.create_command_encoder(&CommandEncoderDescriptor::default());
    {
        let mut pass = encoder.begin_compute_pass(&ComputePassDescriptor::default());
        let workgroups = WATER_GRID_SIZE / WORKGROUP_SIZE;

        pass.set_pipeline(flow_pipeline);
        pass.set_bind_group(0, &bind_groups.bind_group, &[]);
        pass.dispatch_workgroups(workgroups, workgroups, 1);

        pass.set_pipeline(outflow_pipeline);
        pass.set_bind_group(0, &bind_groups.bind_group, &[]);
        pass.dispatch_workgroups(workgroups, workgroups, 1);

        pass.set_pipeline(height_pipeline);
        pass.set_bind_group(0, &bind_groups.bind_group, &[]);
        pass.dispatch_workgroups(workgroups, workgroups, 1);
    }
    render_queue.submit(vec![encoder.finish()]);
}
