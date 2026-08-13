use serde::{Deserialize, Serialize};

#[cfg(any(feature = "server", feature = "python"))]
use crate::serialization::{FastVec, ServerHeader, serialize, serialize_heap};

#[derive(Clone, Copy, Serialize, Deserialize)]
pub(crate) enum ImageType {
    Color,
    ColorAlpha,
    Gray,
    GrayAlpha,
}

impl ImageType {
    #[inline]
    pub(crate) fn bytes_per_pixel(&self) -> usize {
        match self {
            Self::Color => 3,
            Self::ColorAlpha => 4,
            Self::Gray => 1,
            Self::GrayAlpha => 2,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub(crate) enum ImageSetHeader {
    All([u32; 2], bool),  // [x, y], update
    Start([u32; 2], u32), // [x, y], pixels
    Batch(u32),           // pixels
    End(u32, bool),       // pixels, update
}

#[derive(Serialize, Deserialize)]
pub(crate) enum ImageMultiHeader {
    Remove(u32, bool),             // index, update
    Modify(u32, ImageHeader, u32), // index, image header, payload size
    Reset(bool),                   // update
}

#[cfg(any(feature = "server", feature = "python"))]
pub(crate) fn serialize_image_multi_header(
    id: u64,
    index: u32,
    header: ImageHeader,
    size: u32,
) -> Result<FastVec<32>, ()> {
    let header = ServerHeader::ImageMulti(id, ImageMultiHeader::Modify(index, header, size));
    serialize_heap(&header)
}

#[cfg(any(feature = "server", feature = "python"))]
impl ImageSetHeader {
    pub(crate) fn serialize(
        self,
        id: u64,
        image_type: ImageType,
        size: u32,
    ) -> Result<FastVec<32>, ()> {
        let header = &ServerHeader::Image(id, ImageHeader::Set(self, image_type), size);
        serialize_heap(header)
    }

    pub(crate) fn serialize_multi(
        self,
        id: u64,
        index: u32,
        image_type: ImageType,
        size: u32,
    ) -> Result<FastVec<32>, ()> {
        serialize_image_multi_header(id, index, ImageHeader::Set(self, image_type), size)
    }
}

#[cfg(any(feature = "server", feature = "python"))]
impl ImageMultiHeader {
    pub(crate) fn serialize(self, id: u64) -> Result<FastVec<32>, ()> {
        serialize(&ServerHeader::ImageMulti(id, self))
    }
}

#[derive(Serialize, Deserialize)]
pub(crate) enum ImageHeader {
    Set(ImageSetHeader, ImageType),    // header
    Update([u32; 4], ImageType, bool), // [x, y, w, h], image_type, update
    Fill([u32; 2], [u8; 4], bool),     // [width, height], RGBA, update
}
