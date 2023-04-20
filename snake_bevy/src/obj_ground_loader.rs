use bevy::asset::{AssetLoader, Error, LoadContext, LoadedAsset};
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::utils::BoxedFuture;
use parry3d::math::Point;
use parry3d::shape::TriMesh;

use std::collections::HashMap;
use std::str::FromStr;

use super::ground_mesh::GroundMesh;

#[derive(Default)]
pub struct ObjGroundLoader;

impl AssetLoader for ObjGroundLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<(), Error>> {
        Box::pin(async move {
            let data = std::str::from_utf8(bytes)?;
            let (mesh, ground) = load_obj(data)?;
            load_context.set_default_asset(LoadedAsset::new(mesh));
            load_context.set_labeled_asset("ground", LoadedAsset::new(ground));
            Ok(())
        })
    }
    fn extensions(&self) -> &[&str] {
        &["obj"]
    }
}

fn parse_array<'a, T, const N: usize, I>(mut iter: I) -> Result<[T; N], Error>
where
    T: FromStr + Default + Copy,
    <T as FromStr>::Err: Sync + Send + std::error::Error + 'static,
    I: Iterator<Item = &'a str>,
{
    let mut a = [T::default(); N];
    for i in 0..N {
        a[i] = iter.next().ok_or_else(|| Error::msg("iter end"))?.parse()?
    }
    Ok(a)
}

fn parse_3d<'a, I>(iter: I) -> Result<[f32; 3], Error>
where
    I: Iterator<Item = &'a str>,
{
    let a: [f32; 3] = parse_array(iter)?;
    Ok([-a[2], a[1], a[0]])
}

pub fn load_obj(data: &str) -> Result<(Mesh, GroundMesh), Error> {
    let mut v: Vec<[f32; 3]> = Vec::new();
    let mut vn: Vec<[f32; 3]> = Vec::new();
    let mut vt: Vec<[f32; 2]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    let mut fmap = HashMap::new();
    let mut fdata = Vec::new();
    let mut vind = Vec::new();
    for line in data.lines() {
        let mut t = line.split(" ");
        match t.next() {
            Some("v") => {
                v.push(parse_3d(t)?);
            }
            Some("vn") => {
                vn.push(parse_3d(t)?);
            }
            Some("vt") => {
                vt.push(parse_array(t)?);
            }
            Some("f") => {
                let mut fv = [0; 3];
                for i in 0..3 {
                    let fsrc = t.next().ok_or_else(|| Error::msg("load obj error"))?;
                    let mut f: [u32; 3] = parse_array(fsrc.split("/"))?;
                    for j in f.iter_mut() {
                        *j -= 1;
                    }
                    if let Some(fi) = fmap.get(&f) {
                        indices.push(*fi);
                    } else {
                        let index = fdata.len() as u32;
                        fmap.insert(f, index);
                        fdata.push(f);
                        indices.push(index);
                    }
                    fv[i] = f[0];
                }
                vind.push(fv)
            }
            _ => {}
        }
    }
    let mut pos = Vec::with_capacity(fdata.len());
    let mut nor = Vec::with_capacity(fdata.len());
    let mut uv = Vec::with_capacity(fdata.len());
    for i in fdata.iter() {
        pos.push(v[i[0] as usize]);
        nor.push(vn[i[2] as usize]);
        uv.push(vt[i[1] as usize]);
    }
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
    mesh.set_indices(Some(Indices::U32(indices)));
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, pos);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, nor);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);

    let v1 = v.iter().map(|tv| Point::new(tv[0], tv[1], tv[2])).collect();
    let ground_mesh = GroundMesh::new(TriMesh::new(v1, vind));

    Ok((mesh, ground_mesh))
}
