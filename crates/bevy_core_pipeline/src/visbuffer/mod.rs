use core::ops::Range;

use bevy_ecs::entity::Entity;
use bevy_render::{
    render_phase::{
        BinnedPhaseItem, CachedRenderPipelinePhaseItem, DrawFunctionId, PhaseItem,
        PhaseItemExtraIndex,
    },
    render_resource::{CachedRenderPipelineId, ColorTargetState, ColorWrites},
    sync_world::MainEntity,
};

use crate::prepass::{
    OpaqueNoLightmap3dBatchSetKey, OpaqueNoLightmap3dBinKey, VISBUFFER_PREPASS_FORMAT,
};

pub mod node;

#[derive(PartialEq, Eq, Hash)]
pub struct Opaque3dVisbuffer {
    /// Determines which objects can be placed into a *batch set*.
    ///
    /// Objects in a single batch set can potentially be multi-drawn together,
    /// if it's enabled and the current platform supports it.
    pub batch_set_key: OpaqueNoLightmap3dBatchSetKey,
    /// Information that separates items into bins.
    pub bin_key: OpaqueNoLightmap3dBinKey,
    pub representative_entity: (Entity, MainEntity),
    pub batch_range: Range<u32>,
    pub extra_index: PhaseItemExtraIndex,
}

impl PhaseItem for Opaque3dVisbuffer {
    #[inline]
    fn entity(&self) -> Entity {
        self.representative_entity.0
    }

    fn main_entity(&self) -> MainEntity {
        self.representative_entity.1
    }

    #[inline]
    fn draw_function(&self) -> DrawFunctionId {
        self.batch_set_key.draw_function
    }

    #[inline]
    fn batch_range(&self) -> &Range<u32> {
        &self.batch_range
    }

    #[inline]
    fn batch_range_mut(&mut self) -> &mut Range<u32> {
        &mut self.batch_range
    }

    #[inline]
    fn extra_index(&self) -> PhaseItemExtraIndex {
        self.extra_index.clone()
    }

    #[inline]
    fn batch_range_and_extra_index_mut(&mut self) -> (&mut Range<u32>, &mut PhaseItemExtraIndex) {
        (&mut self.batch_range, &mut self.extra_index)
    }
}

impl BinnedPhaseItem for Opaque3dVisbuffer {
    type BatchSetKey = OpaqueNoLightmap3dBatchSetKey;
    type BinKey = OpaqueNoLightmap3dBinKey;

    #[inline]
    fn new(
        batch_set_key: Self::BatchSetKey,
        bin_key: Self::BinKey,
        representative_entity: (Entity, MainEntity),
        batch_range: Range<u32>,
        extra_index: PhaseItemExtraIndex,
    ) -> Self {
        Self {
            batch_set_key,
            bin_key,
            representative_entity,
            batch_range,
            extra_index,
        }
    }
}

impl CachedRenderPipelinePhaseItem for Opaque3dVisbuffer {
    #[inline]
    fn cached_pipeline(&self) -> CachedRenderPipelineId {
        self.batch_set_key.pipeline
    }
}

#[derive(PartialEq, Eq, Hash)]
pub struct AlphaMask3dVisbuffer {
    /// Determines which objects can be placed into a *batch set*.
    ///
    /// Objects in a single batch set can potentially be multi-drawn together,
    /// if it's enabled and the current platform supports it.
    pub batch_set_key: OpaqueNoLightmap3dBatchSetKey,
    /// Information that separates items into bins.
    pub bin_key: OpaqueNoLightmap3dBinKey,
    pub representative_entity: (Entity, MainEntity),
    pub batch_range: Range<u32>,
    pub extra_index: PhaseItemExtraIndex,
}

impl PhaseItem for AlphaMask3dVisbuffer {
    #[inline]
    fn entity(&self) -> Entity {
        self.representative_entity.0
    }

    fn main_entity(&self) -> MainEntity {
        self.representative_entity.1
    }

    #[inline]
    fn draw_function(&self) -> DrawFunctionId {
        self.batch_set_key.draw_function
    }

    #[inline]
    fn batch_range(&self) -> &Range<u32> {
        &self.batch_range
    }

    #[inline]
    fn batch_range_mut(&mut self) -> &mut Range<u32> {
        &mut self.batch_range
    }

    #[inline]
    fn extra_index(&self) -> PhaseItemExtraIndex {
        self.extra_index.clone()
    }

    #[inline]
    fn batch_range_and_extra_index_mut(&mut self) -> (&mut Range<u32>, &mut PhaseItemExtraIndex) {
        (&mut self.batch_range, &mut self.extra_index)
    }
}

impl BinnedPhaseItem for AlphaMask3dVisbuffer {
    type BatchSetKey = OpaqueNoLightmap3dBatchSetKey;
    type BinKey = OpaqueNoLightmap3dBinKey;

    #[inline]
    fn new(
        batch_set_key: Self::BatchSetKey,
        bin_key: Self::BinKey,
        representative_entity: (Entity, MainEntity),
        batch_range: Range<u32>,
        extra_index: PhaseItemExtraIndex,
    ) -> Self {
        Self {
            batch_set_key,
            bin_key,
            representative_entity,
            batch_range,
            extra_index,
        }
    }
}

impl CachedRenderPipelinePhaseItem for AlphaMask3dVisbuffer {
    #[inline]
    fn cached_pipeline(&self) -> CachedRenderPipelineId {
        self.batch_set_key.pipeline
    }
}

pub fn visbuffer_target_descriptors() -> Vec<Option<ColorTargetState>> {
    vec![Some(ColorTargetState {
        format: VISBUFFER_PREPASS_FORMAT,
        blend: None,
        write_mask: ColorWrites::RED,
    })]
}
