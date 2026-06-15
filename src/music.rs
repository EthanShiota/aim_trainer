use bevy::prelude::*;
use cpal::{
    FromSample, Sample, SizedSample, Stream,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use std::{sync::Arc, thread::sleep, time::Duration};

use symphonia::core::{audio::conv::IntoSample, dsp};
