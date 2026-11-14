use std::{any::Any, collections::HashMap, error::Error, path::PathBuf};
use tracing::{debug, debug_span};

use nom::{
    IResult, Parser,
    bytes::{
        complete::{is_not, tag},
        take_until,
    },
    character::{
        char,
        complete::{alphanumeric1, line_ending, multispace0, multispace1, not_line_ending, one_of},
    },
    combinator::{eof, map, opt, recognize, rest},
    error::context,
    multi::{many0, many1, separated_list1},
    number::complete::recognize_float,
    sequence::{delimited, pair, preceded, separated_pair, terminated},
};

type KV<'a> = HashMap<&'a str, &'a str>;
type List<'a> = Vec<Vec<&'a str>>;

#[derive(Clone, Debug)]
pub struct HitObject {
    pub position: Point,
    // integer milliseconds
    pub time: usize,
    pub type_bitmask: u8,
    pub hit_sound: u8,
    // Extra stuff
    // TODO: Should be enum
    pub object_params: Option<SliderParams>,
    // Last item, is optional
    pub hit_sample: Option<Vec<u8>>,
}

#[derive(Clone, Copy, Debug)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}
#[derive(Clone, Debug)]
pub struct SliderParams {
    pub curve_type: CurveType,
    pub curve_points: Vec<Point>,
    pub slides: usize,
    pub length: f32,
    // INFO: unimplemented
    // edge_sounds:
    // edge_sets:
}

fn parse_slider_params(s: &[&str]) -> Result<SliderParams, Box<dyn Error>> {
    let point = separated_pair(
        delimited(
            multispace0::<_, nom::error::Error<&str>>,
            recognize_float,
            multispace0,
        ),
        char(':'),
        preceded(multispace0, recognize_float),
    );
    debug!(?s);
    let (_remainder, (slider_type, points)) = (
        preceded(multispace0, one_of("BCLP")),
        preceded(char('|'), separated_list1(char('|'), point)),
    )
        .parse_complete(s[0])
        .unwrap();
    let rest = &s[1..];
    let slides = rest[0].parse().unwrap();
    let length = rest[1].parse().unwrap();

    Ok(SliderParams {
        curve_type: slider_type.try_into().unwrap(),
        curve_points: points
            .iter()
            .map(|(x, y)| Point {
                x: x.trim().parse().unwrap(),
                y: y.trim().parse().unwrap(),
            })
            .collect(),
        slides,
        length,
    })
}

impl TryFrom<char> for CurveType {
    type Error = ();
    fn try_from(value: char) -> Result<Self, ()> {
        Ok(match value {
            'B' => CurveType::Bezier,
            'C' => CurveType::CentripetalCatmullRom,
            'L' => CurveType::Linear,
            'P' => CurveType::PerfectCircle,
            _ => return Err(()),
        })
    }
}
#[derive(Clone, Debug, PartialEq)]
pub enum CurveType {
    Bezier,
    CentripetalCatmullRom,
    Linear,
    PerfectCircle,
}

impl TryFrom<&[&str]> for HitObject {
    type Error = Box<dyn Error>;
    fn try_from(s: &[&str]) -> Result<Self, Self::Error> {
        let mut iter = s.iter();

        let x: i32 = iter.next().unwrap().parse()?;
        let y: i32 = iter.next().unwrap().parse()?;
        let position = Point { x, y };
        let time: usize = iter.next().unwrap().parse()?;
        let type_bitmask: u8 = iter.next().unwrap().parse()?;
        let hit_sound: u8 = iter.next().unwrap().parse()?;

        let object_params = if (type_bitmask >> 1) & 1u8 == 1 {
            // slider
            // INFO:
            // 0  Marks the object as a hit circle
            // 1  Marks the object as a slider
            // 2  Marks the start of a new combo
            // 3  Marks the object as a spinner
            // 4, 5, 6 A 3-bit integer specifying how many combo colours to skip, a practice referred to as "colour hax". Only relevant if the object starts a new combo.
            // 7  Marks the object as an osu!mania hold note.
            let slider_params: SliderParams =
                parse_slider_params(&iter.copied().collect::<Vec<&str>>()).unwrap();
            Some(slider_params)
        } else {
            // hit circle
            None
        };
        Ok(HitObject {
            position,
            time,
            type_bitmask,
            hit_sound,
            object_params,
            hit_sample: None,
        })
    }
}

