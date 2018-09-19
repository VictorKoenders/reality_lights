use image::bmp::BMPDecoder;
use image::{DecodingResult, ImageDecoder};
use messages::{Animation, AnimationFrame};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Seek};
use Result;

pub struct AnimationHandler {
    pub animations: Vec<Animation>,
}

impl Default for AnimationHandler {
    fn default() -> AnimationHandler {
        AnimationHandler::new().expect("Could not load animations")
    }
}

macro_rules! unwrap_none {
    ($e:expr) => {
        match $e {
            Some(v) => v,
            None => bail!("Could not get value"),
        }
    };
}

impl AnimationHandler {
    pub fn new() -> Result<AnimationHandler> {
        let _ = fs::create_dir("animations");
        let mut handler = AnimationHandler {
            animations: Vec::new(),
        };
        for file in fs::read_dir("animations")? {
            let file = file?;
            if file.file_type()?.is_dir() {
                let name: String =
                    unwrap_none!(unwrap_none!(file.path().file_name()).to_str()).to_owned();
                let mut files: HashMap<String, File> = HashMap::new();
                for file in fs::read_dir(file.path())? {
                    let file = file?;
                    let name: String =
                        unwrap_none!(unwrap_none!(file.path().file_name()).to_str()).to_owned();
                    let file = File::open(file.path())?;
                    files.insert(name, file);
                }
                if let Err(e) = handler.load(name, files) {
                    println!("Could not load animation {:?}: {:?}", file.path(), e);
                }
            }
        }

        Ok(handler)
    }

    pub fn load<T: Read + Seek>(
        &mut self,
        name: String,
        map: HashMap<String, T>,
    ) -> Result<&Animation> {
        println!("Loading {:?}", name);
        let mut animation = Animation::default();
        animation.name = name;

        for (name, read) in map {
            let mut path = name.rsplitn(2, '.');

            let (extension, name) = (unwrap_none!(path.next()), unwrap_none!(path.next()));
            if extension == "bmp" {
                let index = name.parse::<usize>()?;
                while animation.frames.len() <= index {
                    animation.frames.push(Default::default());
                }
                animation.frames[index] = AnimationHandler::parse_bmp(read)?;
            } else if extension == "json" {
                AnimationHandler::load_config(read, &mut animation)?;
            }
        }
        self.animations.push(animation);
        Ok(unwrap_none!(self.animations.last()))
    }

    fn parse_bmp<T: Read + Seek>(read: T) -> Result<AnimationFrame> {
        let mut decoder = BMPDecoder::new(read);
        let image = decoder.read_image()?;
        match image {
            DecodingResult::U8(vec) => {
                let mut result = AnimationFrame::default();
                if vec.len() != 22 * 7 * 3 {
                    bail!(
                        "Unexpected byte length, expected {}, got {}",
                        22 * 7 * 3,
                        vec.len()
                    );
                }
                for (index, chunk) in vec.chunks(21).enumerate() {
                    let mut row: [(u8, u8, u8); 7] = [(0u8, 0u8, 0u8); 7];
                    for (index, slice) in chunk.chunks(3).enumerate() {
                        // Normalize to 0-150 instead of 0-255 because the torches can overheat
                        let r = (u16::from(slice[0]) * 150 / 255) as u8;
                        let g = (u16::from(slice[1]) * 150 / 255) as u8;
                        let b = (u16::from(slice[2]) * 150 / 255) as u8;
                        row[index] = (r, g, b);
                    }
                    result[index] = row;
                }
                Ok(result)
            }
            _ => bail!("Unsupported binary format:"),
        }
    }
    fn load_config<T>(_read: T, _animation: &mut Animation) -> Result<()> {
        bail!("Not implemented");
    }
}
