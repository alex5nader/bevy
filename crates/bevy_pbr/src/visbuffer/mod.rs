#![expect(unused_imports)]

use crate::material_bind_groups::MaterialBindGroupAllocator;
use bevy_render::{
    batching::gpu_preprocessing::GpuPreprocessingSupport,
    mesh::{allocator::MeshAllocator, Mesh3d, MeshVertexBufferLayoutRef, RenderMesh},
    render_resource::binding_types::uniform_buffer,
    renderer::RenderAdapter,
    sync_world::RenderEntity,
    view::{RenderVisibilityRanges, VISIBILITY_RANGES_STORAGE_BUFFER_COUNT},
};

use bevy_asset::{load_internal_asset, AssetServer};
use bevy_core_pipeline::{
    core_3d::CORE_3D_DEPTH_FORMAT,
    deferred::*,
    prelude::Camera3d,
    prepass::*,
    visbuffer::{visbuffer_target_descriptors, AlphaMask3dVisbuffer, Opaque3dVisbuffer},
};
use bevy_ecs::{
    prelude::*,
    query::{ROQueryItem, WorldQuery},
    system::{
        lifetimeless::{Read, SRes},
        SystemParamItem,
    },
};
use bevy_math::{Affine3A, Vec4};
use bevy_render::{
    globals::{GlobalsBuffer, GlobalsUniform},
    prelude::{Camera, Mesh},
    render_asset::RenderAssets,
    render_phase::*,
    render_resource::*,
    renderer::{RenderDevice, RenderQueue},
    view::{ExtractedView, Msaa, ViewUniform, ViewUniformOffset, ViewUniforms},
    Extract,
};
use bevy_transform::prelude::GlobalTransform;
use tracing::error;

#[cfg(feature = "meshlet")]
use crate::meshlet::{
    prepare_material_meshlet_meshes_prepass, queue_material_meshlet_meshes, InstanceManager,
    MeshletMesh3d,
};
use crate::*;

use bevy_render::view::RenderVisibleEntities;
use core::{hash::Hash, marker::PhantomData};

pub const VISBUFFER_PREPASS_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(21142424535904663167378759386861688790);
pub const VISBUFFER_UTILS_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(259939080518385425152214904457457353386);
pub const VISBUFFER_IO_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(261947013329844448084660199004371559244);

pub struct VisbufferPrepassPipelinePlugin<M: Material>(PhantomData<M>);

impl<M: Material> Default for VisbufferPrepassPipelinePlugin<M> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<M: Material> Plugin for VisbufferPrepassPipelinePlugin<M>
where
    M::Data: PartialEq + Eq + Hash + Clone,
{
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            VISBUFFER_PREPASS_SHADER_HANDLE,
            "visbuffer_prepass.wgsl",
            Shader::from_wgsl
        );

        load_internal_asset!(
            app,
            VISBUFFER_UTILS_SHADER_HANDLE,
            "visbuffer_utils.wgsl",
            Shader::from_wgsl
        );

        load_internal_asset!(
            app,
            VISBUFFER_IO_SHADER_HANDLE,
            "visbuffer_io.wgsl",
            Shader::from_wgsl
        );

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .add_systems(
                Render,
                prepare_visbuffer_prepass_view_bind_group::<M>.in_set(RenderSet::PrepareBindGroups),
            )
            .init_resource::<VisbufferPrepassViewBindGroup>()
            .init_resource::<SpecializedMeshPipelines<VisbufferPrepassPipeline<M>>>()
            .allow_ambiguous_resource::<SpecializedMeshPipelines<VisbufferPrepassPipeline<M>>>();
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.init_resource::<VisbufferPrepassPipeline<M>>();
    }
}

#[derive(Resource)]
pub struct VisbufferPrepassPipeline<M: Material> {
    pub view_layout: BindGroupLayout,
    pub mesh_layouts: MeshLayouts,
    pub material_layout: BindGroupLayout,
    pub vertex_shader: Option<Handle<Shader>>,
    pub fragment_shader: Option<Handle<Shader>>,
    pub material_pipeline: MaterialPipeline<M>,

    /// Whether skins will use uniform buffers on account of storage buffers
    /// being unavailable on this platform.
    pub skins_use_uniform_buffers: bool,

    pub depth_clip_control_supported: bool,

    /// Whether binding arrays (a.k.a. bindless textures) are usable on the
    /// current render device.
    pub binding_arrays_are_usable: bool,

    _marker: PhantomData<M>,
}

impl<M: Material> FromWorld for VisbufferPrepassPipeline<M> {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let render_adapter = world.resource::<RenderAdapter>();
        let asset_server = world.resource::<AssetServer>();