#[derive(Clone)]
pub struct General {
    pub audio_filename: String,
    pub audio_lead_in: usize,
}

impl<'a> TryFrom<&KV<'a>> for General {
    type Error = String;
    fn try_from(value: &KV) -> Result<Self, Self::Error> {
        Ok(Self {
            audio_filename: value
                .get("AudioFilename")
                .ok_or("Missing AudioFilename")?
                .to_string(),
            audio_lead_in: value
                .get("AudioLeadIn")
                .ok_or("Missing AudioLeadIn")?
                .parse()
                .unwrap(),
        })
    }
}

// #[derive(Clone)]
// pub struct Events {
//
// }
//
// pub enum Event {
//     Background()
// }

#[derive(Clone)]
pub struct Metadata {
    pub title: String,
    pub title_unicode: String,
    pub version: String,
    pub artist: String,
    pub artist_unicode: String,
    pub creator: String,
    pub source: String,
    pub tags: Vec<String>,
    pub beatmap_id: Option<i32>,
    pub beatmap_set_id: Option<i32>,
}

impl<'a> TryFrom<&KV<'a>> for Metadata {
    type Error = String;
    fn try_from(value: &KV<'a>) -> Result<Self, Self::Error> {
        Ok(Self {
            title: value.get("Title").ok_or("Missing Title")?.to_string(),
            version: value.get("Version").ok_or("Missing Version")?.to_string(),
            title_unicode: value
                .get("TitleUnicode")
                .or_else(|| value.get("Title"))
                .unwrap()
                .to_string(),
            artist: value.get("Artist").ok_or("Missing Artist")?.to_string(),
            artist_unicode: value
                .get("ArtistUnicode")
                .or_else(|| value.get("Artist"))
                .unwrap()
                .to_string(),
            creator: value.get("Creator").ok_or("Missing Creator")?.to_string(),
            source: value.get("Source").ok_or("Missing Source")?.to_string(),
            // Space seperated
            tags: value
                .get("Tags")
                .ok_or("Missing Tags")?
                .split(' ')
                .map(str::to_string)
                .collect(),
            beatmap_id: value.get("BeatmapID").and_then(|&s| s.parse().ok()),
            beatmap_set_id: value.get("BeatmapSetID").and_then(|&s| s.parse().ok()),
        })
    }
}

#[derive(Clone)]
pub struct Difficulty {
    pub hp_drain_rate: f32,
    pub circle_size: f32,
    pub overall_difficulty: f32,
    pub approach_rate: f32,
    pub slider_multiplier: f32,
    pub slider_tick_rate: f32,
}
impl<'a> TryFrom<&KV<'a>> for Difficulty {
    type Error = Box<dyn Error>;
    fn try_from(difficulty: &KV) -> Result<Self, Self::Error> {
        Ok(Self {
            hp_drain_rate: difficulty
                .get("HPDrainRate")
                .ok_or("Missing HPDrainRate")?
                .parse()?,
            circle_size: difficulty
                .get("CircleSize")
                .ok_or("Missing CircleSize")?
                .parse()?,
            overall_difficulty: difficulty
                .get("OverallDifficulty")
                .ok_or("Missing OverallDifficulty")?
                .parse()?,
            approach_rate: difficulty
                .get("ApproachRate")
                .ok_or("Missing ApproachRate")?
                .parse()?,
            slider_multiplier: difficulty
                .get("SliderMultiplier")
                .ok_or("Missing SliderMultiplier")?
                .parse()?,
            slider_tick_rate: difficulty
                .get("SliderTickRate")
                .ok_or("Missing SliderTickRate")?
                .parse()?,
        })
    }
}

#[derive(Clone, Debug)]
pub struct TimingPoint {
    // INFO: time in milliseconds for some reason this is a decimal and not a integer
    pub time: f32,
    pub beat_length: f32,
    pub meter: u32,
    pub sample_set: u32,
    pub sample_index: u32,
    pub volume: u32,
    pub uninherited: bool,
    pub effects: u32,
}

