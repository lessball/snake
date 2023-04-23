use bevy::asset::{AssetLoader, Error, LoadContext, LoadedAsset};
use bevy::utils::BoxedFuture;
use parry3d::math::Point;
use parry3d::shape::TriMesh;

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
            let ground = load_obj(data)?;
            load_context.set_default_asset(LoadedAsset::new(ground));
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

pub fn load_obj(data: &str) -> Result<GroundMesh, Error> {
    let mut v: Vec<Point<f32>> = Vec::new();
    let mut ind = Vec::new();
    for line in data.lines() {
        let mut t = line.split(" ");
        match t.next() {
            Some("v") => {
                let a: [f32; 3] = parse_array(t)?;
                v.push(Point::new(a[0], a[1], a[2]));
            }
            Some("f") => {
                let mut fv = [0; 3];
                for i in 0..3 {
                    let fsrc = t.next().ok_or_else(|| Error::msg("load obj error"))?;
                    let f: [u32; 1] = parse_array(fsrc.split("/"))?;
                    fv[i] = f[0] - 1;
                }
                ind.push(fv)
            }
            _ => {}
        }
    }
    let ground_mesh = GroundMesh::new(TriMesh::new(v, ind));
    Ok(ground_mesh)
}
