use std::{collections::HashMap, error::Error, path::PathBuf};

use nom::{
    IResult, Parser,
    bytes::{
        complete::{is_not, tag},
        take_till,
    },
    character::{
        char,
        complete::{
            alphanumeric1, digit1, line_ending, multispace0, multispace1, not_line_ending, one_of,
        },
    },
    combinator::{map, opt, recognize},
    error::context,
    multi::{many0, many1, separated_list1},
    number::complete::recognize_float,
    sequence::{delimited, pair, preceded, separated_pair, terminated},
};

#[derive(Clone)]
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

#[derive(Clone, Copy)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}
#[derive(Clone)]
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
    let (_remainder, (slider_type, points)) = (
        preceded(multispace0, one_of("BCLP")),
        preceded(char('|'), separated_list1(char('|'), point)),
    )
        .parse_complete(s[0])
        .unwrap();
    let rest = &s[1..];
    let slides = rest[0].parse().unwrap();
    let length = rest[1].parse().unwrap();

    return Ok(SliderParams {
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
    });
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
#[derive(Clone)]
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

#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
enum OsuHeader {
    General,
    Editor,
    Metadata,
    Difficulty,
    Events,
    TimingPoints,
    Colours,
    HitObjects,
}

enum OsuValue<'a> {
    KV(HashMap<&'a str, &'a str>),
    LIST(Vec<Vec<&'a str>>),
}

fn section_header(input: &str) -> IResult<&str, OsuHeader> {
    map(
        terminated(
            delimited(char('['), recognize(alphanumeric1), char(']')),
            line_ending,
        ),
        |res| match res {
            "General" => OsuHeader::General,
            "Editor" => OsuHeader::Editor,
            "Metadata" => OsuHeader::Metadata,
            "Difficulty" => OsuHeader::Difficulty,
            "Events" => OsuHeader::Events,
            "TimingPoints" => OsuHeader::TimingPoints,
            "Colours" => OsuHeader::Colours,
            "HitObjects" => OsuHeader::HitObjects,
            _ => unimplemented!(),
        },
    )
    .parse_complete(input)
}

fn parser<'a>(
    input: &'a str,
) -> IResult<&'a str, (&'a str, Vec<(OsuHeader, OsuValue<'a>)>), nom::error::Error<&'a str>> {
    let metadata = terminated(not_line_ending, multispace1);
    let comment = |input: &'a str| {
        many0(terminated(
            preceded(tag("//"), not_line_ending::<&str, nom::error::Error<&str>>),
            multispace0,
        ))
        .parse(input)
    };
    let key_value = |input: &'a str| {
        terminated(
            separated_pair(
                terminated(recognize(alphanumeric1), multispace0),
                char(':'),
                recognize(not_line_ending::<&'a str, _>).map(|res| res.trim()),
            ),
            opt(line_ending),
        )
        .parse(input)
    };

    let list_value = |input: &'a str| {
        delimited(
            multispace0,
            separated_list1(char(','), is_not(",\n\r")),
            multispace0,
        )
        .parse(input)
    };
    let body = take_till(|c| c == '[');

    let section = pair(section_header, body).map(|(header, body)| {
        let values = match header {
            OsuHeader::General
            | OsuHeader::Editor
            | OsuHeader::Metadata
            | OsuHeader::Difficulty
            | OsuHeader::Colours => many1(delimited(comment, key_value, comment))
                .parse(body)
                .map(|(_, value)| OsuValue::KV(value.into_iter().collect())),
            OsuHeader::Events | OsuHeader::TimingPoints | OsuHeader::HitObjects => {
                many1(delimited(comment, list_value, comment))
                    .parse(body)
                    .map(|(_, value)| OsuValue::LIST(value))
            }
        }
        .expect("section parser failed");
        (header, values)
    });

    pair(metadata, many1(context("section", section)))
        .parse_complete(input)
        .into()
}

impl BeatMapOsu {
    pub fn new(value: PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let s = std::fs::read_to_string(value.clone())?;

        let (_, parse_res) = parser(&s).map_err(|e| e.to_owned())?;
        let (_metadata, sections) = parse_res;
        let sections: HashMap<OsuHeader, OsuValue> = sections.into_iter().collect();
        let OsuValue::KV(general) = sections.get(&OsuHeader::General).unwrap() else {
            unreachable!()
        };
        let OsuValue::KV(metadata) = sections.get(&OsuHeader::Metadata).unwrap() else {
            unreachable!()
        };
        let OsuValue::LIST(hitobj) = sections.get(&OsuHeader::HitObjects).unwrap() else {
            unreachable!()
        };

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
            hit_objects: hitobj
                .into_iter()
                .map(|s| s.as_slice().try_into().unwrap())
                .collect(),
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

#[test]
fn nom_parser_test() {
    let input = include_str!(
        "../osu_beatmaps/163112 Kuba Oms - My Love.osz/Kuba Oms - My Love (W h i t e) [Normal].osu"
    );
    let (s, (metadata, res)) = parser(input).unwrap();

    println!("meta: {}", metadata);
    for (header, value) in res {
        println!("header: {:?}", header);
        match value {
            OsuValue::KV(value) => {
                for (k, v) in value {
                    println!("{} = {}", k, v);
                }
            }
            OsuValue::LIST(list) => {
                for item in list {
                    println!("{item:?}");
                }
            }
        }
    }
}