impl TryFrom<&[&str]> for TimingPoint {
    type Error = Box<dyn Error>;
    fn try_from(value: &[&str]) -> Result<Self, Self::Error> {
        if let [
            time,
            beat_length,
            meter,
            sample_set,
            sample_index,
            volume,
            uninherited,
            effects,
        ] = *value
        {
            Ok(Self {
                time: time.parse()?,
                beat_length: beat_length.parse()?,
                meter: meter.parse()?,
                sample_set: sample_set.parse()?,
                sample_index: sample_index.parse()?,
                volume: volume.parse()?,
                uninherited: uninherited.parse::<u32>().map(|i| i == 1)?,
                effects: effects.parse()?,
            })
        } else {
            // Parse Error
            return Err(format!("Could not parse timing point {:?}", value).into());
        }
    }
}
#[derive(Clone)]
pub struct BeatMapOsu {
    // pub events: Events,
    pub beat_map_path: PathBuf,
    pub general: General,
    pub metadata: Metadata,
    pub difficulty: Difficulty,
    pub timing_points: Vec<TimingPoint>,
    pub hit_objects: Vec<HitObject>,
}

#[derive(Debug, Eq, PartialEq)]
enum OsuSection<'a> {
    General(KV<'a>),
    Editor(KV<'a>),
    Metadata(KV<'a>),
    Difficulty(KV<'a>),
    Events(List<'a>),
    TimingPoints(List<'a>),
    Colours(KV<'a>),
    HitObjects(List<'a>),
}

impl<'a> Ord for OsuSection<'a> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let lhs = match self {
            OsuSection::General(_) => 1,
            OsuSection::Editor(_) => 2,
            OsuSection::Metadata(_) => 3,
            OsuSection::Difficulty(_) => 4,
            OsuSection::Events(_) => 5,
            OsuSection::TimingPoints(_) => 6,
            OsuSection::Colours(_) => 7,
            OsuSection::HitObjects(_) => 8,
        };

        let rhs = match other {
            OsuSection::General(_) => 1,
            OsuSection::Editor(_) => 2,
            OsuSection::Metadata(_) => 3,
            OsuSection::Difficulty(_) => 4,
            OsuSection::Events(_) => 5,
            OsuSection::TimingPoints(_) => 6,
            OsuSection::Colours(_) => 7,
            OsuSection::HitObjects(_) => 8,
        };
        lhs.cmp(&rhs)
    }
}

impl<'a> PartialOrd for OsuSection<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

fn section<'a>(input: &'a str) -> IResult<&'a str, OsuSection<'a>> {
    let header = terminated(
        delimited(char('['), recognize(alphanumeric1), char(']')),
        line_ending,
    );

    let body = take_until("\n[").or(rest).or(eof);

    let comment = |input: &'a str| {
        many0(terminated(
            preceded(
                tag("//"),
                not_line_ending::<&'a str, nom::error::Error<&'a str>>,
            ),
            multispace0,
        ))
        .parse_complete(input)
    };

    let key_value = |input: &'a str| {
        context(
            "key_value",
            terminated(
                separated_pair(
                    terminated(recognize(alphanumeric1), multispace0),
                    char(':'),
                    recognize(not_line_ending::<&'a str, _>).map(|res| res.trim()),
                ),
                opt(line_ending),
            ),
        )
        .parse_complete(input)
    };

    let list_value = |input: &'a str| {
        context(
            "list_value",
            delimited(
                multispace0,
                separated_list1(char(','), is_not(",\n\r")),
                multispace0,
            ),
        )
        .parse_complete(input)
    };

    let kv = |body: &'a str| {
        many0(delimited(comment, key_value, comment))
            .parse_complete(body)
            .inspect(|(l, value)| debug!(l, ?value))
            .map(|(_, value)| value.into_iter().collect::<KV>())
    };

    let list = |body| {
        many0(delimited(comment, list_value, comment))
            .parse_complete(body)
            .inspect(|(l, value)| debug!(l, ?value))
            .map(|(_, value)| value)
    };

    map(
        pair(header, terminated(body, multispace0)),
        |(header, body)| match header {
            "General" => OsuSection::General(kv(body).unwrap()),
            "Editor" => OsuSection::Editor(kv(body).unwrap()),
            "Metadata" => OsuSection::Metadata(kv(body).unwrap()),
            "Difficulty" => OsuSection::Difficulty(kv(body).unwrap()),
            "Events" => OsuSection::Events(list(body).unwrap()),
            "TimingPoints" => OsuSection::TimingPoints(list(body).unwrap()),
            "Colours" => OsuSection::Colours(kv(body).unwrap()),
            "HitObjects" => OsuSection::HitObjects(list(body).unwrap()),
            _ => unimplemented!(),
        },
    )
    .parse_complete(input)
}

fn parser<'a>(
    input: &'a str,
) -> IResult<&'a str, (&'a str, Vec<OsuSection<'a>>), nom::error::Error<&'a str>> {
    let metadata = terminated(not_line_ending, multispace1);

    pair(metadata, many1(context("section", section))).parse_complete(input)
}

