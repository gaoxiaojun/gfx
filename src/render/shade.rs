// Copyright 2014 The Gfx-rs Developers.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Shader parameter handling.

use std::cell::Cell;
use device::{back, shade};
use device::shade::UniformValue;
use device::{Resources, RawBufferHandle, TextureHandle, SamplerHandle};

pub use device::shade::{Stage, CreateShaderError};

/// Helper trait to transform base types into their corresponding uniforms
pub trait ToUniform {
    /// Create a `UniformValue` representing this value.
    fn to_uniform(&self) -> shade::UniformValue;
}

macro_rules! impl_ToUniform(
    ($srcty:ty, $dstty:expr) => (
        impl ToUniform for $srcty {
            fn to_uniform(&self) -> shade::UniformValue {
                $dstty(*self)
            }
        }
    );
);

impl_ToUniform!(i32, UniformValue::I32);
impl_ToUniform!(f32, UniformValue::F32);

impl_ToUniform!([i32; 2], UniformValue::I32Vector2);
impl_ToUniform!([i32; 3], UniformValue::I32Vector3);
impl_ToUniform!([i32; 4], UniformValue::I32Vector4);

impl_ToUniform!([f32; 2], UniformValue::F32Vector2);
impl_ToUniform!([f32; 3], UniformValue::F32Vector3);
impl_ToUniform!([f32; 4], UniformValue::F32Vector4);

impl_ToUniform!([[f32; 2]; 2], UniformValue::F32Matrix2);
impl_ToUniform!([[f32; 3]; 3], UniformValue::F32Matrix3);
impl_ToUniform!([[f32; 4]; 4], UniformValue::F32Matrix4);

/// Variable index of a uniform.
pub type VarUniform = u16;

/// Variable index of a uniform block.
pub type VarBlock = u8;

/// Variable index of a texture.
pub type VarTexture = u8;

/// A texture parameter: consists of a texture handle with an optional sampler.
pub type TextureParam = (TextureHandle<back::GlResources>, Option<SamplerHandle<back::GlResources>>);

/// A borrowed mutable storage for shader parameter values.
// Not sure if it's the best data structure to represent it.
pub struct ParamValues<'a> {
    /// uniform values to be provided
    pub uniforms: &'a mut Vec<UniformValue>,
    /// uniform buffers to be provided
    pub blocks  : &'a mut Vec<RawBufferHandle<back::GlResources>>,
    /// textures to be provided
    pub textures: &'a mut Vec<TextureParam>,
}

/// An error type on either the parameter storage or the program side
#[derive(Clone, PartialEq, Debug)]
pub enum ParameterError {
    /// Internal error
    ParameterGeneralMismatch,
    /// Shader requested a uniform that the parameters do not have
    MissingUniform(String),
    /// Shader requested a block that the parameters do not have
    MissingBlock(String),
    /// Shader requested a texture that the parameters do not have
    MissingTexture(String),
}

/// Abstracts the shader parameter structure, generated by the `shader_param` attribute
pub trait ShaderParam {
    /// A helper structure to contain variable indices inside the shader
    type Link;
    /// Create a new link to be used with a given program
    fn create_link(Option<&Self>, &shade::ProgramInfo) -> Result<Self::Link, ParameterError>;
    /// Get all the contained parameter values, using a given link
    fn fill_params(&self, &Self::Link, ParamValues);
}

impl ShaderParam for () {
    type Link = ();

    fn create_link(_: Option<&()>, info: &shade::ProgramInfo) -> Result<(), ParameterError> {
        match info.uniforms[..].first() {
            Some(u) => return Err(ParameterError::MissingUniform(u.name.clone())),
            None => (),
        }
        match info.blocks[..].first() {
            Some(b) => return Err(ParameterError::MissingBlock(b.name.clone())),
            None => (),
        }
        match info.textures[..].first() {
            Some(t) => return Err(ParameterError::MissingTexture(t.name.clone())),
            None => (),
        }
        Ok(())
    }

    fn fill_params(&self, _: &(), _: ParamValues) {
        //empty
    }
}

/// A named cell containing arbitrary value
pub struct NamedCell<T> {
    /// Name
    pub name: String,
    /// Value
    pub value: Cell<T>,
}

/// A dictionary of parameters, meant to be shared between different programs
pub struct ParamDictionary<R: Resources> {
    /// Uniform dictionary
    pub uniforms: Vec<NamedCell<shade::UniformValue>>,
    /// Block dictionary
    pub blocks: Vec<NamedCell<RawBufferHandle<R>>>,
    /// Texture dictionary
    pub textures: Vec<NamedCell<TextureParam>>,
}

/// Redirects program input to the relevant ParamDictionary cell
pub struct ParamDictionaryLink {
    uniforms: Vec<usize>,
    blocks: Vec<usize>,
    textures: Vec<usize>,
}

impl ShaderParam for ParamDictionary<back::GlResources> {
    type Link = ParamDictionaryLink;

    fn create_link(this: Option<&ParamDictionary<back::GlResources>>, info: &shade::ProgramInfo)
                   -> Result<ParamDictionaryLink, ParameterError> {
        let this = match this {
            Some(d) => d,
            None => return Err(ParameterError::ParameterGeneralMismatch),
        };
        //TODO: proper error checks
        Ok(ParamDictionaryLink {
            uniforms: info.uniforms.iter().map(|var|
                this.uniforms.iter().position(|c| c.name == var.name).unwrap()
            ).collect(),
            blocks: info.blocks.iter().map(|var|
                this.blocks  .iter().position(|c| c.name == var.name).unwrap()
            ).collect(),
            textures: info.textures.iter().map(|var|
                this.textures.iter().position(|c| c.name == var.name).unwrap()
            ).collect(),
        })
    }

    fn fill_params(&self, link: &ParamDictionaryLink, params: ParamValues) {
        for &id in link.uniforms.iter() {
            params.uniforms.push(self.uniforms[id].value.get());
        }
        for &id in link.blocks.iter() {
            params.blocks.push(self.blocks[id].value.get());
        }
        for &id in link.textures.iter() {
            params.textures.push(self.textures[id].value.get());
        }
    }
}
