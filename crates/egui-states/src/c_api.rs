//! Internal dynamic server support for the companion `egui_states_c` crate.
//!
//! This module is public only so a separate `staticlib`/`cdylib` package can
//! call it. It is not a stable Rust API; the stable boundary is the C header.

use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use bytes::Bytes;
use parking_lot::RwLock;

use crate::ObjectType;
use crate::data_transport::DataType;
use crate::hashing::NoHashMap;
use crate::image_transport::ImageType;
use crate::server_core::data_core::{Data, DataHolder, DataMulti};
use crate::server_core::data_take_core::{DataMultiTake, DataTake};
use crate::server_core::image_core::{Image, ImageData};
use crate::server_core::image_core_common::checked_image_rect;
use crate::server_core::image_multi_core::{ImageMulti, ImageMultiData};
use crate::server_core::map_core::ValueMap;
use crate::server_core::server::Server;
use crate::server_core::signals::{
    CLIENT_MESSAGE_ID, LOGGING_ID, ON_CONNECT_ID, ON_DISCONNECT_ID, SignalsManager,
};
use crate::server_core::value_parsing::{ValueCreator, ValueParser};
use crate::server_core::values_core::{SignalCore, ValueCore, ValueStaticCore, ValueTakeCore};
use crate::server_core::vec_core::ValueList;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ErrorKind {
    InvalidArgument,
    InvalidState,
    TypeMismatch,
    NotFound,
    OutOfRange,
    BufferTooSmall,
    Timeout,
    Io,
    Internal,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Error {
    pub kind: ErrorKind,
    message: String,
}

impl Error {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    fn invalid(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::InvalidArgument, message)
    }

    fn state(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::InvalidState, message)
    }

    fn mismatch(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::TypeMismatch, message)
    }

    fn missing(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::NotFound, message)
    }

    fn core(message: impl Into<String>) -> Self {
        let message = message.into();
        let lower = message.to_ascii_lowercase();
        let kind = if lower.contains("not found") {
            ErrorKind::NotFound
        } else if lower.contains("out of bounds") || lower.contains("exceeds") {
            ErrorKind::OutOfRange
        } else if lower.contains("type") {
            ErrorKind::TypeMismatch
        } else if lower.contains("finalized") || lower.contains("initialized") {
            ErrorKind::InvalidState
        } else {
            ErrorKind::InvalidArgument
        };
        Self::new(kind, message)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug, PartialEq)]
pub enum DynamicValue {
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
    Bool(bool),
    String(String),
    Enum(u32),
    Sequence(Vec<DynamicValue>),
    Map(Vec<(DynamicValue, DynamicValue)>),
    Option(Option<Box<DynamicValue>>),
    Empty,
}

impl DynamicValue {
    pub fn kind_name(&self) -> &'static str {
        match self {
            Self::U8(_) => "u8",
            Self::U16(_) => "u16",
            Self::U32(_) => "u32",
            Self::U64(_) => "u64",
            Self::I8(_) => "i8",
            Self::I16(_) => "i16",
            Self::I32(_) => "i32",
            Self::I64(_) => "i64",
            Self::F32(_) => "f32",
            Self::F64(_) => "f64",
            Self::Bool(_) => "bool",
            Self::String(_) => "string",
            Self::Enum(_) => "enum",
            Self::Sequence(_) => "sequence",
            Self::Map(_) => "map",
            Self::Option(_) => "option",
            Self::Empty => "empty",
        }
    }
}

fn mismatch(expected: &str, value: &DynamicValue) -> Error {
    Error::mismatch(format!(
        "expected {expected} value, received {}",
        value.kind_name()
    ))
}

fn add_value(
    creator: &mut ValueCreator,
    value: &DynamicValue,
    object_type: &ObjectType,
) -> Result<()> {
    match (object_type, value) {
        (ObjectType::U8, DynamicValue::U8(value)) => creator.add(value),
        (ObjectType::U16, DynamicValue::U16(value)) => creator.add(value),
        (ObjectType::U32, DynamicValue::U32(value)) => creator.add(value),
        (ObjectType::U64, DynamicValue::U64(value)) => creator.add(value),
        (ObjectType::I8, DynamicValue::I8(value)) => creator.add(value),
        (ObjectType::I16, DynamicValue::I16(value)) => creator.add(value),
        (ObjectType::I32, DynamicValue::I32(value)) => creator.add(value),
        (ObjectType::I64, DynamicValue::I64(value)) => creator.add(value),
        (ObjectType::F32, DynamicValue::F32(value)) => creator.add(value),
        (ObjectType::F64, DynamicValue::F64(value)) => creator.add(value),
        (ObjectType::Bool, DynamicValue::Bool(value)) => creator.add(value),
        (ObjectType::String, DynamicValue::String(value)) => creator.add(value),
        (ObjectType::Enum(_, variants), DynamicValue::Enum(index)) => {
            if (*index as usize) >= variants.len() {
                return Err(Error::invalid(format!(
                    "enum ordinal {index} is outside 0..{}",
                    variants.len()
                )));
            }
            creator.add(index)
        }
        (ObjectType::Tuple(types), DynamicValue::Sequence(values)) => {
            add_sequence(creator, values, types, "tuple")?;
            return Ok(());
        }
        (ObjectType::Struct(_, fields), DynamicValue::Sequence(values)) => {
            if values.len() != fields.len() {
                return Err(Error::mismatch(format!(
                    "struct has {} fields, received {} values",
                    fields.len(),
                    values.len()
                )));
            }
            for (value, (_, field_type)) in values.iter().zip(fields) {
                add_value(creator, value, field_type)?;
            }
            return Ok(());
        }
        (ObjectType::List(size, item_type), DynamicValue::Sequence(values)) => {
            if values.len() != *size as usize {
                return Err(Error::mismatch(format!(
                    "fixed list has length {size}, received {} values",
                    values.len()
                )));
            }
            for value in values {
                add_value(creator, value, item_type)?;
            }
            return Ok(());
        }
        (ObjectType::Vec(item_type), DynamicValue::Sequence(values)) => {
            let len = u64::try_from(values.len())
                .map_err(|_| Error::invalid("vector length exceeds protocol limits"))?;
            creator.add(&len).map_err(|_| {
                Error::new(ErrorKind::Internal, "failed to serialize vector length")
            })?;
            for value in values {
                add_value(creator, value, item_type)?;
            }
            return Ok(());
        }
        (ObjectType::Map(key_type, value_type), DynamicValue::Map(entries)) => {
            let len = u64::try_from(entries.len())
                .map_err(|_| Error::invalid("map length exceeds protocol limits"))?;
            creator
                .add(&len)
                .map_err(|_| Error::new(ErrorKind::Internal, "failed to serialize map length"))?;
            for (key, value) in entries {
                add_value(creator, key, key_type)?;
                add_value(creator, value, value_type)?;
            }
            return Ok(());
        }
        (ObjectType::Option(inner), DynamicValue::Option(value)) => {
            match value {
                None => creator.add(&0_u8).map_err(|_| {
                    Error::new(ErrorKind::Internal, "failed to serialize option tag")
                })?,
                Some(value) => {
                    creator.add(&1_u8).map_err(|_| {
                        Error::new(ErrorKind::Internal, "failed to serialize option tag")
                    })?;
                    add_value(creator, value, inner)?;
                }
            }
            return Ok(());
        }
        (ObjectType::Empty, DynamicValue::Empty) => return Ok(()),
        (object_type, value) => return Err(mismatch(object_type_name(object_type), value)),
    }
    .map_err(|_| Error::new(ErrorKind::Internal, "failed to serialize value"))
}

fn add_sequence(
    creator: &mut ValueCreator,
    values: &[DynamicValue],
    types: &[ObjectType],
    name: &str,
) -> Result<()> {
    if values.len() != types.len() {
        return Err(Error::mismatch(format!(
            "{name} has {} elements, received {} values",
            types.len(),
            values.len()
        )));
    }
    for (value, object_type) in values.iter().zip(types) {
        add_value(creator, value, object_type)?;
    }
    Ok(())
}