impl BeatMapOsu {
    pub fn new(value: PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let s = std::fs::read_to_string(value.clone())?;

        let file_name = value.to_string_lossy();
        let span = debug_span!("parser");
        let _guard = span.enter();
        debug!(file_name = ?file_name);
        let (extra, parse_res) = parser(&s).map_err(|e| e.to_owned())?;
        let (_metadata, mut sections) = parse_res;
        debug!("extra {:#?}", extra);
        debug!("sections {:#?}", sections);

        sections.sort();
        let [
            OsuSection::General(general),
            OsuSection::Editor(_editor),
            OsuSection::Metadata(metadata),
            OsuSection::Difficulty(difficulty),
            OsuSection::Events(_events),
            OsuSection::TimingPoints(timing_points),
            OsuSection::Colours(_colours),
            OsuSection::HitObjects(hit_objects),
        ] = sections.as_slice()
        else {
            return Err("section missing".into());
        };

        let timing_points: Vec<TimingPoint> = timing_points
            .into_iter()
            .map(|e| TimingPoint::try_from(e.as_slice()).unwrap())
            .collect();

        let hit_objects: Vec<HitObject> = hit_objects
            .iter()
            .map(|s| s.as_slice().try_into().unwrap())
            .collect();

        // INFO: this should be sorted
        // TODO: Don't do this
        assert!(timing_points.is_sorted_by_key(|k| k.time));
        assert!(hit_objects.is_sorted_by_key(|k| k.time));

        Ok(BeatMapOsu {
            beat_map_path: value,
            general: General::try_from(general).unwrap(),
            metadata: Metadata::try_from(metadata).unwrap(),
            difficulty: Difficulty::try_from(difficulty).unwrap(),
            timing_points,
            hit_objects,
        })
    }

    pub fn difficulty_multiplier(&self) -> f32 {
        // TODO: Star rating
        const PERFORMANCE_BASE_MULTIPLIER: f32 = 1.12; // This is being adjusted to keep the final pp value scaled around what it used to be when changing things.
        const PERFORMANCE_NORM_EXPONENT: f32 = 1.1;

        // Difficulty multiplier = Round((HP Drain + Circle Size + Overall Difficulty + Clamp(Hit object count / Drain time in seconds * 8, 0, 16)) / 38 * 5)
        let Difficulty {
            approach_rate,
            hp_drain_rate,
            circle_size,
            overall_difficulty,
            ..
        } = self.difficulty;
        // let hit_object_count = self.hit_objects.len();
        // TODO: Make sure this is correct
        ((hp_drain_rate + circle_size + overall_difficulty) // TODO: Drain per second (hit_object_count as f32 / (hp_drain_rate * 8.)).clamp(0., 16.)
         / 30.0
            * 5.0)
            .round()
    }

    // HACK: Search beatmap folder for a image and use that as the background
    pub fn background(&self) -> Option<PathBuf> {
        for file in self.beat_map_path.parent()?.read_dir().ok()?.flatten() {
            let file_path: PathBuf = file.path().iter().collect();
            if !file_path.starts_with("osu_beatmaps") {
                return None;
            }

            return match file.file_name().to_str() {
                Some(s) => match s.split('.').next_back() {
                    Some("jpg" | "png") => Some(std::env::current_dir().ok()?.join(file.path())),
                    _ => continue,
                },
                None => continue,
            };
        }
        None
    }
}

#[test]
fn parser_test() {
    env_logger::init();
    for file in std::fs::read_dir("test").unwrap().flatten() {
        BeatMapOsu::new(file.path()).unwrap();
    }
}

#[test]
fn nom_parser_test() {
    let input = include_str!("../test/Kuba Oms - My Love (W h i t e) [Insane].osu");
    let (_, (metadata, res)) = parser(input).unwrap();
}

#[test]
fn hit_object_test() {
    use crate::CurveType::Bezier;
    let input = include_str!(
        "../test/Windbell - Flow of Life (TSAR  Tu Zi ST Remix) (vivicat) [Extra].osu"
    );
    let (_, (_metadata, res)) = parser(input).unwrap();
}
