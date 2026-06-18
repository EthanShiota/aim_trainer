use std::{collections::HashMap, error::Error, fs::File, io::Read, str::FromStr};

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

pub struct General {
    pub audio_filename: String,
    pub audio_lead_in: usize,
}
pub struct BeatMapOsu {
    pub general: General,
    pub hit_objects: Vec<HitObject>,
}

impl BeatMapOsu {
    pub fn new(mut value: File) -> Result<Self, Box<dyn std::error::Error>> {
        let mut s = String::new();
        value.read_to_string(&mut s)?;

        let lines_iter = s.lines();

        let sections = lines_iter.collect::<Vec<_>>();
        let mut sections_iter = sections
            .split(|line| line.starts_with('[') && line.ends_with(']'))
            .skip(1);
        let general = sections_iter.next().unwrap();
        let hitobj = sections_iter.skip(6).next().unwrap();

        let dict_gen = general
            .iter()
            .filter_map(|s| s.split_once(':').map(|(k, v)| (k, v.trim())))
            .collect::<HashMap<&str, &str>>();
        Ok(BeatMapOsu {
            general: General {
                audio_filename: dict_gen["AudioFilename"].to_string(),
                audio_lead_in: dict_gen["AudioLeadIn"].parse().unwrap(),
            },
            hit_objects: hitobj.iter().map(|&s| s.parse().unwrap()).collect(),
        })
    }
}

#[test]
fn parser_test() {
    BeatMapOsu::new(File::open(
        "osu_beatmaps/163112 Kuba Oms - My Love.osz/Kuba Oms - My Love (W h i t e) [Normal].osu",
    ).unwrap());
}