fn object_type_name(object_type: &ObjectType) -> &'static str {
    match object_type {
        ObjectType::U8 => "u8",
        ObjectType::U16 => "u16",
        ObjectType::U32 => "u32",
        ObjectType::U64 => "u64",
        ObjectType::I8 => "i8",
        ObjectType::I16 => "i16",
        ObjectType::I32 => "i32",
        ObjectType::I64 => "i64",
        ObjectType::F32 => "f32",
        ObjectType::F64 => "f64",
        ObjectType::String => "string",
        ObjectType::Bool => "bool",
        ObjectType::Enum(_, _) => "enum",
        ObjectType::Struct(_, _) => "struct sequence",
        ObjectType::Tuple(_) => "tuple sequence",
        ObjectType::List(_, _) => "fixed-list sequence",
        ObjectType::Vec(_) => "vector sequence",
        ObjectType::Map(_, _) => "map",
        ObjectType::Option(_) => "option",
        ObjectType::Empty => "empty",
    }
}

pub fn serialize_value(value: &DynamicValue, object_type: &ObjectType) -> Result<Bytes> {
    let mut creator = ValueCreator::new();
    add_value(&mut creator, value, object_type)?;
    Ok(creator.finalize())
}

fn parser_get<T: for<'a> serde::Deserialize<'a> + Default>(
    parser: &mut ValueParser,
    name: &str,
) -> Result<T> {
    let mut value = T::default();
    parser
        .get(&mut value)
        .map_err(|_| Error::mismatch(format!("failed to parse {name}")))?;
    Ok(value)
}

fn parse_value(parser: &mut ValueParser, object_type: &ObjectType) -> Result<DynamicValue> {
    Ok(match object_type {
        ObjectType::U8 => DynamicValue::U8(parser_get(parser, "u8")?),
        ObjectType::U16 => DynamicValue::U16(parser_get(parser, "u16")?),
        ObjectType::U32 => DynamicValue::U32(parser_get(parser, "u32")?),
        ObjectType::U64 => DynamicValue::U64(parser_get(parser, "u64")?),
        ObjectType::I8 => DynamicValue::I8(parser_get(parser, "i8")?),
        ObjectType::I16 => DynamicValue::I16(parser_get(parser, "i16")?),
        ObjectType::I32 => DynamicValue::I32(parser_get(parser, "i32")?),
        ObjectType::I64 => DynamicValue::I64(parser_get(parser, "i64")?),
        ObjectType::F32 => DynamicValue::F32(parser_get(parser, "f32")?),
        ObjectType::F64 => DynamicValue::F64(parser_get(parser, "f64")?),
        ObjectType::Bool => DynamicValue::Bool(parser_get(parser, "bool")?),
        ObjectType::String => DynamicValue::String(parser_get(parser, "string")?),
        ObjectType::Enum(_, variants) => {
            let index: u32 = parser_get(parser, "enum")?;
            if index as usize >= variants.len() {
                return Err(Error::mismatch(format!(
                    "enum ordinal {index} is outside 0..{}",
                    variants.len()
                )));
            }
            DynamicValue::Enum(index)
        }
        ObjectType::Tuple(types) => DynamicValue::Sequence(parse_sequence(parser, types)?),
        ObjectType::Struct(_, fields) => {
            let mut values = Vec::with_capacity(fields.len());
            for (_, field_type) in fields {
                values.push(parse_value(parser, field_type)?);
            }
            DynamicValue::Sequence(values)
        }
        ObjectType::List(size, item_type) => {
            let mut values = Vec::with_capacity(*size as usize);
            for _ in 0..*size {
                values.push(parse_value(parser, item_type)?);
            }
            DynamicValue::Sequence(values)
        }
        ObjectType::Vec(item_type) => {
            let len: u64 = parser_get(parser, "vector length")?;
            let len = checked_collection_len(len, parser)?;
            let mut values = Vec::with_capacity(len);
            for _ in 0..len {
                values.push(parse_value(parser, item_type)?);
            }
            DynamicValue::Sequence(values)
        }
        ObjectType::Map(key_type, value_type) => {
            let len: u64 = parser_get(parser, "map length")?;
            let len = checked_collection_len(len, parser)?;
            let mut entries = Vec::with_capacity(len);
            for _ in 0..len {
                entries.push((
                    parse_value(parser, key_type)?,
                    parse_value(parser, value_type)?,
                ));
            }
            DynamicValue::Map(entries)
        }
        ObjectType::Option(inner) => match parser_get::<u8>(parser, "option tag")? {
            0 => DynamicValue::Option(None),
            1 => DynamicValue::Option(Some(Box::new(parse_value(parser, inner)?))),
            tag => return Err(Error::mismatch(format!("invalid option tag {tag}"))),
        },
        ObjectType::Empty => DynamicValue::Empty,
    })
}

fn checked_collection_len(len: u64, parser: &ValueParser) -> Result<usize> {
    let len = usize::try_from(len)
        .map_err(|_| Error::mismatch("collection length exceeds platform limits"))?;
    const MAX_DYNAMIC_ITEMS: usize = 1_048_576;
    if len > MAX_DYNAMIC_ITEMS && len > parser.remaining_len() {
        return Err(Error::mismatch(
            "dynamic collection length is unreasonably large",
        ));
    }
    Ok(len)
}

fn parse_sequence(parser: &mut ValueParser, types: &[ObjectType]) -> Result<Vec<DynamicValue>> {
    let mut values = Vec::with_capacity(types.len());
    for object_type in types {
        values.push(parse_value(parser, object_type)?);
    }
    Ok(values)
}