        let visibility_ranges_buffer_binding_type = render_device
            .get_supported_read_only_binding_type(VISIBILITY_RANGES_STORAGE_BUFFER_COUNT);

        let view_layout = render_device.create_bind_group_layout(
            "visbuffer_prepass_view_layout",
            &BindGroupLayoutEntries::with_indices(
                ShaderStages::VERTEX_FRAGMENT,
                (
                    // View
                    (0, uniform_buffer::<ViewUniform>(true)),
                    // Globals
                    (1, uniform_buffer::<GlobalsUniform>(false)),
                    // VisibilityRanges
                    (
                        14,
                        buffer_layout(
                            visibility_ranges_buffer_binding_type,
                            false,
                            Some(Vec4::min_size()),
                        )
                        .visibility(ShaderStages::VERTEX),
                    ),
                ),
            ),
        );

        let mesh_pipeline = world.resource::<MeshPipeline>();

        let depth_clip_control_supported = render_device
            .features()
            .contains(WgpuFeatures::DEPTH_CLIP_CONTROL);

        VisbufferPrepassPipeline {
            view_layout,
            mesh_layouts: mesh_pipeline.mesh_layouts.clone(),
            // todo: is it correct to just use forward vertex shader?
            vertex_shader: match M::prepass_vertex_shader() {
                ShaderRef::Default => None,
                ShaderRef::Handle(handle) => Some(handle),
                ShaderRef::Path(path) => Some(asset_server.load(path)),
            },
            fragment_shader: None,
            material_layout: M::bind_group_layout(render_device),
            material_pipeline: world.resource::<MaterialPipeline<M>>().clone(),
            skins_use_uniform_buffers: skin::skins_use_uniform_buffers(render_device),
            depth_clip_control_supported,
            binding_arrays_are_usable: binding_arrays_are_usable(render_device, render_adapter),
            _marker: PhantomData,
        }
    }
}

