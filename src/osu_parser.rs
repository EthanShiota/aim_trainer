use std::{collections::HashMap, error::Error, fs::File, io::Read, path::PathBuf, str::FromStr};

#[derive(Clone)]
pub struct HitObject {
    pub x: usize,
    pub y: usize,
    // integer milliseconds
    pub time: usize,
    pub type_bitmask: u8,
    // hitSound: u8
    // objectParams:
    // hitSample:
}

impl FromStr for HitObject {
    type Err = Box<dyn Error>;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut iter = s.split(',');

        let x: usize = iter.next().unwrap().parse()?;
        let y: usize = iter.next().unwrap().parse()?;
        let time: usize = iter.next().unwrap().parse()?;
        let type_bitmask: u8 = iter.next().unwrap().parse()?;
        Ok(HitObject {
            x,
            y,
            time,
            type_bitmask,
        })
    }
}

#[derive(Clone)]
pub struct General {
    pub audio_filename: String,
    pub audio_lead_in: usize,
}

#[derive(Clone)]
pub struct Metadata {
    pub title: String,
    pub version: String,
}
#[derive(Clone)]
pub struct BeatMapOsu {
    pub beat_map_path: PathBuf,
    pub general: General,
    pub metadata: Metadata,
    pub hit_objects: Vec<HitObject>,
}

impl BeatMapOsu {
    pub fn new(mut value: PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let s = std::fs::read_to_string(value.clone())?;

        let lines_iter = s.lines();

        let sections = lines_iter.collect::<Vec<_>>();
        let mut sections_iter = sections
            .split(|line| line.starts_with('[') && line.ends_with(']'))
            .skip(1);
        let general = sections_iter.next().ok_or("general missing")?;
        let metadata = sections_iter.nth(1).ok_or("metadata missing")?;
        let hitobj = sections_iter.nth(4).ok_or("HitObj missing")?;

        let general = general
            .iter()
            .filter_map(|s| s.split_once(':').map(|(k, v)| (k, v.trim())))
            .collect::<HashMap<&str, &str>>();
        let metadata = metadata
            .iter()
            .filter_map(|s| s.split_once(':').map(|(k, v)| (k, v.trim())))
            .collect::<HashMap<&str, &str>>();
        Ok(BeatMapOsu {
            beat_map_path: value,
            general: General {
                audio_filename: general["AudioFilename"].to_string(),
                audio_lead_in: general["AudioLeadIn"].parse().unwrap(),
            },
            metadata: Metadata {
                title: metadata["Title"].to_string(),
                version: metadata["Version"].to_string(),
            },
            hit_objects: hitobj.iter().map(|&s| s.parse().unwrap()).collect(),
        })
    }
}

#[test]
fn parser_test() {
    BeatMapOsu::new(
        "osu_beatmaps/163112 Kuba Oms - My Love.osz/Kuba Oms - My Love (W h i t e) [Normal].osu"
            .into(),
    );
}