pub fn deserialize_value(data: Bytes, object_type: &ObjectType) -> Result<DynamicValue> {
    let mut parser = ValueParser::new(data);
    let value = parse_value(&mut parser, object_type)?;
    if !parser.is_finished() {
        return Err(Error::mismatch("value contains trailing serialized bytes"));
    }
    Ok(value)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum DynamicDataType {
    U8 = 0,
    U16 = 1,
    U32 = 2,
    U64 = 3,
    I8 = 4,
    I16 = 5,
    I32 = 6,
    I64 = 7,
    F32 = 8,
    F64 = 9,
}

impl DynamicDataType {
    pub fn from_id(id: u8) -> Result<Self> {
        match id {
            0 => Ok(Self::U8),
            1 => Ok(Self::U16),
            2 => Ok(Self::U32),
            3 => Ok(Self::U64),
            4 => Ok(Self::I8),
            5 => Ok(Self::I16),
            6 => Ok(Self::I32),
            7 => Ok(Self::I64),
            8 => Ok(Self::F32),
            9 => Ok(Self::F64),
            _ => Err(Error::invalid(format!("invalid numeric data type {id}"))),
        }
    }

    pub fn item_size(self) -> usize {
        match self {
            Self::U8 | Self::I8 => 1,
            Self::U16 | Self::I16 => 2,
            Self::U32 | Self::I32 | Self::F32 => 4,
            Self::U64 | Self::I64 | Self::F64 => 8,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DynamicImageFormat {
    Gray,
    GrayAlpha,
    Rgb,
    Rgba,
}

impl DynamicImageFormat {
    pub fn channels(self) -> usize {
        match self {
            Self::Gray => 1,
            Self::GrayAlpha => 2,
            Self::Rgb => 3,
            Self::Rgba => 4,
        }
    }

    fn core(self) -> ImageType {
        match self {
            Self::Gray => ImageType::Gray,
            Self::GrayAlpha => ImageType::GrayAlpha,
            Self::Rgb => ImageType::Color,
            Self::Rgba => ImageType::ColorAlpha,
        }
    }
}

pub struct DynamicImageView<'a> {
    pub data: &'a [u8],
    pub height: usize,
    pub width: usize,
    pub row_stride: usize,
    pub format: DynamicImageFormat,
}

impl DynamicImageView<'_> {
    fn validate(&self) -> Result<(usize, bool)> {
        if self.height == 0 || self.width == 0 {
            return Err(Error::invalid("image dimensions cannot be zero"));
        }
        let packed_stride = self
            .width
            .checked_mul(self.format.channels())
            .ok_or_else(|| Error::invalid("image row size overflows usize"))?;
        let stride = if self.row_stride == 0 {
            packed_stride
        } else {
            self.row_stride
        };
        if stride < packed_stride {
            return Err(Error::invalid(
                "image row stride is smaller than the packed row",
            ));
        }
        let required = self
            .height
            .saturating_sub(1)
            .checked_mul(stride)
            .and_then(|size| size.checked_add(packed_stride))
            .ok_or_else(|| Error::invalid("image byte size overflows usize"))?;
        if self.data.len() < required {
            return Err(Error::invalid(format!(
                "image buffer has {} bytes, requires at least {required}",
                self.data.len()
            )));
        }
        Ok((stride, stride == packed_stride))
    }

    fn image_data(&self) -> Result<ImageData> {
        let (stride, contiguous) = self.validate()?;
        Ok(ImageData {
            size: [self.height, self.width],
            stride,
            contiguous,
            image_type: self.format.core(),
            data: self.data.as_ptr(),
        })
    }

    fn image_multi_data(&self) -> Result<ImageMultiData> {
        let (stride, contiguous) = self.validate()?;
        Ok(ImageMultiData {
            size: [self.height, self.width],
            stride,
            contiguous,
            image_type: self.format.core(),
            data: self.data.as_ptr(),
        })
    }
}

struct ValuesInner {
    values: NoHashMap<u64, (Arc<ValueCore>, ObjectType)>,
    values_take: NoHashMap<u64, (Arc<ValueTakeCore>, ObjectType)>,
    static_values: NoHashMap<u64, (Arc<ValueStaticCore>, ObjectType)>,
    signals: NoHashMap<u64, (Arc<SignalCore>, ObjectType)>,
    signal_types: NoHashMap<u64, ObjectType>,
    maps: NoHashMap<u64, (Arc<ValueMap>, ObjectType, ObjectType)>,
    lists: NoHashMap<u64, (Arc<ValueList>, ObjectType)>,
    images: NoHashMap<u64, Arc<Image>>,
    image_multi: NoHashMap<u64, Arc<ImageMulti>>,
    data: NoHashMap<u64, Arc<Data>>,
    data_take: NoHashMap<u64, Arc<DataTake>>,
    data_multi: NoHashMap<u64, Arc<DataMulti>>,
    data_multi_take: NoHashMap<u64, Arc<DataMultiTake>>,
}

pub struct DynamicServer {
    server: RwLock<Server>,
    signals: SignalsManager,
    inner: OnceLock<ValuesInner>,
    types: RwLock<Option<NoHashMap<u64, ObjectType>>>,
}

impl DynamicServer {
    pub fn new(version: Option<u64>) -> Self {
        let server = Server::new(version);
        let signals = server.get_signals_manager();
        let mut types = NoHashMap::default();
        types.insert(
            LOGGING_ID,
            ObjectType::Tuple(vec![ObjectType::U8, ObjectType::String]),
        );
        types.insert(ON_CONNECT_ID, ObjectType::String);
        types.insert(ON_DISCONNECT_ID, ObjectType::Empty);
        types.insert(CLIENT_MESSAGE_ID, ObjectType::String);
        Self {
            server: RwLock::new(server),
            signals,
            inner: OnceLock::new(),
            types: RwLock::new(Some(types)),
        }
    }

    fn values(&self) -> Result<&ValuesInner> {
        self.inner
            .get()
            .ok_or_else(|| Error::state("server has not been finalized"))
    }

    fn save_type(&self, id: u64, object_type: ObjectType) -> Result<()> {
        let mut types = self.types.write();
        let types = types
            .as_mut()
            .ok_or_else(|| Error::state("cannot register states after finalization"))?;
        types.insert(id, object_type);
        Ok(())
    }

    pub fn finalize(&self) -> Result<()> {
        let states = self.server.write().finalize();
        let types = self.types.write().take();
        match (states, types) {
            (None, None) => return Ok(()),
            (Some(_), None) | (None, Some(_)) => {
                return Err(Error::state("inconsistent state during finalization"));
            }
            (Some(states), Some(mut types)) => {
                let mut values = NoHashMap::default();
                for (id, value) in states.values {
                    let object_type = types
                        .get(&id)
                        .cloned()
                        .ok_or_else(|| Error::state(format!("missing type for value {id}")))?;
                    values.insert(id, (value, object_type));
                }
                let mut values_take = NoHashMap::default();
                for (id, value) in states.values_take {
                    let object_type = types
                        .get(&id)
                        .cloned()
                        .ok_or_else(|| Error::state(format!("missing type for value take {id}")))?;
                    values_take.insert(id, (value, object_type));
                }
                let mut static_values = NoHashMap::default();
                for (id, value) in states.static_values {
                    let object_type = types
                        .remove(&id)
                        .ok_or_else(|| Error::state(format!("missing type for static {id}")))?;
                    static_values.insert(id, (value, object_type));
                }
                let mut signal_values = NoHashMap::default();
                for (id, value) in states.signals {
                    let object_type = types
                        .get(&id)
                        .cloned()
                        .ok_or_else(|| Error::state(format!("missing type for signal {id}")))?;
                    signal_values.insert(id, (value, object_type));
                }
                let mut maps = NoHashMap::default();
                for (id, value) in states.maps {
                    let object_type = types
                        .remove(&id)
                        .ok_or_else(|| Error::state(format!("missing type for map {id}")))?;
                    let ObjectType::Map(key_type, value_type) = object_type else {
                        return Err(Error::state(format!("invalid stored map type for {id}")));
                    };
                    maps.insert(id, (value, *key_type, *value_type));
                }
                let mut lists = NoHashMap::default();
                for (id, value) in states.lists {
                    let object_type = types
                        .remove(&id)
                        .ok_or_else(|| Error::state(format!("missing type for list {id}")))?;
                    lists.insert(id, (value, object_type));
                }
                let inner = ValuesInner {
                    values,
                    values_take,
                    static_values,
                    signals: signal_values,
                    signal_types: types,
                    maps,
                    lists,
                    images: states.images,
                    image_multi: states.image_multi,
                    data: states.data,
                    data_take: states.data_take,
                    data_multi: states.data_multi,
                    data_multi_take: states.data_multi_take,
                };
                self.inner
                    .set(inner)
                    .map_err(|_| Error::state("server has already been finalized"))?;
            }
        }
        Ok(())
    }

    pub fn start(&self, port: u16, ip: Option<[u8; 4]>, token: Option<String>) -> Result<()> {
        self.values()?;
        let ip = ip
            .map(|value| Ipv4Addr::new(value[0], value[1], value[2], value[3]))
            .unwrap_or(Ipv4Addr::UNSPECIFIED);
        self.server
            .write()
            .start(SocketAddrV4::new(ip, port), token)
            .map_err(|error| Error::new(ErrorKind::Io, error.to_string()))
    }

    pub fn stop(&self) {
        self.server.write().stop();
    }

    pub fn disconnect_client(&self) {
        self.server.write().disconnect_client();
    }

    pub fn is_running(&self) -> bool {
        self.server.read().is_running()
    }

    pub fn is_connected(&self) -> bool {
        self.server.read().is_connected()
    }

    pub fn update(&self, duration: Option<f32>) -> Result<()> {
        self.server
            .read()
            .update(duration)
            .map_err(|_| Error::new(ErrorKind::Io, "failed to send update"))
    }

    pub fn add_value(
        &self,
        name: &str,
        object_type: &ObjectType,
        initial: &DynamicValue,
        queue: bool,
    ) -> Result<u64> {
        let data = serialize_value(initial, object_type)?;
        let id = self
            .server
            .write()
            .add_value(name, object_type.get_hash(), data, queue)
            .map_err(Error::core)?;
        self.save_type(id, object_type.clone())?;
        Ok(id)
    }

    pub fn add_value_take(&self, name: &str, object_type: &ObjectType) -> Result<u64> {
        let id = self
            .server
            .write()
            .add_value_take(name, object_type.get_hash())
            .map_err(Error::core)?;
        self.save_type(id, object_type.clone())?;
        Ok(id)
    }

    pub fn add_static(
        &self,
        name: &str,
        object_type: &ObjectType,
        initial: &DynamicValue,
    ) -> Result<u64> {
        let data = serialize_value(initial, object_type)?;
        let id = self
            .server
            .write()
            .add_static(name, object_type.get_hash(), data)
            .map_err(Error::core)?;
        self.save_type(id, object_type.clone())?;
        Ok(id)
    }

    pub fn add_signal(&self, name: &str, object_type: &ObjectType, queue: bool) -> Result<u64> {
        let id = self
            .server
            .write()
            .add_signal(name, object_type.get_hash(), queue)
            .map_err(Error::core)?;
        self.save_type(id, object_type.clone())?;
        Ok(id)
    }

    pub fn add_list(&self, name: &str, object_type: &ObjectType) -> Result<u64> {
        let id = self
            .server
            .write()
            .add_vec(name, object_type.get_hash())
            .map_err(Error::core)?;
        self.save_type(id, object_type.clone())?;
        Ok(id)
    }

    pub fn add_map(
        &self,
        name: &str,
        key_type: &ObjectType,
        value_type: &ObjectType,
    ) -> Result<u64> {
        let type_id = value_type.get_hash_from(key_type.get_hash());
        let id = self
            .server
            .write()
            .add_map(name, type_id)
            .map_err(Error::core)?;
        self.save_type(
            id,
            ObjectType::Map(Box::new(key_type.clone()), Box::new(value_type.clone())),
        )?;
        Ok(id)
    }

    pub fn add_image(&self, name: &str) -> Result<u64> {
        self.server.write().add_image(name).map_err(Error::core)
    }

    pub fn add_image_multi(&self, name: &str) -> Result<u64> {
        self.server
            .write()
            .add_image_multi(name)
            .map_err(Error::core)
    }

    pub fn add_data(&self, name: &str, data_type: DynamicDataType) -> Result<u64> {
        self.server
            .write()
            .add_data(name, data_type as u8)
            .map_err(Error::core)
    }

    pub fn add_data_take(&self, name: &str, data_type: DynamicDataType) -> Result<u64> {
        self.server
            .write()
            .add_data_take(name, data_type as u8)
            .map_err(Error::core)
    }

    pub fn add_data_multi(&self, name: &str, data_type: DynamicDataType) -> Result<u64> {
        self.server
            .write()
            .add_data_multi(name, data_type as u8)
            .map_err(Error::core)
    }

    pub fn add_data_multi_take(&self, name: &str, data_type: DynamicDataType) -> Result<u64> {
        self.server
            .write()
            .add_data_multi_take(name, data_type as u8)
            .map_err(Error::core)
    }
}

impl DynamicServer {
    pub fn id_to_name(&self, id: u64) -> Result<String> {
        let values = self.values()?;
        if let Some((value, _)) = values.values.get(&id) {
            return Ok(value.name.clone());
        }
        if let Some((value, _)) = values.values_take.get(&id) {
            return Ok(value.name.clone());
        }
        if let Some((value, _)) = values.static_values.get(&id) {
            return Ok(value.name.clone());
        }
        if let Some((value, _)) = values.signals.get(&id) {
            return Ok(value.name.clone());
        }
        if let Some((value, _, _)) = values.maps.get(&id) {
            return Ok(value.name.clone());
        }
        if let Some((value, _)) = values.lists.get(&id) {
            return Ok(value.name.clone());
        }
        if let Some(value) = values.images.get(&id) {
            return Ok(value.name.clone());
        }
        if let Some(value) = values.image_multi.get(&id) {
            return Ok(value.name.clone());
        }
        if let Some(value) = values.data.get(&id) {
            return Ok(value.name.clone());
        }
        if let Some(value) = values.data_take.get(&id) {
            return Ok(value.name.clone());
        }
        if let Some(value) = values.data_multi.get(&id) {
            return Ok(value.name.clone());
        }
        if let Some(value) = values.data_multi_take.get(&id) {
            return Ok(value.name.clone());
        }
        Err(Error::missing(format!("state id {id} was not found")))
    }

    pub fn value_get(&self, id: u64) -> Result<DynamicValue> {
        let (value, object_type) = self
            .values()?
            .values
            .get(&id)
            .ok_or_else(|| Error::missing(format!("value id {id} was not found")))?;
        deserialize_value(value.get(), object_type)
    }

    pub fn value_set(
        &self,
        id: u64,
        value: &DynamicValue,
        set_signal: bool,
        update: bool,
    ) -> Result<()> {
        let (inner, object_type) = self
            .values()?
            .values
            .get(&id)
            .ok_or_else(|| Error::missing(format!("value id {id} was not found")))?;
        inner
            .set(serialize_value(value, object_type)?, set_signal, update)
            .map_err(Error::core)
    }

    pub fn value_take_set(
        &self,
        id: u64,
        value: &DynamicValue,
        blocking: bool,
        update: bool,
    ) -> Result<()> {
        let (inner, object_type) = self
            .values()?
            .values_take
            .get(&id)
            .ok_or_else(|| Error::missing(format!("value-take id {id} was not found")))?;
        inner
            .set(serialize_value(value, object_type)?, blocking, update)
            .map_err(Error::core)
    }

    pub fn static_get(&self, id: u64) -> Result<DynamicValue> {
        let (value, object_type) = self
            .values()?
            .static_values
            .get(&id)
            .ok_or_else(|| Error::missing(format!("static id {id} was not found")))?;
        deserialize_value(value.get(), object_type)
    }

    pub fn static_set(&self, id: u64, value: &DynamicValue, update: bool) -> Result<()> {
        let (inner, object_type) = self
            .values()?
            .static_values
            .get(&id)
            .ok_or_else(|| Error::missing(format!("static id {id} was not found")))?;
        inner
            .set(serialize_value(value, object_type)?, update)
            .map_err(Error::core)
    }

    pub fn signal_set(&self, id: u64, value: &DynamicValue) -> Result<()> {
        let (inner, object_type) = self
            .values()?
            .signals
            .get(&id)
            .ok_or_else(|| Error::missing(format!("signal id {id} was not found")))?;
        inner.set(serialize_value(value, object_type)?);
        Ok(())
    }

    pub fn signal_register(&self, id: u64, register: bool, with_previous: bool) -> Result<()> {
        if register && !self.values()?.signal_types.contains_key(&id) {
            return Err(Error::missing(format!(
                "signal-capable id {id} was not found"
            )));
        }
        self.signals.set_register(id, register, with_previous);
        Ok(())
    }

    pub fn signal_set_to_queue(&self, id: u64) -> Result<()> {
        if !self.values()?.signal_types.contains_key(&id) {
            return Err(Error::missing(format!(
                "signal-capable id {id} was not found"
            )));
        }
        self.signals.set_to_queue(id);
        Ok(())
    }

    pub fn signal_set_to_single(&self, id: u64) -> Result<()> {
        if !self.values()?.signal_types.contains_key(&id) {
            return Err(Error::missing(format!(
                "signal-capable id {id} was not found"
            )));
        }
        self.signals.set_to_single(id);
        Ok(())
    }

    pub fn signal_wait(&self, timeout: Option<Duration>) -> Result<DynamicSignalEvent> {
        let (id, data, previous) = self
            .signals
            .wait_changed_value_timeout(timeout)
            .ok_or_else(|| Error::new(ErrorKind::Timeout, "signal wait timed out"))?;
        let parsed = (|| {
            let object_type =
                self.values()?.signal_types.get(&id).ok_or_else(|| {
                    Error::missing(format!("signal type for id {id} was not found"))
                })?;
            let value = deserialize_value(data, object_type)?;
            let previous = previous
                .map(|data| deserialize_value(data, object_type))
                .transpose()?;
            Ok((value, previous))
        })();
        match parsed {
            Ok((value, previous)) => Ok(DynamicSignalEvent {
                signals: self.signals.clone(),
                id,
                value,
                previous,
            }),
            Err(error) => {
                self.signals.release(id);
                Err(error)
            }
        }
    }

    fn list(&self, id: u64) -> Result<(&Arc<ValueList>, &ObjectType)> {
        self.values()?
            .lists
            .get(&id)
            .map(|(list, object_type)| (list, object_type))
            .ok_or_else(|| Error::missing(format!("list id {id} was not found")))
    }

    pub fn list_set(&self, id: u64, value: &DynamicValue, update: bool) -> Result<()> {
        let DynamicValue::Sequence(items) = value else {
            return Err(mismatch("sequence", value));
        };
        let (list, object_type) = self.list(id)?;
        let data = items
            .iter()
            .map(|value| serialize_value(value, object_type))
            .collect::<Result<Vec<_>>>()?;
        list.set(data, update).map_err(Error::core)
    }

    pub fn list_get(&self, id: u64) -> Result<DynamicValue> {
        let (list, object_type) = self.list(id)?;
        let values = list
            .get()
            .into_iter()
            .map(|data| deserialize_value(data, object_type))
            .collect::<Result<Vec<_>>>()?;
        Ok(DynamicValue::Sequence(values))
    }

    pub fn list_set_item(
        &self,
        id: u64,
        index: usize,
        value: &DynamicValue,
        update: bool,
    ) -> Result<()> {
        let (list, object_type) = self.list(id)?;
        list.set_item_py(index, serialize_value(value, object_type)?, update)
            .map_err(Error::core)
    }

    pub fn list_get_item(&self, id: u64, index: usize) -> Result<DynamicValue> {
        let (list, object_type) = self.list(id)?;
        let data = list
            .get_item(index)
            .map_err(|error| Error::new(ErrorKind::OutOfRange, error))?;
        deserialize_value(data, object_type)
    }

    pub fn list_remove_item(&self, id: u64, index: usize, update: bool) -> Result<DynamicValue> {
        let (list, object_type) = self.list(id)?;
        let data = list
            .remove_item(index, update)
            .map_err(|error| Error::new(ErrorKind::OutOfRange, error))?;
        deserialize_value(data, object_type)
    }

    pub fn list_append_item(&self, id: u64, value: &DynamicValue, update: bool) -> Result<()> {
        let (list, object_type) = self.list(id)?;
        list.append_item(serialize_value(value, object_type)?, update)
            .map_err(Error::core)
    }

    pub fn list_len(&self, id: u64) -> Result<usize> {
        Ok(self.list(id)?.0.len())
    }

    fn map(&self, id: u64) -> Result<(&Arc<ValueMap>, &ObjectType, &ObjectType)> {
        self.values()?
            .maps
            .get(&id)
            .map(|(map, key, value)| (map, key, value))
            .ok_or_else(|| Error::missing(format!("map id {id} was not found")))
    }

    pub fn map_set(&self, id: u64, value: &DynamicValue, update: bool) -> Result<()> {
        let DynamicValue::Map(entries) = value else {
            return Err(mismatch("map", value));
        };
        let (map, key_type, value_type) = self.map(id)?;
        let mut data = HashMap::with_capacity(entries.len());
        for (key, value) in entries {
            data.insert(
                serialize_value(key, key_type)?,
                serialize_value(value, value_type)?,
            );
        }
        map.set(data, update).map_err(Error::core)
    }

    pub fn map_get(&self, id: u64) -> Result<DynamicValue> {
        let (map, key_type, value_type) = self.map(id)?;
        let mut entries = Vec::with_capacity(map.len());
        for (key, value) in map.get() {
            entries.push((
                deserialize_value(key, key_type)?,
                deserialize_value(value, value_type)?,
            ));
        }
        Ok(DynamicValue::Map(entries))
    }

    pub fn map_set_item(
        &self,
        id: u64,
        key: &DynamicValue,
        value: &DynamicValue,
        update: bool,
    ) -> Result<()> {
        let (map, key_type, value_type) = self.map(id)?;
        map.set_item(
            serialize_value(key, key_type)?,
            serialize_value(value, value_type)?,
            update,
        )
        .map_err(Error::core)
    }

    pub fn map_get_item(&self, id: u64, key: &DynamicValue) -> Result<DynamicValue> {
        let (map, key_type, value_type) = self.map(id)?;
        let key = serialize_value(key, key_type)?;
        let value = map
            .get_item(&key)
            .ok_or_else(|| Error::missing("map key was not found"))?;
        deserialize_value(value, value_type)
    }

    pub fn map_remove_item(
        &self,
        id: u64,
        key: &DynamicValue,
        update: bool,
    ) -> Result<DynamicValue> {
        let (map, key_type, value_type) = self.map(id)?;
        let key = serialize_value(key, key_type)?;
        let value = map
            .remove_item(&key, update)
            .map_err(|_| Error::new(ErrorKind::Internal, "failed to remove map item"))?
            .ok_or_else(|| Error::missing("map key was not found"))?;
        deserialize_value(value, value_type)
    }

    pub fn map_len(&self, id: u64) -> Result<usize> {
        Ok(self.map(id)?.0.len())
    }
}

pub struct DynamicSignalEvent {
    signals: SignalsManager,
    id: u64,
    value: DynamicValue,
    previous: Option<DynamicValue>,
}

impl DynamicSignalEvent {
    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn value(&self) -> &DynamicValue {
        &self.value
    }

    pub fn previous(&self) -> Option<&DynamicValue> {
        self.previous.as_ref()
    }
}

impl Drop for DynamicSignalEvent {
    fn drop(&mut self) {
        self.signals.release(self.id);
    }
}

impl DynamicServer {
    fn image(&self, id: u64) -> Result<&Arc<Image>> {
        self.values()?
            .images
            .get(&id)
            .ok_or_else(|| Error::missing(format!("image id {id} was not found")))
    }

    pub fn image_size(&self, id: u64) -> Result<[usize; 2]> {
        Ok(self.image(id)?.get_size())
    }

    pub fn image_get(&self, id: u64) -> Result<(Vec<u8>, [usize; 2])> {
        Ok(self
            .image(id)?
            .get_image(|(data, size)| (data.clone(), *size)))
    }

    pub fn image_set(&self, id: u64, view: &DynamicImageView<'_>, update: bool) -> Result<()> {
        self.image(id)?
            .set_image(view.image_data()?, update)
            .map_err(Error::core)
    }

    pub fn image_set_all(
        &self,
        id: u64,
        size: [usize; 2],
        rgba: [u8; 4],
        update: bool,
    ) -> Result<()> {
        self.image(id)?
            .set_all_image(size, rgba, update)
            .map_err(Error::core)
    }

    pub fn image_update(
        &self,
        id: u64,
        origin: [usize; 2],
        view: &DynamicImageView<'_>,
        update: bool,
        force: bool,
    ) -> Result<()> {
        checked_image_rect(&origin, [view.height, view.width]).map_err(Error::core)?;
        self.image(id)?
            .update_image(&origin, view.image_data()?, update, force)
            .map_err(Error::core)
    }

    fn image_multi(&self, id: u64) -> Result<&Arc<ImageMulti>> {
        self.values()?
            .image_multi
            .get(&id)
            .ok_or_else(|| Error::missing(format!("image-multi id {id} was not found")))
    }

    pub fn image_multi_size(&self, id: u64, index: u32) -> Result<[usize; 2]> {
        self.image_multi(id)?
            .get_size(index)
            .ok_or_else(|| Error::missing(format!("image-multi index {index} was not found")))
    }

    pub fn image_multi_get(&self, id: u64, index: u32) -> Result<(Vec<u8>, [usize; 2])> {
        self.image_multi(id)?.get_image(index, |image| {
            image
                .map(|(data, size)| (data.clone(), *size))
                .ok_or_else(|| Error::missing(format!("image-multi index {index} was not found")))
        })
    }

    pub fn image_multi_set(
        &self,
        id: u64,
        index: u32,
        view: &DynamicImageView<'_>,
        update: bool,
    ) -> Result<()> {
        self.image_multi(id)?
            .set_image(index, view.image_multi_data()?, update)
            .map_err(Error::core)
    }

    pub fn image_multi_set_all(
        &self,
        id: u64,
        index: u32,
        size: [usize; 2],
        rgba: [u8; 4],
        update: bool,
    ) -> Result<()> {
        self.image_multi(id)?
            .set_all_image(index, size, rgba, update)
            .map_err(Error::core)
    }

    pub fn image_multi_update(
        &self,
        id: u64,
        index: u32,
        origin: [usize; 2],
        view: &DynamicImageView<'_>,
        update: bool,
        force: bool,
    ) -> Result<()> {
        self.image_multi(id)?
            .update_image(index, &origin, view.image_multi_data()?, update, force)
            .map_err(Error::core)
    }

    pub fn image_multi_remove_index(&self, id: u64, index: u32, update: bool) -> Result<()> {
        self.image_multi(id)?
            .remove_index(index, update)
            .map_err(Error::core)
    }

    pub fn image_multi_reset(&self, id: u64, update: bool) -> Result<()> {
        self.image_multi(id)?
            .reset_images(update)
            .map_err(Error::core)
    }

    pub fn image_multi_len(&self, id: u64) -> Result<usize> {
        Ok(self.image_multi(id)?.len())
    }

    pub fn image_multi_contains(&self, id: u64, index: u32) -> Result<bool> {
        Ok(self.image_multi(id)?.contains(index))
    }

    pub fn image_multi_indices(&self, id: u64) -> Result<Vec<u32>> {
        Ok(self.image_multi(id)?.indices())
    }

    fn data(&self, id: u64) -> Result<&Arc<Data>> {
        self.values()?
            .data
            .get(&id)
            .ok_or_else(|| Error::missing(format!("data id {id} was not found")))
    }

    fn data_take(&self, id: u64) -> Result<&Arc<DataTake>> {
        self.values()?
            .data_take
            .get(&id)
            .ok_or_else(|| Error::missing(format!("data-take id {id} was not found")))
    }

    fn data_multi(&self, id: u64) -> Result<&Arc<DataMulti>> {
        self.values()?
            .data_multi
            .get(&id)
            .ok_or_else(|| Error::missing(format!("data-multi id {id} was not found")))
    }

    fn data_multi_take(&self, id: u64) -> Result<&Arc<DataMultiTake>> {
        self.values()?
            .data_multi_take
            .get(&id)
            .ok_or_else(|| Error::missing(format!("data-multi-take id {id} was not found")))
    }

    pub fn data_get(&self, id: u64) -> Result<Vec<u8>> {
        Ok(self.data(id)?.get(<[u8]>::to_vec))
    }

    pub fn data_set(&self, id: u64, bytes: &[u8], count: usize, update: bool) -> Result<()> {
        let data = self.data(id)?;
        data.set(data_holder(bytes, count, data.data_type)?, update)
            .map_err(Error::core)
    }

    pub fn data_add(&self, id: u64, bytes: &[u8], count: usize, update: bool) -> Result<()> {
        let data = self.data(id)?;
        data.add(data_holder(bytes, count, data.data_type)?, update)
            .map_err(Error::core)
    }

    pub fn data_replace(
        &self,
        id: u64,
        bytes: &[u8],
        count: usize,
        index: usize,
        update: bool,
    ) -> Result<()> {
        index.checked_add(count).ok_or_else(|| {
            Error::new(ErrorKind::OutOfRange, "numeric data range overflows usize")
        })?;
        let data = self.data(id)?;
        data.replace(data_holder(bytes, count, data.data_type)?, index, update)
            .map_err(Error::core)
    }

    pub fn data_remove(&self, id: u64, index: usize, count: usize, update: bool) -> Result<()> {
        index.checked_add(count).ok_or_else(|| {
            Error::new(ErrorKind::OutOfRange, "numeric data range overflows usize")
        })?;
        self.data(id)?
            .remove(index, count, update)
            .map_err(Error::core)
    }

    pub fn data_clear(&self, id: u64, update: bool) -> Result<()> {
        self.data(id)?.clear(update).map_err(Error::core)
    }

    pub fn data_take_set(
        &self,
        id: u64,
        bytes: &[u8],
        count: usize,
        blocking: bool,
        update: bool,
        cache: bool,
    ) -> Result<()> {
        let data = self.data_take(id)?;
        data.set(
            data_holder(bytes, count, data.data_type)?,
            blocking,
            update,
            cache,
        )
        .map_err(Error::core)
    }

    pub fn data_multi_get(&self, id: u64, index: u32) -> Result<Vec<u8>> {
        self.data_multi(id)?.get(index, |data| {
            data.map(<[u8]>::to_vec)
                .ok_or_else(|| Error::missing(format!("data-multi index {index} was not found")))
        })
    }

    pub fn data_multi_set(
        &self,
        id: u64,
        index: u32,
        bytes: &[u8],
        count: usize,
        update: bool,
    ) -> Result<()> {
        let data = self.data_multi(id)?;
        data.set(index, data_holder(bytes, count, data.data_type)?, update)
            .map_err(Error::core)
    }

    pub fn data_multi_add(
        &self,
        id: u64,
        index: u32,
        bytes: &[u8],
        count: usize,
        update: bool,
    ) -> Result<()> {
        let data = self.data_multi(id)?;
        data.add(index, data_holder(bytes, count, data.data_type)?, update)
            .map_err(Error::core)
    }

    pub fn data_multi_replace(
        &self,
        id: u64,
        index: u32,
        bytes: &[u8],
        count: usize,
        data_index: usize,
        update: bool,
    ) -> Result<()> {
        data_index.checked_add(count).ok_or_else(|| {
            Error::new(
                ErrorKind::OutOfRange,
                "numeric data-multi range overflows usize",
            )
        })?;
        let data = self.data_multi(id)?;
        data.replace(
            index,
            data_index,
            data_holder(bytes, count, data.data_type)?,
            update,
        )
        .map_err(Error::core)
    }

    pub fn data_multi_remove(
        &self,
        id: u64,
        index: u32,
        data_index: usize,
        count: usize,
        update: bool,
    ) -> Result<()> {
        data_index.checked_add(count).ok_or_else(|| {
            Error::new(
                ErrorKind::OutOfRange,
                "numeric data-multi range overflows usize",
            )
        })?;
        self.data_multi(id)?
            .remove(index, data_index, count, update)
            .map_err(Error::core)
    }

    pub fn data_multi_clear(&self, id: u64, index: u32, update: bool) -> Result<()> {
        self.data_multi(id)?
            .clear(index, update)
            .map_err(Error::core)
    }

    pub fn data_multi_remove_index(&self, id: u64, index: u32, update: bool) -> Result<()> {
        self.data_multi(id)?
            .remove_index(index, update)
            .map_err(Error::core)
    }

    pub fn data_multi_reset(&self, id: u64, update: bool) -> Result<()> {
        self.data_multi(id)?.reset(update).map_err(Error::core)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn data_multi_take_set(
        &self,
        id: u64,
        index: u32,
        bytes: &[u8],
        count: usize,
        blocking: bool,
        update: bool,
        cache: bool,
    ) -> Result<()> {
        let data = self.data_multi_take(id)?;
        data.set(
            index,
            data_holder(bytes, count, data.data_type)?,
            blocking,
            update,
            cache,
        )
        .map_err(Error::core)
    }

    pub fn data_multi_take_remove_index(&self, id: u64, index: u32, update: bool) -> Result<()> {
        self.data_multi_take(id)?
            .remove_index(index, update)
            .map_err(Error::core)
    }

    pub fn data_multi_take_reset(&self, id: u64, update: bool) -> Result<()> {
        self.data_multi_take(id)?.reset(update).map_err(Error::core)
    }
}

fn data_holder(bytes: &[u8], count: usize, data_type: DataType) -> Result<DataHolder> {
    let expected = count
        .checked_mul(data_type.item_size())
        .ok_or_else(|| Error::invalid("numeric data byte size overflows usize"))?;
    if bytes.len() != expected {
        return Err(Error::invalid(format!(
            "numeric data has {} bytes, expected {expected} for {count} elements",
            bytes.len()
        )));
    }
    Ok(DataHolder {
        data: bytes.as_ptr(),
        count,
        data_size: bytes.len(),
        data_type,
    })
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Barrier, mpsc};
    use std::thread;

    use super::*;

    #[test]
    fn dynamic_codec_round_trips_nested_values() {
        let object_type = ObjectType::Struct(
            "Example".to_owned(),
            vec![
                ("number".to_owned(), ObjectType::I32),
                (
                    "choice".to_owned(),
                    ObjectType::Enum(
                        "Choice".to_owned(),
                        vec![("A".to_owned(), 10), ("B".to_owned(), 20)],
                    ),
                ),
                (
                    "tuple".to_owned(),
                    ObjectType::Tuple(vec![ObjectType::Bool, ObjectType::String]),
                ),
                (
                    "array".to_owned(),
                    ObjectType::List(2, Box::new(ObjectType::U16)),
                ),
                ("vec".to_owned(), ObjectType::Vec(Box::new(ObjectType::F64))),
                (
                    "map".to_owned(),
                    ObjectType::Map(Box::new(ObjectType::U8), Box::new(ObjectType::String)),
                ),
                (
                    "optional".to_owned(),
                    ObjectType::Option(Box::new(ObjectType::U64)),
                ),
            ],
        );
        let value = DynamicValue::Sequence(vec![
            DynamicValue::I32(-12),
            DynamicValue::Enum(1),
            DynamicValue::Sequence(vec![
                DynamicValue::Bool(true),
                DynamicValue::String("hello".to_owned()),
            ]),
            DynamicValue::Sequence(vec![DynamicValue::U16(2), DynamicValue::U16(3)]),
            DynamicValue::Sequence(vec![DynamicValue::F64(1.5), DynamicValue::F64(-2.0)]),
            DynamicValue::Map(vec![(
                DynamicValue::U8(7),
                DynamicValue::String("seven".to_owned()),
            )]),
            DynamicValue::Option(Some(Box::new(DynamicValue::U64(99)))),
        ]);

        let encoded = serialize_value(&value, &object_type).unwrap();
        assert_eq!(deserialize_value(encoded, &object_type).unwrap(), value);
        for (object_type, value) in [
            (ObjectType::U8, DynamicValue::U8(1)),
            (ObjectType::U16, DynamicValue::U16(2)),
            (ObjectType::U32, DynamicValue::U32(3)),
            (ObjectType::U64, DynamicValue::U64(4)),
            (ObjectType::I8, DynamicValue::I8(-1)),
            (ObjectType::I16, DynamicValue::I16(-2)),
            (ObjectType::I32, DynamicValue::I32(-3)),
            (ObjectType::I64, DynamicValue::I64(-4)),
            (ObjectType::F32, DynamicValue::F32(1.25)),
            (ObjectType::F64, DynamicValue::F64(-2.5)),
            (ObjectType::Bool, DynamicValue::Bool(true)),
            (ObjectType::String, DynamicValue::String("utf-8 ✓".into())),
            (
                ObjectType::Option(Box::new(ObjectType::U8)),
                DynamicValue::Option(None),
            ),
            (ObjectType::Empty, DynamicValue::Empty),
        ] {
            let encoded = serialize_value(&value, &object_type).unwrap();
            assert_eq!(deserialize_value(encoded, &object_type).unwrap(), value);
        }
        assert_eq!(
            serialize_value(&DynamicValue::String("wrong".to_owned()), &ObjectType::I32)
                .unwrap_err()
                .kind,
            ErrorKind::TypeMismatch
        );
        assert_eq!(
            serialize_value(
                &DynamicValue::Enum(2),
                &ObjectType::Enum("Choice".into(), vec![("A".into(), 10), ("B".into(), 20)],),
            )
            .unwrap_err()
            .kind,
            ErrorKind::InvalidArgument
        );
        assert_eq!(
            serialize_value(
                &DynamicValue::Sequence(vec![DynamicValue::U8(1)]),
                &ObjectType::List(2, Box::new(ObjectType::U8)),
            )
            .unwrap_err()
            .kind,
            ErrorKind::TypeMismatch
        );
        let mut trailing = serialize_value(&DynamicValue::U8(1), &ObjectType::U8)
            .unwrap()
            .to_vec();
        trailing.push(0);
        assert_eq!(
            deserialize_value(Bytes::from(trailing), &ObjectType::U8)
                .unwrap_err()
                .kind,
            ErrorKind::TypeMismatch
        );
    }

    #[test]
    fn dynamic_server_exercises_every_state_family() {
        let server = DynamicServer::new(Some(5));
        let value_id = server
            .add_value("root.value", &ObjectType::I32, &DynamicValue::I32(1), true)
            .unwrap();
        let take_id = server
            .add_value_take("root.take", &ObjectType::String)
            .unwrap();
        let static_id = server
            .add_static("root.static", &ObjectType::Bool, &DynamicValue::Bool(false))
            .unwrap();
        let signal_id = server
            .add_signal("root.signal", &ObjectType::Empty, false)
            .unwrap();
        let list_id = server.add_list("root.list", &ObjectType::U16).unwrap();
        let map_id = server
            .add_map("root.map", &ObjectType::U8, &ObjectType::String)
            .unwrap();
        let image_id = server.add_image("root.image").unwrap();
        let image_multi_id = server.add_image_multi("root.images").unwrap();
        let data_id = server.add_data("root.data", DynamicDataType::U16).unwrap();
        let data_take_id = server
            .add_data_take("root.data_take", DynamicDataType::U8)
            .unwrap();
        let data_multi_id = server
            .add_data_multi("root.data_multi", DynamicDataType::I32)
            .unwrap();
        let data_multi_take_id = server
            .add_data_multi_take("root.data_multi_take", DynamicDataType::F32)
            .unwrap();
        server.finalize().unwrap();
        server.finalize().unwrap();
        assert_eq!(server.id_to_name(value_id).unwrap(), "root.value");
        assert_eq!(
            server.id_to_name(u64::MAX).unwrap_err().kind,
            ErrorKind::NotFound
        );
        assert_eq!(
            server.add_image("root.too_late").unwrap_err().kind,
            ErrorKind::InvalidState
        );

        assert_eq!(server.value_get(value_id).unwrap(), DynamicValue::I32(1));
        server.signal_register(value_id, true, true).unwrap();
        server
            .value_set(value_id, &DynamicValue::I32(2), true, false)
            .unwrap();
        let event = server.signal_wait(Some(Duration::ZERO)).unwrap();
        assert_eq!(event.id(), value_id);
        assert_eq!(event.value(), &DynamicValue::I32(2));
        assert_eq!(event.previous(), Some(&DynamicValue::I32(1)));
        drop(event);
        match server.signal_wait(Some(Duration::ZERO)) {
            Err(error) => assert_eq!(error.kind, ErrorKind::Timeout),
            Ok(_) => panic!("signal wait unexpectedly returned an event"),
        }

        server
            .value_take_set(take_id, &DynamicValue::String("x".into()), false, false)
            .unwrap();
        server
            .static_set(static_id, &DynamicValue::Bool(true), false)
            .unwrap();
        assert_eq!(
            server.static_get(static_id).unwrap(),
            DynamicValue::Bool(true)
        );
        server.signal_register(signal_id, true, false).unwrap();
        server.signal_set(signal_id, &DynamicValue::Empty).unwrap();
        assert_eq!(
            server.signal_wait(Some(Duration::ZERO)).unwrap().value(),
            &DynamicValue::Empty
        );

        server
            .list_set(
                list_id,
                &DynamicValue::Sequence(vec![DynamicValue::U16(1), DynamicValue::U16(2)]),
                false,
            )
            .unwrap();
        assert_eq!(
            server.list_get(list_id).unwrap(),
            DynamicValue::Sequence(vec![DynamicValue::U16(1), DynamicValue::U16(2)])
        );
        server
            .list_set_item(list_id, 1, &DynamicValue::U16(4), false)
            .unwrap();
        assert_eq!(
            server.list_get_item(list_id, 1).unwrap(),
            DynamicValue::U16(4)
        );
        server
            .list_append_item(list_id, &DynamicValue::U16(3), false)
            .unwrap();
        assert_eq!(server.list_len(list_id).unwrap(), 3);
        assert_eq!(
            server.list_remove_item(list_id, 1, false).unwrap(),
            DynamicValue::U16(4)
        );
        assert_eq!(
            server.list_get_item(list_id, 99).unwrap_err().kind,
            ErrorKind::OutOfRange
        );

        server
            .map_set(
                map_id,
                &DynamicValue::Map(vec![
                    (DynamicValue::U8(1), DynamicValue::String("one".into())),
                    (DynamicValue::U8(2), DynamicValue::String("two".into())),
                ]),
                false,
            )
            .unwrap();
        assert_eq!(server.map_len(map_id).unwrap(), 2);
        let DynamicValue::Map(all_map_entries) = server.map_get(map_id).unwrap() else {
            panic!("map_get did not return a dynamic map")
        };
        assert_eq!(all_map_entries.len(), 2);
        server
            .map_set_item(
                map_id,
                &DynamicValue::U8(1),
                &DynamicValue::String("uno".into()),
                false,
            )
            .unwrap();
        assert_eq!(
            server.map_get_item(map_id, &DynamicValue::U8(1)).unwrap(),
            DynamicValue::String("uno".into())
        );
        assert_eq!(
            server
                .map_remove_item(map_id, &DynamicValue::U8(2), false)
                .unwrap(),
            DynamicValue::String("two".into())
        );
        assert_eq!(server.map_len(map_id).unwrap(), 1);
        assert_eq!(
            server
                .map_get_item(map_id, &DynamicValue::U8(9))
                .unwrap_err()
                .kind,
            ErrorKind::NotFound
        );

        let rgb_with_padding = [255, 0, 0, 0, 255, 0, 9, 9, 0, 0, 255, 255, 255, 255, 9, 9];
        let image = DynamicImageView {
            data: &rgb_with_padding,
            height: 2,
            width: 2,
            row_stride: 8,
            format: DynamicImageFormat::Rgb,
        };
        server.image_set(image_id, &image, false).unwrap();
        let (rgba, size) = server.image_get(image_id).unwrap();
        assert_eq!(size, [2, 2]);
        assert_eq!(server.image_size(image_id).unwrap(), [2, 2]);
        assert_eq!(rgba.len(), 16);
        assert_eq!(&rgba[..4], &[255, 0, 0, 255]);
        server
            .image_set_all(image_id, [2, 3], [1, 2, 3, 4], false)
            .unwrap();
        let gray = [9];
        let gray_view = DynamicImageView {
            data: &gray,
            height: 1,
            width: 1,
            row_stride: 0,
            format: DynamicImageFormat::Gray,
        };
        server
            .image_update(image_id, [1, 2], &gray_view, false, false)
            .unwrap();
        let (rgba, size) = server.image_get(image_id).unwrap();
        assert_eq!(size, [2, 3]);
        assert_eq!(&rgba[20..24], &[9, 9, 9, 255]);
        assert!(
            server
                .image_update(image_id, [usize::MAX, 0], &gray_view, false, false)
                .is_err()
        );

        server
            .image_multi_set(image_multi_id, 4, &image, false)
            .unwrap();
        assert_eq!(server.image_multi_size(image_multi_id, 4).unwrap(), [2, 2]);
        assert_eq!(server.image_multi_get(image_multi_id, 4).unwrap().1, [2, 2]);
        assert!(server.image_multi_contains(image_multi_id, 4).unwrap());
        server
            .image_multi_set_all(image_multi_id, 5, [2, 2], [4, 3, 2, 1], false)
            .unwrap();
        server
            .image_multi_update(image_multi_id, 5, [0, 1], &gray_view, false, false)
            .unwrap();
        assert_eq!(server.image_multi_len(image_multi_id).unwrap(), 2);
        assert_eq!(
            server.image_multi_indices(image_multi_id).unwrap(),
            vec![4, 5]
        );
        server
            .image_multi_remove_index(image_multi_id, 4, false)
            .unwrap();
        assert!(!server.image_multi_contains(image_multi_id, 4).unwrap());
        server.image_multi_reset(image_multi_id, false).unwrap();
        assert_eq!(server.image_multi_len(image_multi_id).unwrap(), 0);
        assert_eq!(
            server.image_multi_size(image_multi_id, 5).unwrap_err().kind,
            ErrorKind::NotFound
        );

        let encode_u16 = |values: &[u16]| {
            values
                .iter()
                .flat_map(|value| value.to_ne_bytes())
                .collect::<Vec<_>>()
        };
        let initial_u16 = encode_u16(&[1, 2]);
        server.data_set(data_id, &initial_u16, 2, false).unwrap();
        assert_eq!(server.data_get(data_id).unwrap(), initial_u16);
        server
            .data_add(data_id, &3_u16.to_ne_bytes(), 1, false)
            .unwrap();
        server
            .data_replace(data_id, &4_u16.to_ne_bytes(), 1, 1, false)
            .unwrap();
        assert_eq!(server.data_get(data_id).unwrap(), encode_u16(&[1, 4, 3]));
        server.data_remove(data_id, 0, 1, false).unwrap();
        assert_eq!(server.data_get(data_id).unwrap(), encode_u16(&[4, 3]));
        assert_eq!(
            server
                .data_remove(data_id, usize::MAX, 2, false)
                .unwrap_err()
                .kind,
            ErrorKind::OutOfRange
        );
        server.data_clear(data_id, false).unwrap();
        assert!(server.data_get(data_id).unwrap().is_empty());
        assert_eq!(
            server
                .data_set(data_id, &1_u16.to_ne_bytes(), 2, false)
                .unwrap_err()
                .kind,
            ErrorKind::InvalidArgument
        );
        server
            .data_take_set(data_take_id, &[1, 2], 2, false, false, true)
            .unwrap();
        let i32_bytes = |values: &[i32]| {
            values
                .iter()
                .flat_map(|value| value.to_ne_bytes())
                .collect::<Vec<_>>()
        };
        let first_i32 = i32_bytes(&[1, 2]);
        server
            .data_multi_set(data_multi_id, 8, &first_i32, 2, false)
            .unwrap();
        server
            .data_multi_add(data_multi_id, 8, &3_i32.to_ne_bytes(), 1, false)
            .unwrap();
        server
            .data_multi_replace(data_multi_id, 8, &4_i32.to_ne_bytes(), 1, 1, false)
            .unwrap();
        assert_eq!(
            server.data_multi_get(data_multi_id, 8).unwrap(),
            i32_bytes(&[1, 4, 3])
        );
        server
            .data_multi_remove(data_multi_id, 8, 0, 1, false)
            .unwrap();
        assert_eq!(
            server.data_multi_get(data_multi_id, 8).unwrap(),
            i32_bytes(&[4, 3])
        );
        server.data_multi_clear(data_multi_id, 8, false).unwrap();
        assert!(server.data_multi_get(data_multi_id, 8).unwrap().is_empty());
        server
            .data_multi_remove_index(data_multi_id, 8, false)
            .unwrap();
        assert_eq!(
            server.data_multi_get(data_multi_id, 8).unwrap_err().kind,
            ErrorKind::NotFound
        );
        server
            .data_multi_set(data_multi_id, 1, &1_i32.to_ne_bytes(), 1, false)
            .unwrap();
        server
            .data_multi_set(data_multi_id, 2, &2_i32.to_ne_bytes(), 1, false)
            .unwrap();
        server.data_multi_reset(data_multi_id, false).unwrap();
        assert_eq!(
            server.data_multi_get(data_multi_id, 1).unwrap_err().kind,
            ErrorKind::NotFound
        );
        server
            .data_multi_take_set(
                data_multi_take_id,
                9,
                &1_f32.to_ne_bytes(),
                1,
                false,
                false,
                true,
            )
            .unwrap();
        server
            .data_multi_take_remove_index(data_multi_take_id, 9, false)
            .unwrap();
        server
            .data_multi_take_set(
                data_multi_take_id,
                10,
                &2_f32.to_ne_bytes(),
                1,
                false,
                false,
                true,
            )
            .unwrap();
        server
            .data_multi_take_reset(data_multi_take_id, false)
            .unwrap();
    }

    #[test]
    fn dynamic_signal_wait_handles_claims_modes_and_multiple_waiters() {
        let server = Arc::new(DynamicServer::new(None));
        let first_id = server
            .add_value("root.first", &ObjectType::I32, &DynamicValue::I32(0), false)
            .unwrap();
        let second_id = server
            .add_value(
                "root.second",
                &ObjectType::I32,
                &DynamicValue::I32(0),
                false,
            )
            .unwrap();
        server.finalize().unwrap();
        server.signal_register(first_id, true, true).unwrap();
        server.signal_register(second_id, true, false).unwrap();

        server
            .value_set(first_id, &DynamicValue::I32(1), true, false)
            .unwrap();
        let first_event = server.signal_wait(Some(Duration::ZERO)).unwrap();
        assert_eq!(first_event.value(), &DynamicValue::I32(1));
        assert_eq!(first_event.previous(), Some(&DynamicValue::I32(0)));

        server
            .value_set(first_id, &DynamicValue::I32(2), true, false)
            .unwrap();
        match server.signal_wait(Some(Duration::ZERO)) {
            Err(error) => assert_eq!(error.kind, ErrorKind::Timeout),
            Ok(_) => panic!("same-id event was delivered before its claim was released"),
        }
        drop(first_event);
        let second_event = server.signal_wait(Some(Duration::ZERO)).unwrap();
        assert_eq!(second_event.value(), &DynamicValue::I32(2));
        assert_eq!(second_event.previous(), Some(&DynamicValue::I32(1)));
        drop(second_event);

        server.signal_set_to_single(first_id).unwrap();
        server
            .value_set(first_id, &DynamicValue::I32(3), true, false)
            .unwrap();
        server
            .value_set(first_id, &DynamicValue::I32(4), true, false)
            .unwrap();
        let coalesced = server.signal_wait(Some(Duration::ZERO)).unwrap();
        assert_eq!(coalesced.value(), &DynamicValue::I32(4));
        assert_eq!(coalesced.previous(), Some(&DynamicValue::I32(2)));
        drop(coalesced);

        server.signal_set_to_queue(first_id).unwrap();
        server
            .value_set(first_id, &DynamicValue::I32(5), true, false)
            .unwrap();
        server
            .value_set(first_id, &DynamicValue::I32(6), true, false)
            .unwrap();
        let queued_first = server.signal_wait(Some(Duration::ZERO)).unwrap();
        assert_eq!(queued_first.value(), &DynamicValue::I32(5));
        drop(queued_first);
        let queued_second = server.signal_wait(Some(Duration::ZERO)).unwrap();
        assert_eq!(queued_second.value(), &DynamicValue::I32(6));
        drop(queued_second);

        let barrier = Arc::new(Barrier::new(3));
        let (sender, receiver) = mpsc::channel();
        let mut waiters = Vec::new();
        for _ in 0..2 {
            let server = server.clone();
            let barrier = barrier.clone();
            let sender = sender.clone();
            waiters.push(thread::spawn(move || {
                barrier.wait();
                let event = server.signal_wait(Some(Duration::from_secs(1))).unwrap();
                sender.send(event.id()).unwrap();
            }));
        }
        drop(sender);
        barrier.wait();
        server
            .value_set(first_id, &DynamicValue::I32(7), true, false)
            .unwrap();
        server
            .value_set(second_id, &DynamicValue::I32(8), true, false)
            .unwrap();

        let mut received = receiver.into_iter().collect::<Vec<_>>();
        for waiter in waiters {
            waiter.join().unwrap();
        }
        received.sort_unstable();
        let mut expected = vec![first_id, second_id];
        expected.sort_unstable();
        assert_eq!(received, expected);
    }
}