impl<M: Material> SpecializedMeshPipeline for VisbufferPrepassPipeline<M>
where
    M::Data: PartialEq + Eq + Hash + Clone,
{
    type Key = MaterialPipelineKey<M>;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayoutRef,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mut bind_group_layouts = vec![self.view_layout.clone()];
        let mut shader_defs = Vec::new();
        let mut vertex_attributes = Vec::new();

        shader_defs.push("VISBUFFER_PREPASS".into());

        // Let the shader code know that it's running in a prepass pipeline.
        // (PBR code will use this to detect that it's running in deferred mode,
        // since that's the only time it gets called from a prepass pipeline.)
        shader_defs.push("PREPASS_PIPELINE".into());

        // NOTE: Eventually, it would be nice to only add this when the shaders are overloaded by the Material.
        // The main limitation right now is that bind group order is hardcoded in shaders.
        bind_group_layouts.push(self.material_layout.clone());

        #[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
        shader_defs.push("WEBGL2".into());

        shader_defs.push("VERTEX_OUTPUT_INSTANCE_INDEX".into());

        if key.mesh_key.contains(MeshPipelineKey::MAY_DISCARD) {
            shader_defs.push("MAY_DISCARD".into());
        }

        if layout.0.contains(Mesh::ATTRIBUTE_POSITION) {
            shader_defs.push("VERTEX_POSITIONS".into());
            vertex_attributes.push(Mesh::ATTRIBUTE_POSITION.at_shader_location(0));
        }

        // For directional light shadow map views, use unclipped depth via either the native GPU feature,
        // or emulated by setting depth in the fragment shader for GPUs that don't support it natively.
        let emulate_unclipped_depth = key
            .mesh_key
            .contains(MeshPipelineKey::UNCLIPPED_DEPTH_ORTHO)
            && !self.depth_clip_control_supported;
        if emulate_unclipped_depth {
            shader_defs.push("UNCLIPPED_DEPTH_ORTHO_EMULATION".into());
        }
        let unclipped_depth = key
            .mesh_key
            .contains(MeshPipelineKey::UNCLIPPED_DEPTH_ORTHO)
            && self.depth_clip_control_supported;

        if layout.0.contains(Mesh::ATTRIBUTE_UV_0) {
            shader_defs.push("VERTEX_UVS".into());
            shader_defs.push("VERTEX_UVS_A".into());
            vertex_attributes.push(Mesh::ATTRIBUTE_UV_0.at_shader_location(1));
        }

        if layout.0.contains(Mesh::ATTRIBUTE_UV_1) {
            shader_defs.push("VERTEX_UVS".into());
            shader_defs.push("VERTEX_UVS_B".into());
            vertex_attributes.push(Mesh::ATTRIBUTE_UV_1.at_shader_location(2));
        }

        // If bindless mode is on, add a `BINDLESS` define.
        if self.material_pipeline.bindless {
            shader_defs.push("BINDLESS".into());
        }

        let bind_group = setup_morph_and_skinning_defs(
            &self.mesh_layouts,
            layout,
            5,
            &key.mesh_key,
            &mut shader_defs,
            &mut vertex_attributes,
            self.skins_use_uniform_buffers,
        );
        bind_group_layouts.insert(1, bind_group);

        let vertex_buffer_layout = layout.0.get_layout(&vertex_attributes)?;

        // shouldn't this use the material's vertex shader?
        let vert_shader_handle = VISBUFFER_PREPASS_SHADER_HANDLE;

        let mut descriptor = RenderPipelineDescriptor {
            vertex: VertexState {
                shader: vert_shader_handle,
                entry_point: "vertex".into(),
                shader_defs: shader_defs.clone(),
                buffers: vec![vertex_buffer_layout],
            },
            fragment: Some(FragmentState {
                shader: VISBUFFER_PREPASS_SHADER_HANDLE,
                entry_point: "fragment".into(),
                shader_defs,
                targets: visbuffer_target_descriptors(),
            }),
            layout: bind_group_layouts,
            primitive: PrimitiveState {
                topology: key.mesh_key.primitive_topology(),
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(DepthStencilState {
                format: CORE_3D_DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: CompareFunction::GreaterEqual,
                stencil: StencilState {
                    front: StencilFaceState::IGNORE,
                    back: StencilFaceState::IGNORE,
                    read_mask: 0,
                    write_mask: 0,
                },
                bias: DepthBiasState {
                    constant: 0,
                    slope_scale: 0.0,
                    clamp: 0.0,
                },
            }),
            multisample: MultisampleState {
                count: key.mesh_key.msaa_samples(),
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            push_constant_ranges: vec![],
            label: Some("visbuffer_pipeline".into()),
            zero_initialize_workgroup_memory: false,
        };

        // This is a bit risky because it's possible to change something that would
        // break the prepass but be fine in the main pass.
        // Since this api is pretty low-level it doesn't matter that much, but it is a potential issue.
        M::specialize(&self.material_pipeline, &mut descriptor, layout, key)?;

        Ok(descriptor)
    }
}

// todo: share prepass no_motion_vectors?
#[derive(Default, Resource)]
pub struct VisbufferPrepassViewBindGroup(Option<BindGroup>);

pub fn prepare_visbuffer_prepass_view_bind_group<M: Material>(
    render_device: Res<RenderDevice>,
    visbuffer_prepass_pipeline: Res<VisbufferPrepassPipeline<M>>,
    view_uniforms: Res<ViewUniforms>,
    globals_buffer: Res<GlobalsBuffer>,
    visibility_ranges: Res<RenderVisibilityRanges>,
    mut prepass_view_bind_group: ResMut<VisbufferPrepassViewBindGroup>,
) {
    if let (Some(view_binding), Some(globals_binding), Some(visibility_ranges_buffer)) = (
        view_uniforms.uniforms.binding(),
        globals_buffer.buffer.binding(),
        visibility_ranges.buffer().buffer(),
    ) {
        prepass_view_bind_group.0 = Some(render_device.create_bind_group(
            "visbuffer_prepass_view_bind_group",
            &visbuffer_prepass_pipeline.view_layout,
            &BindGroupEntries::with_indices((
                (0, view_binding.clone()),
                (1, globals_binding.clone()),
                (14, visibility_ranges_buffer.as_entire_binding()),
            )),
        ));
    }
}

pub struct SetVisbufferPrepassViewBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetVisbufferPrepassViewBindGroup<I> {
    type Param = SRes<VisbufferPrepassViewBindGroup>;
    type ViewQuery = Read<ViewUniformOffset>;
    type ItemQuery = ();

    #[inline]
    fn render<'w>(
        _item: &P,
        view_uniform_offset: ROQueryItem<'w, Self::ViewQuery>,
        _entity: Option<()>,
        prepass_view_bind_group: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let prepass_view_bind_group = prepass_view_bind_group.into_inner();

        pass.set_bind_group(
            I,
            prepass_view_bind_group.0.as_ref().unwrap(),
            &[view_uniform_offset.offset],
        );

        RenderCommandResult::Success
    }
}

pub type DrawVisbuffer<M> = (
    SetItemPipeline,
    SetVisbufferPrepassViewBindGroup<0>,
    SetMeshBindGroup<1>,
    SetMaterialBindGroup<M, 2>,
    DrawMesh,
);
