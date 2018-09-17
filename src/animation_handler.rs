use image::bmp::BMPDecoder;
use image::{DecodingResult, ImageDecoder};
use messages::{Animation, AnimationFrame};
use std::fs::{self, File};
use std::path::Path;
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
                if let Err(e) = handler.load(&file.path()) {
                    println!("Could not load animation {:?}: {:?}", file.path(), e);
                }
            }
        }

        Ok(handler)
    }

    pub fn load(&mut self, dir: &Path) -> Result<&Animation> {
        println!("Loading {:?}", unwrap_none!(dir.file_name()));
        let mut animation = Animation::default();
        animation.name = unwrap_none!(unwrap_none!(dir.file_name()).to_str()).to_string();

        for file in fs::read_dir(dir)? {
            let file = file?;

            let file_path = file.path();
            let path = unwrap_none!(file_path.file_name());
            let path = unwrap_none!(path.to_str());
            let mut path = path.rsplitn(2, '.');

            let (extension, name) = (unwrap_none!(path.next()), unwrap_none!(path.next()));
            if extension == "bmp" {
                let index = name.parse::<usize>()?;
                while animation.frames.len() <= index {
                    animation.frames.push(Default::default());
                }
                animation.frames[index] = AnimationHandler::parse_bmp(&file_path)?;
            } else if extension == "json" {
                AnimationHandler::load_config(&file_path, &mut animation)?;
            }
        }
        self.animations.push(animation);
        Ok(self.animations.last().unwrap())
    }

    fn parse_bmp(path: &Path) -> Result<AnimationFrame> {
        let file = File::open(path)?;
        let mut decoder = BMPDecoder::new(file);
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
                        row[index] = (slice[0], slice[1], slice[2]);
                    }
                    result[index] = row;
                }
                Ok(result)
            }
            _ => bail!("Unsupported binary format:"),
        }
    }
    fn load_config(_path: &Path, _animation: &mut Animation) -> Result<()> {
        bail!("Not implemented");
    }
}
