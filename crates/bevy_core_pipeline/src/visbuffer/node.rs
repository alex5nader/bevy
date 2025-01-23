use bevy_ecs::{query::QueryItem, system::lifetimeless::Read, world::World};
use bevy_render::{
    camera::ExtractedCamera,
    render_graph::{NodeRunError, RenderGraphContext, ViewNode},
    render_phase::{TrackedRenderPass, ViewBinnedRenderPhases},
    render_resource::{CommandEncoderDescriptor, RenderPassDescriptor, StoreOp},
    renderer::RenderContext,
    view::{ExtractedView, ViewDepthTexture},
};
use tracing::error;

use crate::prepass::ViewPrepassTextures;

use super::{AlphaMask3dVisbuffer, Opaque3dVisbuffer};

#[derive(Default)]
pub struct VisbufferPrepassNode;

impl ViewNode for VisbufferPrepassNode {
    type ViewQuery = (
        Read<ExtractedCamera>,
        Read<ExtractedView>,
        Read<ViewDepthTexture>,
        Read<ViewPrepassTextures>,
    );

    fn run<'w>(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        (camera, extracted_view, view_depth_texture, view_prepass_textures): QueryItem<
            'w,
            Self::ViewQuery,
        >,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let (Some(opaque_visbuffer_phases), Some(alpha_mask_visbuffer_phases)) = (
            world.get_resource::<ViewBinnedRenderPhases<Opaque3dVisbuffer>>(),
            world.get_resource::<ViewBinnedRenderPhases<AlphaMask3dVisbuffer>>(),
        ) else {
            return Ok(());
        };

        let (Some(opaque_visbuffer_phase), Some(alpha_mask_visbuffer_phase)) = (
            opaque_visbuffer_phases.get(&extracted_view.retained_view_entity),
            alpha_mask_visbuffer_phases.get(&extracted_view.retained_view_entity),
        ) else {
            return Ok(());
        };

        let Some(visbuffer_texture) = &view_prepass_textures.visbuffer else {
            // todo: error?
            return Ok(());
        };

        // todo: get rid of pointless vec? could use std::slice::from_ref
        let color_attachments = vec![Some(visbuffer_texture.get_attachment())];

        let depth_stencil_attachment = Some(view_depth_texture.get_attachment(StoreOp::Store));

        let view_entity = graph.view_entity();
        render_context.add_command_buffer_generation_task(move |render_device| {
            #[cfg(feature = "trace")]
            let _visbuffer_span = info_span!("visbuffer_prepass").entered();

            // Command encoder setup
            let mut command_encoder =
                render_device.create_command_encoder(&CommandEncoderDescriptor {
                    label: Some("visbuffer_prepass_command_encoder"),
                });

            // Render pass setup
            let render_pass = command_encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("visbuffer_prepass"),
                color_attachments: &color_attachments,
                depth_stencil_attachment,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            let mut render_pass = TrackedRenderPass::new(&render_device, render_pass);
            if let Some(viewport) = camera.viewport.as_ref() {
                render_pass.set_camera_viewport(viewport);
            }

            // Opaque draws
            if !opaque_visbuffer_phase.multidrawable_mesh_keys.is_empty()
                || !opaque_visbuffer_phase.batchable_mesh_keys.is_empty()
                || !opaque_visbuffer_phase.unbatchable_mesh_keys.is_empty()
            {
                #[cfg(feature = "trace")]
                let _opaque_prepass_span = info_span!("opaque_visbuffer_prepass").entered();
                if let Err(err) =
                    opaque_visbuffer_phase.render(&mut render_pass, world, view_entity)
                {
                    error!("Error encountered while rendering the opaque visbuffer phase {err:?}");
                }
            }

            // Alpha masked draws
            if !alpha_mask_visbuffer_phase.is_empty() {
                #[cfg(feature = "trace")]
                let _alpha_mask_visbuffer_span =
                    info_span!("alpha_mask_visbuffer_prepass").entered();
                if let Err(err) =
                    alpha_mask_visbuffer_phase.render(&mut render_pass, world, view_entity)
                {
                    error!(
                        "Error encountered while rendering the alpha mask visbuffer phase {err:?}"
                    );
                }
            }

            drop(render_pass);

            command_encoder.finish()
        });

        Ok(())
    }
}
